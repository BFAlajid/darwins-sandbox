//! Main simulation world for the STH (Soil-Transmitted Helminths) simulation.
//!
//! Ties together agents, environment, interventions, KAP dynamics, and
//! infection transmission into a coherent tick-based simulation loop.

use std::collections::HashMap;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use slotmap::{DenseSlotMap, Key};

use crate::agent::{Agent, AgentKey, AgentType, Location};
use crate::environment::{EnvironmentGrid, FacilityType, TerrainType};
use crate::infection::{self, SthSpecies};
use crate::intervention::{
    self, ActiveIntervention, Drug, EducationMethod, InterventionType,
};
use crate::kap::KapScores;
use crate::schedule::{self, TimeOfDay};
use crate::sth_config::SthConfig;
use crate::sth_event_tracker::{SthEventTracker, SthEventType};
use crate::sth_render_buffer::SthRenderBuffer;

// ---------------------------------------------------------------------------
// Barangay stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarangayStats {
    pub name: String,
    pub setting: String,
    pub prevalence_ascaris: f32,
    pub prevalence_trichuris: f32,
    pub prevalence_hookworm: f32,
    pub prevalence_any: f32,
    pub mean_epg: f32,
    pub mean_knowledge: f32,
    pub mean_attitude: f32,
    pub mean_practice: f32,
    pub latrine_coverage: f32,
    pub water_coverage: f32,
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

pub struct SthWorld {
    pub config: SthConfig,
    pub agents: DenseSlotMap<AgentKey, Agent>,
    pub environment: EnvironmentGrid,
    pub active_interventions: Vec<ActiveIntervention>,
    pub tick: u64,
    pub budget_spent: f32,
    pub budget_remaining: f32,

    // Cumulative stats
    pub total_mda_rounds: u32,
    pub total_children_treated: u32,
    pub total_latrines_built: u32,
    pub total_water_sources: u32,

    // Per-barangay stats
    pub barangay_stats: Vec<BarangayStats>,

    // Internal state
    rng: SmallRng,
    render_buffer: SthRenderBuffer,
    event_tracker: SthEventTracker,

    // Location mapping: Location -> (x, y) world position
    location_map: HashMap<LocationKey, (f32, f32)>,

    // Previous prevalence for detecting drops
    prev_prevalence: f32,
}

// Location enum is not Hash/Eq-able due to the enum variants,
// so we create a simple key representation.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
enum LocationKey {
    Home(u16),
    School(u8),
    Community,
    HealthCenter,
    WaterSource(u8),
}

impl From<Location> for LocationKey {
    fn from(loc: Location) -> Self {
        match loc {
            Location::Home(id) => LocationKey::Home(id),
            Location::School(id) => LocationKey::School(id),
            Location::Community => LocationKey::Community,
            Location::HealthCenter => LocationKey::HealthCenter,
            Location::WaterSource(id) => LocationKey::WaterSource(id),
        }
    }
}

/// Returns a scatter radius (in pixels) for a given location type.
/// Larger locations get wider scatter so agents spread out visually.
fn scatter_radius(location: Location) -> f32 {
    match location {
        Location::Home(_) => 15.0,
        Location::School(_) => 40.0,
        Location::Community => 60.0,
        Location::HealthCenter => 25.0,
        Location::WaterSource(_) => 20.0,
    }
}

/// Deterministic hash [0,1) for per-agent scatter angle.
/// Uses the agent key's raw bits to produce a stable value per agent per location change.
fn agent_scatter_hash(key: AgentKey, tick: u64) -> f32 {
    let raw = key.data().as_ffi();
    // Combine key with tick so scatter changes when agent re-enters same location
    let bits = raw.wrapping_mul(2654435761).wrapping_add(tick).wrapping_mul(0x517cc1b727220a95);
    ((bits >> 33) as u32 as f32) / (u32::MAX as f32)
}

/// Secondary deterministic hash [0,1) for per-agent scatter distance (stable per agent).
fn agent_scatter_hash_secondary(key: AgentKey) -> f32 {
    let raw = key.data().as_ffi();
    let bits = raw.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x6A09E667);
    ((bits >> 33) as u32 as f32) / (u32::MAX as f32)
}

impl SthWorld {
    /// Create a new world from a configuration.
    pub fn new(config: SthConfig) -> Result<Self, String> {
        config.validate()?;

        let mut rng = SmallRng::seed_from_u64(config.seed);
        let w = config.world_width;
        let h = config.world_height;

        // Create environment grid (10px cells)
        let cell_size = 10.0;
        let mut environment = EnvironmentGrid::new(w, h, cell_size);

        // Build location map and populate environment
        let mut location_map = HashMap::new();

        // We process each barangay and place agents within it
        let mut all_agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let num_barangays = config.barangays.len();

        for (b_idx, barangay) in config.barangays.iter().enumerate() {
            // Divide world horizontally for comparison mode
            let x_offset = if num_barangays == 2 {
                b_idx as f32 * (w / 2.0)
            } else {
                0.0
            };
            let zone_width = if num_barangays == 2 { w / 2.0 } else { w };
            let margin = 20.0;

            // --- Lay out zones within the barangay area ---
            // Residential: top-left quadrant
            let res_x0 = x_offset + margin;
            let res_y0 = margin;
            let res_w = zone_width * 0.45;
            let res_h = h * 0.55;

            // School area: top-right
            let school_x0 = x_offset + zone_width * 0.50;
            let school_y0 = margin;

            // Health center: right-center
            let hc_x = x_offset + zone_width * 0.75;
            let hc_y = h * 0.40;

            // Community/market: bottom-center
            let comm_x = x_offset + zone_width * 0.40;
            let comm_y = h * 0.70;

            // Creek: bottom band
            let creek_y = h * 0.85;

            // Register location positions
            location_map.insert(LocationKey::Community, (comm_x, comm_y));
            location_map.insert(LocationKey::HealthCenter, (hc_x, hc_y));

            // Set terrain for creek area
            let grid_w = environment.grid_width();
            let grid_h = environment.grid_height();
            for gx in 0..grid_w {
                let wx = gx as f32 * cell_size;
                if wx >= x_offset && wx < x_offset + zone_width {
                    let creek_cy = (creek_y / cell_size) as i32;
                    if let Some(cell) = environment.get_cell_mut(gx as i32, creek_cy) {
                        cell.terrain = TerrainType::Creek;
                    }
                    // Also mark one row above as OpenField
                    if let Some(cell) = environment.get_cell_mut(gx as i32, creek_cy - 1) {
                        cell.terrain = TerrainType::OpenField;
                    }
                }
            }

            // Set baseline soil contamination
            for gy in 0..grid_h {
                for gx in 0..grid_w {
                    let wx = gx as f32 * cell_size;
                    if wx >= x_offset && wx < x_offset + zone_width {
                        if let Some(cell) = environment.get_cell_mut(gx as i32, gy as i32) {
                            cell.soil_contamination = barangay.baseline_soil_contamination
                                * rng.gen_range(0.5..1.5);
                            cell.soil_contamination =
                                cell.soil_contamination.clamp(0.0, 1.0);
                        }
                    }
                }
            }

            // Place health center facility
            environment.add_facility(hc_x, hc_y, FacilityType::HealthCenter);

            // --- Place schools ---
            for s_idx in 0..barangay.num_schools {
                let sx = school_x0 + (s_idx as f32 * 60.0).min(zone_width * 0.3);
                let sy = school_y0 + 40.0 + s_idx as f32 * 80.0;
                let sy = sy.min(h * 0.5);

                location_map.insert(LocationKey::School(s_idx), (sx, sy));
                environment.add_facility(sx, sy, FacilityType::School);

                // Set school terrain cells
                let (cx, cy) = environment.world_to_cell(sx, sy);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if let Some(cell) = environment.get_cell_mut(cx + dx, cy + dy) {
                            cell.terrain = TerrainType::School;
                        }
                    }
                }
            }

            // --- Place households in a grid ---
            let homes_per_row = ((barangay.num_households as f32).sqrt().ceil()) as u16;
            let home_spacing_x = res_w / (homes_per_row as f32 + 1.0);
            let home_spacing_y = res_h / (homes_per_row as f32 + 1.0);

            for h_idx in 0..barangay.num_households {
                let row = h_idx / homes_per_row;
                let col = h_idx % homes_per_row;
                let hx = res_x0 + (col as f32 + 1.0) * home_spacing_x;
                let hy = res_y0 + (row as f32 + 1.0) * home_spacing_y;
                let hx = hx.min(x_offset + zone_width - margin);
                let hy = hy.min(h - margin);

                location_map.insert(LocationKey::Home(h_idx), (hx, hy));

                // Determine structural access probabilistically
                let has_latrine = rng.gen::<f32>() < barangay.latrine_coverage;
                let _has_water = rng.gen::<f32>() < barangay.water_supply_coverage;

                if has_latrine {
                    environment.add_facility(hx + 5.0, hy + 5.0, FacilityType::Latrine);
                }
            }

            // --- Place water sources ---
            let num_water_sources = (barangay.num_households as f32
                * barangay.water_supply_coverage
                / 10.0)
                .ceil() as u8;
            for ws_idx in 0..num_water_sources.max(1) {
                let wsx = x_offset + margin + rng.gen_range(0.0..zone_width - margin * 2.0);
                let wsy = h * 0.6 + rng.gen_range(0.0..h * 0.15);
                location_map.insert(LocationKey::WaterSource(ws_idx), (wsx, wsy));
                environment.add_facility(wsx, wsy, FacilityType::WaterPump);
            }

            // --- Place handwash stations at schools ---
            for s_idx in 0..barangay.num_schools {
                if rng.gen::<f32>() < barangay.handwash_station_coverage {
                    if let Some(&(sx, sy)) = location_map.get(&LocationKey::School(s_idx)) {
                        environment
                            .add_facility(sx + 10.0, sy + 10.0, FacilityType::HandwashStation);
                    }
                }
            }

            // --- Spawn children ---
            let children_per_household =
                (barangay.child_population as f32 / barangay.num_households as f32).ceil() as u16;

            let mut child_count = 0u16;
            for h_idx in 0..barangay.num_households {
                if child_count >= barangay.child_population {
                    break;
                }

                let has_latrine = rng.gen::<f32>() < barangay.latrine_coverage;
                let has_water = rng.gen::<f32>() < barangay.water_supply_coverage;
                let school_has_wash =
                    rng.gen::<f32>() < barangay.handwash_station_coverage;
                let school_id = (h_idx % barangay.num_schools as u16) as u8;

                let &(home_x, home_y) = location_map
                    .get(&LocationKey::Home(h_idx))
                    .unwrap_or(&(res_x0 + margin, res_y0 + margin));

                let children_this_household =
                    children_per_household.min(barangay.child_population - child_count);

                for _ in 0..children_this_household {
                    let age = rng.gen_range(6..=12);
                    // Gaussian-ish KAP spread around barangay mean
                    let k = (barangay.mean_knowledge
                        + rng.gen_range(-0.15..0.15))
                        .clamp(0.0, 1.0);
                    let a = (barangay.mean_attitude
                        + rng.gen_range(-0.15..0.15))
                        .clamp(0.0, 1.0);
                    let p = (barangay.mean_practice
                        + rng.gen_range(-0.15..0.15))
                        .clamp(0.0, 1.0);

                    let cx = home_x + rng.gen_range(-3.0..3.0);
                    let cy = home_y + rng.gen_range(-3.0..3.0);
                    let cx = cx.clamp(0.0, w);
                    let cy = cy.clamp(0.0, h);

                    let mut child = Agent::new_child(
                        h_idx,
                        school_id,
                        b_idx as u8,
                        age,
                        cx,
                        cy,
                        k,
                        a,
                        p,
                        has_latrine,
                        has_water,
                        school_has_wash,
                    );

                    // Seed baseline infections from thesis prevalence rates
                    let ascaris_prev = match barangay.setting {
                        crate::barangay::SettingType::Urban => 0.203,
                        crate::barangay::SettingType::Rural => 0.049,
                    };
                    let trichuris_prev = match barangay.setting {
                        crate::barangay::SettingType::Urban => 0.094,
                        crate::barangay::SettingType::Rural => 0.012,
                    };
                    // Hookworm: 0% in thesis data for both settings
                    let hookworm_prev: f32 = 0.0;

                    if rng.gen::<f32>() < ascaris_prev {
                        child.ascaris.worm_burden = rng.gen_range(1.0..20.0);
                        child.ascaris.epg = child.ascaris.worm_burden * 10000.0;
                        child.ascaris.days_infected = rng.gen_range(30..365);
                    }
                    if rng.gen::<f32>() < trichuris_prev {
                        child.trichuris.worm_burden = rng.gen_range(1.0..10.0);
                        child.trichuris.epg = child.trichuris.worm_burden * 1000.0;
                        child.trichuris.days_infected = rng.gen_range(30..365);
                    }
                    if rng.gen::<f32>() < hookworm_prev {
                        child.hookworm.worm_burden = rng.gen_range(0.5..5.0);
                        child.hookworm.epg = child.hookworm.worm_burden * 1000.0;
                        child.hookworm.days_infected = rng.gen_range(30..365);
                    }

                    all_agents.insert(child);
                    child_count += 1;
                }

                // Spawn a parent for every ~2 children households
                if h_idx % 2 == 0 {
                    let parent = Agent::new_adult(
                        AgentType::Parent,
                        h_idx,
                        school_id,
                        b_idx as u8,
                        rng.gen_range(25..55),
                        home_x + rng.gen_range(-2.0..2.0),
                        home_y + rng.gen_range(-2.0..2.0),
                        has_latrine,
                        has_water,
                    );
                    all_agents.insert(parent);
                }
            }

            // --- Spawn teachers (1 per ~30 children per school) ---
            for s_idx in 0..barangay.num_schools {
                let teachers_needed =
                    (barangay.child_population as f32 / barangay.num_schools as f32 / 30.0)
                        .ceil() as u8;
                let &(sx, sy) = location_map
                    .get(&LocationKey::School(s_idx))
                    .unwrap_or(&(school_x0, school_y0));

                for _ in 0..teachers_needed.max(1) {
                    // Teacher lives in a random household
                    let t_household = rng.gen_range(0..barangay.num_households);
                    let teacher = Agent::new_adult(
                        AgentType::Teacher,
                        t_household,
                        s_idx,
                        b_idx as u8,
                        rng.gen_range(25..55),
                        sx + rng.gen_range(-5.0..5.0),
                        sy + rng.gen_range(-5.0..5.0),
                        rng.gen::<f32>() < barangay.latrine_coverage,
                        rng.gen::<f32>() < barangay.water_supply_coverage,
                    );
                    all_agents.insert(teacher);
                }
            }

            // --- Spawn health workers ---
            for _ in 0..barangay.num_health_workers {
                let hw_household = rng.gen_range(0..barangay.num_households);
                let hw = Agent::new_adult(
                    AgentType::HealthWorker,
                    hw_household,
                    0,
                    b_idx as u8,
                    rng.gen_range(25..55),
                    hc_x + rng.gen_range(-5.0..5.0),
                    hc_y + rng.gen_range(-5.0..5.0),
                    rng.gen::<f32>() < barangay.latrine_coverage,
                    rng.gen::<f32>() < barangay.water_supply_coverage,
                );
                all_agents.insert(hw);
            }
        }

        // Initial stats
        let barangay_stats = config
            .barangays
            .iter()
            .map(|_| BarangayStats::default())
            .collect();

        let mut world = Self {
            budget_remaining: config.total_budget,
            config,
            agents: all_agents,
            environment,
            active_interventions: Vec::new(),
            tick: 0,
            budget_spent: 0.0,
            total_mda_rounds: 0,
            total_children_treated: 0,
            total_latrines_built: 0,
            total_water_sources: 0,
            barangay_stats,
            rng,
            render_buffer: SthRenderBuffer::new(),
            event_tracker: SthEventTracker::new(),
            location_map,
            prev_prevalence: 0.0,
        };

        // --- Apply auto-WASH placement from config ---
        let w = world.config.world_width;
        let h = world.config.world_height;
        let margin = 20.0;

        for _ in 0..world.config.auto_latrines {
            let x = world.rng.gen_range(margin..w - margin);
            let y = world.rng.gen_range(margin..h - margin);
            world.environment.add_facility(x, y, FacilityType::Latrine);
            world.total_latrines_built += 1;
            // Update latrine access for nearby agents (within 50px)
            for agent in world.agents.values_mut() {
                let dx = agent.x - x;
                let dy = agent.y - y;
                if dx * dx + dy * dy < 2500.0 {
                    agent.household_has_latrine = true;
                }
            }
        }

        for _ in 0..world.config.auto_water_pumps {
            let x = world.rng.gen_range(margin..w - margin);
            let y = world.rng.gen_range(margin..h - margin);
            world.environment.add_facility(x, y, FacilityType::WaterPump);
            world.total_water_sources += 1;
        }

        // --- Apply initial KAP overrides ---
        if let Some(k) = world.config.initial_knowledge {
            let k = k.clamp(0.0, 1.0);
            for agent in world.agents.values_mut() {
                if agent.agent_type == AgentType::Child {
                    agent.knowledge = k;
                }
            }
        }
        if let Some(a) = world.config.initial_attitude {
            let a = a.clamp(0.0, 1.0);
            for agent in world.agents.values_mut() {
                if agent.agent_type == AgentType::Child {
                    agent.attitude = a;
                }
            }
        }
        if let Some(p) = world.config.initial_practice {
            let p = p.clamp(0.0, 1.0);
            for agent in world.agents.values_mut() {
                if agent.agent_type == AgentType::Child {
                    agent.practice = p;
                }
            }
        }

        // --- Auto-launch education if configured ---
        if world.config.auto_education > 0 {
            let num_schools = world.config.barangays[0].num_schools;
            for s_idx in 0..num_schools {
                world.launch_education(0, s_idx as i8); // method 0 = default
            }
        }

        world.update_barangay_stats();
        world.prev_prevalence = world.barangay_stats[0].prevalence_any;
        world.render_buffer.pack(&world.agents, &world.environment);

        Ok(world)
    }

    // -----------------------------------------------------------------------
    // Main step
    // -----------------------------------------------------------------------

    /// Advance the simulation by one tick (1 hour of simulated time).
    ///
    /// Nine-phase loop:
    /// 1. Schedule & Movement
    /// 2. Behavioral Decisions
    /// 3. Contamination Deposit
    /// 4. Transmission
    /// 5. Environment Decay
    /// 6. Process Interventions
    /// 7. KAP Evolution (daily)
    /// 8. Rain Events (daily probabilistic)
    /// 9. Pack Render Buffer
    pub fn step(&mut self) {
        let time_of_day = TimeOfDay::from_tick(self.tick);
        let is_school_day = TimeOfDay::is_school_day(self.tick);
        let day = TimeOfDay::day_of_simulation(self.tick);

        // Check for COVID school closures
        let month = TimeOfDay::month_of_simulation(self.tick);
        let covid_active = match (
            self.config.covid_school_closure_start,
            self.config.covid_school_closure_end,
        ) {
            (Some(start), Some(end)) => month >= start && month < end,
            _ => false,
        };
        let effective_school_day = is_school_day && !covid_active;

        // --- Phase 1: Schedule & Movement ---
        let keys: Vec<AgentKey> = self.agents.keys().collect();
        for &key in &keys {
            let target = {
                let agent = &self.agents[key];
                schedule::target_location(agent, time_of_day, effective_school_day)
            };
            let (tx, ty) = self.location_to_position(target);
            let agent = self.agents.get_mut(key).unwrap();

            // If location changed, compute a new personal scatter offset
            if agent.current_location != target {
                let radius = scatter_radius(target);
                let hash = agent_scatter_hash(key, self.tick);
                let angle = hash * std::f32::consts::TAU;
                // Use a second hash for distance to avoid ring patterns
                let hash2 = agent_scatter_hash_secondary(key);
                let dist_frac = hash2.sqrt(); // sqrt for uniform disk distribution
                let offset_x = angle.cos() * radius * dist_frac;
                let offset_y = angle.sin() * radius * dist_frac;
                agent.target_x = tx + offset_x;
                agent.target_y = ty + offset_y;
            }
            agent.current_location = target;

            // Smooth movement toward personal target (with scatter offset)
            let dx = agent.target_x - agent.x;
            let dy = agent.target_y - agent.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 2.0 {
                let speed = 5.0_f32; // pixels per tick
                let step = speed.min(dist);
                agent.x += dx / dist * step;
                agent.y += dy / dist * step;
            } else {
                // Idle wander: small random nudge so agents look alive
                agent.x += self.rng.gen_range(-0.3..0.3);
                agent.y += self.rng.gen_range(-0.3..0.3);
            }

            // Clamp to world bounds
            agent.x = agent.x.clamp(0.0, self.config.world_width);
            agent.y = agent.y.clamp(0.0, self.config.world_height);
        }

        // --- Phase 2: Behavioral Decisions (hourly, children only) ---
        for &key in &keys {
            let agent = &mut self.agents[key];
            if agent.agent_type != AgentType::Child {
                continue;
            }

            let kap = KapScores {
                knowledge: agent.knowledge,
                attitude: agent.attitude,
                practice: agent.practice,
            };
            agent.washes_hands_before_eating = self.rng.gen::<f32>()
                < kap.handwashing_probability()
                && agent.school_has_wash_facility;
            agent.wears_shoes = self.rng.gen::<f32>() < kap.shoe_wearing_probability();
            agent.uses_latrine =
                self.rng.gen::<f32>() < kap.latrine_use_probability(agent.household_has_latrine);
        }

        // --- Phase 3: Contamination Deposit (once per day at EarlyMorning) ---
        if time_of_day == TimeOfDay::EarlyMorning {
            for &key in &keys {
                let agent = &self.agents[key];
                if !agent.uses_latrine {
                    let total_epg =
                        agent.ascaris.epg + agent.trichuris.epg + agent.hookworm.epg;
                    if total_epg > 0.0 {
                        let amount = (total_epg / 5_000_000.0).min(0.01);
                        self.environment
                            .deposit_contamination(agent.x, agent.y, amount);
                    }
                }
            }
        }

        // --- Phase 4: Transmission (children only, not during sleep) ---
        for &key in &keys {
            let agent = &mut self.agents[key];
            if agent.agent_type != AgentType::Child {
                continue;
            }
            if time_of_day == TimeOfDay::Night {
                continue;
            }

            let (soil_c, _water_c) = self.environment.sample_contamination(agent.x, agent.y);
            let location_risk = match agent.current_location {
                Location::Community => 1.2,
                Location::School(_) => 0.8,
                Location::Home(_) => 0.6,
                Location::WaterSource(_) => 1.5,
                Location::HealthCenter => 0.3,
            };

            for (species, state) in [
                (SthSpecies::Ascaris, &mut agent.ascaris),
                (SthSpecies::Trichuris, &mut agent.trichuris),
                (SthSpecies::Hookworm, &mut agent.hookworm),
            ] {
                // Species-specific effective contamination
                // Ascaris/Trichuris eggs survive months-years in soil
                // Hookworm larvae survive only 2-6 weeks — effective contamination is much lower
                let species_contamination_factor = match species {
                    SthSpecies::Ascaris => 1.0,
                    SthSpecies::Trichuris => 1.0,
                    SthSpecies::Hookworm => 0.1,
                };
                let effective_soil_c = soil_c * species_contamination_factor;

                let prob = infection::transmission_probability(
                    species,
                    effective_soil_c,
                    agent.wears_shoes,
                    agent.washes_hands_before_eating,
                    agent.uses_latrine,
                    location_risk,
                );
                infection::update_worm_burden(state, prob, &mut self.rng, species);
            }
        }

        // --- Phase 5: Environment Decay ---
        self.environment.update_contamination();

        // --- Phase 6: Process Interventions ---
        self.process_interventions();

        // --- Phase 7: KAP Evolution (daily at midnight) ---
        if self.tick % 24 == 0 && self.tick > 0 {
            self.update_kap_scores();
            self.update_barangay_stats();

            // Check for prevalence changes
            let current_prev = if !self.barangay_stats.is_empty() {
                self.barangay_stats[0].prevalence_any
            } else {
                0.0
            };
            if self.prev_prevalence > 0.0
                && current_prev < self.prev_prevalence * 0.9
                && self.prev_prevalence - current_prev > 0.02
            {
                self.event_tracker.push(
                    self.tick,
                    SthEventType::PrevalenceDropped {
                        from: self.prev_prevalence,
                        to: current_prev,
                    },
                    format!(
                        "Prevalence dropped from {:.1}% to {:.1}%",
                        self.prev_prevalence * 100.0,
                        current_prev * 100.0
                    ),
                );
            }
            if current_prev > 0.20 && self.prev_prevalence <= 0.20 {
                self.event_tracker.push(
                    self.tick,
                    SthEventType::OutbreakDetected {
                        prevalence: current_prev,
                    },
                    format!(
                        "Outbreak detected: {:.1}% prevalence exceeds 20% threshold",
                        current_prev * 100.0
                    ),
                );
            }
            self.prev_prevalence = current_prev;

            // Monthly budget increment
            if day % 30 == 0 && day > 0 {
                self.budget_remaining += self.config.monthly_budget_increment;
            }

            // Track days since deworming for all children
            for &key in &keys {
                if let Some(agent) = self.agents.get_mut(key) {
                    if agent.agent_type == AgentType::Child {
                        agent.days_since_last_deworming =
                            agent.days_since_last_deworming.saturating_add(1);
                    }
                }
            }
        }

        // --- Phase 8: Rain Events (daily probabilistic check) ---
        if self.tick % 24 == 0 {
            if self.rng.gen::<f32>() < self.config.rain_frequency {
                self.environment
                    .spread_contamination_rain(self.config.rain_contamination_spread);
                self.event_tracker.push(
                    self.tick,
                    SthEventType::RainEvent,
                    "Heavy rain event — contamination spreading".to_string(),
                );
            }
        }

        // --- Auto MDA (biannual deworming if configured) ---
        if let Some(interval) = self.config.auto_mda_interval_days {
            let day = TimeOfDay::day_of_simulation(self.tick);
            let hour = self.tick % 24;
            // Launch MDA on the scheduled day at hour 8 (morning)
            if day > 0 && day % interval == 0 && hour == 8 {
                let num_schools = self.config.barangays[0].num_schools;
                let auto_coverage = self.config.auto_mda_coverage;
                let auto_drug = self.config.auto_mda_drug;
                for s_idx in 0..num_schools {
                    self.launch_mda_with_coverage(s_idx as i8, auto_drug, auto_coverage);
                }
            }
        }

        // --- Phase 9: Pack Render Buffer ---
        self.render_buffer.pack(&self.agents, &self.environment);

        self.tick += 1;
    }

    // -----------------------------------------------------------------------
    // Intervention API
    // -----------------------------------------------------------------------

    /// Launch Mass Drug Administration.
    /// `school_id`: -1 for all schools, 0+ for specific school.
    /// `drug`: 0 = Albendazole, 1 = Mebendazole.
    /// `coverage`: fraction of eligible children to treat (0.0-1.0).
    pub fn launch_mda_with_coverage(&mut self, school_id: i8, drug: u8, coverage: f32) {
        let coverage = coverage.clamp(0.0, 1.0);
        let drug = match drug {
            1 => Drug::Mebendazole,
            _ => Drug::Albendazole,
        };
        let target_school = if school_id < 0 {
            None
        } else {
            Some(school_id as u8)
        };

        // Calculate cost
        let child_count = self
            .agents
            .values()
            .filter(|a| a.agent_type == AgentType::Child)
            .count() as u16;
        let intervention = InterventionType::MDA {
            target_school,
            drug,
            coverage_pct: coverage,
        };
        let cost = intervention::intervention_cost(
            &intervention,
            child_count,
            self.config.barangays.iter().map(|b| b.num_households).sum(),
        );

        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for MDA campaign".to_string(),
            );
            return;
        }

        // Check medicine supply — use the barangay that owns the target school
        let barangay_idx = target_school
            .and_then(|sid| {
                self.agents
                    .values()
                    .find(|a| a.school_id == sid)
                    .map(|a| a.barangay_id as usize)
            })
            .unwrap_or(0);
        let medicine_supply = self
            .config
            .barangays
            .get(barangay_idx)
            .unwrap_or(&self.config.barangays[0])
            .medicine_supply_reliability;
        if self.rng.gen::<f32>() > medicine_supply {
            self.event_tracker.push(
                self.tick,
                SthEventType::MedicineStockout,
                "Medicine stockout — MDA campaign delayed".to_string(),
            );
            return;
        }

        let treated = intervention::apply_mda(
            &mut self.agents,
            drug,
            target_school,
            coverage,
            medicine_supply,
            &mut self.rng,
        );

        self.budget_remaining -= cost;
        self.budget_spent += cost;
        self.total_mda_rounds += 1;
        self.total_children_treated += treated;

        let duration = intervention::intervention_duration_ticks(&intervention);
        self.active_interventions.push(ActiveIntervention {
            intervention,
            start_tick: self.tick,
            duration_ticks: duration,
            cost,
            progress: 1.0, // MDA is applied immediately
        });

        self.event_tracker.push(
            self.tick,
            SthEventType::MdaCompleted {
                school_id: target_school,
                treated,
            },
            format!("MDA completed: {treated} children treated"),
        );
    }

    /// Launch MDA with default coverage (0.8) — kept for backward compatibility.
    /// `school_id`: -1 for all schools, 0+ for specific school.
    /// `drug`: 0 = Albendazole, 1 = Mebendazole.
    pub fn launch_mda(&mut self, school_id: i8, drug: u8) {
        self.launch_mda_with_coverage(school_id, drug, 0.8);
    }

    /// Build a latrine at the given world position.
    pub fn build_latrine(&mut self, x: f32, y: f32) {
        let cost = self.config.latrine_cost;
        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for latrine".to_string(),
            );
            return;
        }

        self.environment.add_facility(x, y, FacilityType::Latrine);
        self.budget_remaining -= cost;
        self.budget_spent += cost;
        self.total_latrines_built += 1;

        // Update nearby agents' structural access
        for agent in self.agents.values_mut() {
            let dx = agent.x - x;
            let dy = agent.y - y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 2500.0 {
                // within 50px
                agent.household_has_latrine = true;
            }
        }

        self.event_tracker.push(
            self.tick,
            SthEventType::FacilityBuilt {
                facility_type: "Latrine".to_string(),
                x,
                y,
            },
            format!("Latrine built at ({x:.0}, {y:.0})"),
        );
    }

    /// Build a water pump at the given world position.
    pub fn build_water_pump(&mut self, x: f32, y: f32) {
        let cost = self.config.water_pump_cost;
        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for water pump".to_string(),
            );
            return;
        }

        self.environment
            .add_facility(x, y, FacilityType::WaterPump);
        self.budget_remaining -= cost;
        self.budget_spent += cost;
        self.total_water_sources += 1;

        // Update nearby agents' water access
        for agent in self.agents.values_mut() {
            let dx = agent.x - x;
            let dy = agent.y - y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 2500.0 {
                agent.household_has_water = true;
            }
        }

        self.event_tracker.push(
            self.tick,
            SthEventType::FacilityBuilt {
                facility_type: "Water Pump".to_string(),
                x,
                y,
            },
            format!("Water pump built at ({x:.0}, {y:.0})"),
        );
    }

    /// Build a handwash station at the given world position.
    pub fn build_handwash_station(&mut self, x: f32, y: f32) {
        let cost = self.config.handwash_station_cost;
        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for handwash station".to_string(),
            );
            return;
        }

        self.environment
            .add_facility(x, y, FacilityType::HandwashStation);
        self.budget_remaining -= cost;
        self.budget_spent += cost;

        // Update nearby agents' WASH facility access
        for agent in self.agents.values_mut() {
            let dx = agent.x - x;
            let dy = agent.y - y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 2500.0 {
                agent.school_has_wash_facility = true;
            }
        }

        self.event_tracker.push(
            self.tick,
            SthEventType::FacilityBuilt {
                facility_type: "Handwash Station".to_string(),
                x,
                y,
            },
            format!("Handwash station built at ({x:.0}, {y:.0})"),
        );
    }

    /// Launch a health education campaign.
    /// `method`: 0=Cartoon, 1=BoardGame, 2=TeacherLed, 3=ParentMeeting.
    /// `school_id`: -1 for all schools, 0+ for specific school.
    pub fn launch_education(&mut self, method: u8, school_id: i8) {
        let method = match method {
            1 => EducationMethod::BoardGame,
            2 => EducationMethod::TeacherLed,
            3 => EducationMethod::ParentMeeting,
            _ => EducationMethod::Cartoon,
        };
        let target_school = if school_id < 0 {
            None
        } else {
            Some(school_id as u8)
        };

        let intervention = InterventionType::Education {
            method,
            target_school,
        };
        let child_count = self
            .agents
            .values()
            .filter(|a| a.agent_type == AgentType::Child)
            .count() as u16;
        let cost = intervention::intervention_cost(
            &intervention,
            child_count,
            self.config.barangays.iter().map(|b| b.num_households).sum(),
        );

        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for education".to_string(),
            );
            return;
        }

        let affected =
            intervention::apply_education(&mut self.agents, method, target_school, &mut self.rng);

        self.budget_remaining -= cost;
        self.budget_spent += cost;

        let method_name = match method {
            EducationMethod::Cartoon => "Cartoon",
            EducationMethod::BoardGame => "Board Game",
            EducationMethod::TeacherLed => "Teacher-Led",
            EducationMethod::ParentMeeting => "Parent Meeting",
        };

        self.event_tracker.push(
            self.tick,
            SthEventType::EducationCompleted {
                method: method_name.to_string(),
                affected,
            },
            format!("{method_name} education: {affected} agents reached"),
        );
    }

    /// Launch BHW house-to-house visits.
    /// `coverage`: fraction of households to visit (0.0-1.0).
    pub fn launch_bhw_visits(&mut self, coverage: f32) {
        let coverage = coverage.clamp(0.0, 1.0);
        let intervention = InterventionType::BhwVisits {
            coverage_pct: coverage,
            duration_days: 7,
        };
        let household_count: u16 = self.config.barangays.iter().map(|b| b.num_households).sum();
        let cost = intervention::intervention_cost(&intervention, 0, household_count);

        if cost > self.budget_remaining {
            self.event_tracker.push(
                self.tick,
                SthEventType::BudgetDepleted,
                "Insufficient budget for BHW visits".to_string(),
            );
            return;
        }

        let visited =
            intervention::apply_bhw_visits(&mut self.agents, coverage, &mut self.rng);

        self.budget_remaining -= cost;
        self.budget_spent += cost;

        self.event_tracker.push(
            self.tick,
            SthEventType::BhwVisitsCompleted { visited },
            format!("BHW visits completed: {visited} agents visited"),
        );
    }

    /// Increase the monthly budget increment by a multiplier.
    pub fn increase_budget(&mut self, multiplier: f32) {
        let multiplier = multiplier.max(1.0);
        self.config.monthly_budget_increment *= multiplier;
    }

    // -----------------------------------------------------------------------
    // Stats
    // -----------------------------------------------------------------------

    /// Compute per-barangay statistics.
    pub fn update_barangay_stats(&mut self) {
        for (b_idx, stats) in self.barangay_stats.iter_mut().enumerate() {
            let b_id = b_idx as u8;

            // Populate name and setting from config
            if b_idx < self.config.barangays.len() {
                let bc = &self.config.barangays[b_idx];
                stats.name = bc.name.clone();
                stats.setting = match bc.setting {
                    crate::barangay::SettingType::Urban => "urban".to_string(),
                    crate::barangay::SettingType::Rural => "rural".to_string(),
                };
            }

            let children: Vec<&Agent> = self
                .agents
                .values()
                .filter(|a| a.barangay_id == b_id && a.agent_type == AgentType::Child)
                .collect();

            let n = children.len() as f32;
            if n == 0.0 {
                let name = stats.name.clone();
                let setting = stats.setting.clone();
                *stats = BarangayStats { name, setting, ..BarangayStats::default() };
                continue;
            }

            let mut asc_count = 0u32;
            let mut tri_count = 0u32;
            let mut hook_count = 0u32;
            let mut any_count = 0u32;
            let mut total_epg = 0.0f32;
            let mut total_k = 0.0f32;
            let mut total_a = 0.0f32;
            let mut total_p = 0.0f32;
            let mut latrine_count = 0u32;
            let mut water_count = 0u32;

            for child in &children {
                if child.ascaris.is_infected() {
                    asc_count += 1;
                }
                if child.trichuris.is_infected() {
                    tri_count += 1;
                }
                if child.hookworm.is_infected() {
                    hook_count += 1;
                }
                if child.is_infected() {
                    any_count += 1;
                }
                total_epg += child.total_epg();
                total_k += child.knowledge;
                total_a += child.attitude;
                total_p += child.practice;
                if child.household_has_latrine {
                    latrine_count += 1;
                }
                if child.household_has_water {
                    water_count += 1;
                }
            }

            stats.prevalence_ascaris = asc_count as f32 / n;
            stats.prevalence_trichuris = tri_count as f32 / n;
            stats.prevalence_hookworm = hook_count as f32 / n;
            stats.prevalence_any = any_count as f32 / n;
            stats.mean_epg = total_epg / n;
            stats.mean_knowledge = total_k / n;
            stats.mean_attitude = total_a / n;
            stats.mean_practice = total_p / n;
            stats.latrine_coverage = latrine_count as f32 / n;
            stats.water_coverage = water_count as f32 / n;
        }
    }

    /// Get stats as a JSON string.
    pub fn get_stats_json(&self) -> String {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct GlobalStats<'a> {
            tick: u64,
            day: u32,
            month: u32,
            agent_count: usize,
            prevalence_ascaris: f32,
            prevalence_trichuris: f32,
            prevalence_hookworm: f32,
            prevalence_any: f32,
            mean_epg: f32,
            mean_knowledge: f32,
            mean_attitude: f32,
            mean_practice: f32,
            latrine_coverage: f32,
            water_coverage: f32,
            budget_remaining: f32,
            budget_spent: f32,
            mda_rounds: u32,
            children_treated: u32,
            barangays: &'a [BarangayStats],
        }

        // Compute aggregate stats across all barangays
        let total_children = self
            .agents
            .values()
            .filter(|a| a.agent_type == AgentType::Child)
            .count() as f32;

        let (agg_prev_asc, agg_prev_tri, agg_prev_hook, agg_prev_any,
             agg_epg, agg_k, agg_a, agg_p, agg_lat, agg_wat) = if total_children > 0.0 {
            let children: Vec<&Agent> = self.agents.values()
                .filter(|a| a.agent_type == AgentType::Child)
                .collect();
            let n = children.len() as f32;
            let mut asc = 0u32;
            let mut tri = 0u32;
            let mut hook = 0u32;
            let mut any = 0u32;
            let mut epg = 0.0f32;
            let mut k = 0.0f32;
            let mut a = 0.0f32;
            let mut p = 0.0f32;
            let mut lat = 0u32;
            let mut wat = 0u32;
            for c in &children {
                if c.ascaris.is_infected() { asc += 1; }
                if c.trichuris.is_infected() { tri += 1; }
                if c.hookworm.is_infected() { hook += 1; }
                if c.is_infected() { any += 1; }
                epg += c.total_epg();
                k += c.knowledge;
                a += c.attitude;
                p += c.practice;
                if c.household_has_latrine { lat += 1; }
                if c.household_has_water { wat += 1; }
            }
            (asc as f32 / n, tri as f32 / n, hook as f32 / n, any as f32 / n,
             epg / n, k / n, a / n, p / n, lat as f32 / n, wat as f32 / n)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        };

        let stats = GlobalStats {
            tick: self.tick,
            day: TimeOfDay::day_of_simulation(self.tick),
            month: TimeOfDay::month_of_simulation(self.tick),
            agent_count: self.agents.len(),
            prevalence_ascaris: agg_prev_asc,
            prevalence_trichuris: agg_prev_tri,
            prevalence_hookworm: agg_prev_hook,
            prevalence_any: agg_prev_any,
            mean_epg: agg_epg,
            mean_knowledge: agg_k,
            mean_attitude: agg_a,
            mean_practice: agg_p,
            latrine_coverage: agg_lat,
            water_coverage: agg_wat,
            budget_remaining: self.budget_remaining,
            budget_spent: self.budget_spent,
            mda_rounds: self.total_mda_rounds,
            children_treated: self.total_children_treated,
            barangays: &self.barangay_stats,
        };

        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get detailed information about a specific agent by render-order index.
    pub fn get_agent_detail_json(&self, index: usize) -> String {
        let agent = self.agents.values().nth(index);
        match agent {
            Some(agent) => {
                #[derive(Serialize)]
                struct AgentDetail {
                    agent_type: String,
                    age: u8,
                    household_id: u16,
                    school_id: u8,
                    barangay_id: u8,
                    x: f32,
                    y: f32,
                    is_infected: bool,
                    ascaris_epg: f32,
                    trichuris_epg: f32,
                    hookworm_epg: f32,
                    knowledge: f32,
                    attitude: f32,
                    practice: f32,
                    has_latrine: bool,
                    has_water: bool,
                    wears_shoes: bool,
                    washes_hands: bool,
                    uses_latrine: bool,
                    days_since_deworming: u16,
                }

                let detail = AgentDetail {
                    agent_type: format!("{:?}", agent.agent_type),
                    age: agent.age_years,
                    household_id: agent.household_id,
                    school_id: agent.school_id,
                    barangay_id: agent.barangay_id,
                    x: agent.x,
                    y: agent.y,
                    is_infected: agent.is_infected(),
                    ascaris_epg: agent.ascaris.epg,
                    trichuris_epg: agent.trichuris.epg,
                    hookworm_epg: agent.hookworm.epg,
                    knowledge: agent.knowledge,
                    attitude: agent.attitude,
                    practice: agent.practice,
                    has_latrine: agent.household_has_latrine,
                    has_water: agent.household_has_water,
                    wears_shoes: agent.wears_shoes,
                    washes_hands: agent.washes_hands_before_eating,
                    uses_latrine: agent.uses_latrine,
                    days_since_deworming: agent.days_since_last_deworming,
                };

                serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string())
            }
            None => "{}".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Render buffer accessors
    // -----------------------------------------------------------------------

    pub fn agent_render_data(&self) -> &[f32] {
        self.render_buffer.agent_data()
    }

    pub fn env_render_data(&self) -> &[f32] {
        self.render_buffer.env_data()
    }

    pub fn facility_render_data(&self) -> &[f32] {
        self.render_buffer.facility_data()
    }

    pub fn agent_count(&self) -> usize {
        self.render_buffer.agent_count()
    }

    // -----------------------------------------------------------------------
    // Event tracker accessors
    // -----------------------------------------------------------------------

    pub fn drain_events_json(&mut self) -> String {
        self.event_tracker.drain_json()
    }

    pub fn events_json(&self) -> String {
        self.event_tracker.get_json()
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Map a Location enum to world (x, y) coordinates.
    fn location_to_position(&self, location: Location) -> (f32, f32) {
        let key = LocationKey::from(location);
        self.location_map
            .get(&key)
            .copied()
            .unwrap_or((self.config.world_width * 0.5, self.config.world_height * 0.5))
    }

    /// Update KAP scores for all children (called daily).
    fn update_kap_scores(&mut self) {
        // Compute community average KAP for social influence
        let mut total_k = 0.0f32;
        let mut child_count = 0u32;
        for agent in self.agents.values() {
            if agent.agent_type == AgentType::Child {
                total_k += agent.knowledge;
                child_count += 1;
            }
        }
        let community_avg = if child_count > 0 {
            total_k / child_count as f32
        } else {
            0.5
        };

        // Build parent KAP lookup by household_id
        let mut parent_kap: HashMap<u16, KapScores> = HashMap::new();
        for agent in self.agents.values() {
            if agent.agent_type == AgentType::Parent {
                parent_kap.insert(
                    agent.household_id,
                    KapScores::new(agent.knowledge, agent.attitude, agent.practice),
                );
            }
        }
        let default_parent = KapScores::default();

        // Update each child's KAP
        let keys: Vec<AgentKey> = self.agents.keys().collect();
        for key in keys {
            let agent = &self.agents[key];
            if agent.agent_type != AgentType::Child {
                continue;
            }
            let is_infected = agent.is_infected();
            let household_id = agent.household_id;
            let parent_kap_ref = parent_kap.get(&household_id).unwrap_or(&default_parent);

            let mut kap = KapScores {
                knowledge: agent.knowledge,
                attitude: agent.attitude,
                practice: agent.practice,
            };

            crate::kap::update_kap_from_experience(
                &mut kap,
                is_infected,
                false, // education handled via interventions
                parent_kap_ref,
                community_avg,
                &mut self.rng,
            );

            let agent = self.agents.get_mut(key).unwrap();
            agent.knowledge = kap.knowledge;
            agent.attitude = kap.attitude;
            agent.practice = kap.practice;
        }
    }

    /// Process active interventions: advance progress, remove completed ones.
    fn process_interventions(&mut self) {
        self.active_interventions.retain(|iv| {
            if iv.duration_ticks == 0 {
                return false; // instant interventions, already applied
            }
            let elapsed = self.tick.saturating_sub(iv.start_tick);
            elapsed < iv.duration_ticks
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_world() -> SthWorld {
        SthWorld::new(SthConfig::default()).expect("Default config should produce a valid world")
    }

    #[test]
    fn test_new_world_creates_agents() {
        let world = default_world();
        assert!(
            world.agents.len() > 0,
            "World should have agents after initialization"
        );

        // Should have children
        let child_count = world
            .agents
            .values()
            .filter(|a| a.agent_type == AgentType::Child)
            .count();
        assert!(child_count > 0, "Should have children");

        // Should have parents
        let parent_count = world
            .agents
            .values()
            .filter(|a| a.agent_type == AgentType::Parent)
            .count();
        assert!(parent_count > 0, "Should have parents");
    }

    #[test]
    fn test_new_world_agents_in_bounds() {
        let world = default_world();
        for agent in world.agents.values() {
            assert!(
                agent.x >= 0.0 && agent.x <= world.config.world_width,
                "Agent x={} out of bounds [0, {}]",
                agent.x,
                world.config.world_width
            );
            assert!(
                agent.y >= 0.0 && agent.y <= world.config.world_height,
                "Agent y={} out of bounds [0, {}]",
                agent.y,
                world.config.world_height
            );
        }
    }

    #[test]
    fn test_new_world_budget() {
        let world = default_world();
        assert_eq!(world.budget_remaining, world.config.total_budget);
        assert_eq!(world.budget_spent, 0.0);
    }

    #[test]
    fn test_step_increments_tick() {
        let mut world = default_world();
        assert_eq!(world.tick, 0);
        world.step();
        assert_eq!(world.tick, 1);
        world.step();
        assert_eq!(world.tick, 2);
    }

    #[test]
    fn test_step_keeps_agents_in_bounds() {
        let mut world = default_world();
        for _ in 0..48 {
            // 2 simulated days
            world.step();
        }
        for agent in world.agents.values() {
            assert!(
                agent.x >= 0.0 && agent.x <= world.config.world_width,
                "Agent x={} out of bounds after stepping",
                agent.x
            );
            assert!(
                agent.y >= 0.0 && agent.y <= world.config.world_height,
                "Agent y={} out of bounds after stepping",
                agent.y
            );
        }
    }

    #[test]
    fn test_render_buffer_populated() {
        let world = default_world();
        assert!(
            world.render_buffer.agent_count() > 0,
            "Render buffer should be populated after init"
        );
    }

    #[test]
    fn test_stats_json_valid() {
        let world = default_world();
        let json = world.get_stats_json();
        assert!(!json.is_empty());
        assert!(json.contains("tick"));
        assert!(json.contains("budgetRemaining"));
    }

    #[test]
    fn test_barangay_stats_computed() {
        let world = default_world();
        assert!(!world.barangay_stats.is_empty());
        // Prevalence should be between 0 and 1
        let stats = &world.barangay_stats[0];
        assert!(stats.prevalence_any >= 0.0 && stats.prevalence_any <= 1.0);
        assert!(stats.mean_knowledge >= 0.0 && stats.mean_knowledge <= 1.0);
    }

    #[test]
    fn test_location_to_position_returns_valid() {
        let world = default_world();
        let (x, y) = world.location_to_position(Location::Community);
        assert!(x >= 0.0 && x <= world.config.world_width);
        assert!(y >= 0.0 && y <= world.config.world_height);
    }

    #[test]
    fn test_launch_mda_reduces_budget() {
        let mut world = default_world();
        let initial_budget = world.budget_remaining;
        world.launch_mda(-1, 0);
        // Budget should decrease (or remain same if stockout/budget failure)
        assert!(
            world.budget_remaining <= initial_budget,
            "Budget should not increase after MDA"
        );
    }

    #[test]
    fn test_build_latrine_adds_facility() {
        let mut world = default_world();
        let initial_latrines = world.total_latrines_built;
        world.build_latrine(100.0, 100.0);
        assert_eq!(world.total_latrines_built, initial_latrines + 1);
    }

    #[test]
    fn test_build_latrine_budget_check() {
        let mut world = default_world();
        world.budget_remaining = 0.0;
        let initial_latrines = world.total_latrines_built;
        world.build_latrine(100.0, 100.0);
        assert_eq!(
            world.total_latrines_built, initial_latrines,
            "Should not build without budget"
        );
    }

    #[test]
    fn test_increase_budget() {
        let mut world = default_world();
        let initial = world.config.monthly_budget_increment;
        world.increase_budget(2.0);
        assert!(
            (world.config.monthly_budget_increment - initial * 2.0).abs() < 1e-6,
            "Budget increment should double"
        );
    }

    #[test]
    fn test_agent_detail_json() {
        let world = default_world();
        let json = world.get_agent_detail_json(0);
        assert!(!json.is_empty());
        assert!(json.contains("agent_type"));
    }

    #[test]
    fn test_agent_detail_out_of_bounds() {
        let world = default_world();
        let json = world.get_agent_detail_json(999999);
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_drain_events() {
        let mut world = default_world();
        // Run enough ticks for some events to potentially fire
        for _ in 0..48 {
            world.step();
        }
        let json = world.drain_events_json();
        assert!(!json.is_empty());
        // After drain, should be empty
        let json2 = world.drain_events_json();
        assert_eq!(json2, "[]");
    }

    #[test]
    fn test_comparison_mode() {
        use crate::barangay::BarangayConfig;
        let mut config = SthConfig::default();
        config.world_width = 1600.0;
        config.barangays = vec![
            BarangayConfig::urban_default(),
            BarangayConfig::rural_default(),
        ];
        let world = SthWorld::new(config).expect("Comparison config should be valid");

        assert_eq!(world.barangay_stats.len(), 2);
        assert!(world.agents.len() > 0);

        // Should have agents from both barangays
        let b0_count = world
            .agents
            .values()
            .filter(|a| a.barangay_id == 0)
            .count();
        let b1_count = world
            .agents
            .values()
            .filter(|a| a.barangay_id == 1)
            .count();
        assert!(b0_count > 0, "Should have barangay 0 agents");
        assert!(b1_count > 0, "Should have barangay 1 agents");
    }

    #[test]
    fn test_seed_determinism() {
        let world1 = SthWorld::new(SthConfig::default()).unwrap();
        let world2 = SthWorld::new(SthConfig::default()).unwrap();

        // Same seed should produce same initial agent count
        assert_eq!(world1.agents.len(), world2.agents.len());
    }

    #[test]
    fn test_step_24_ticks_is_one_day() {
        let mut world = default_world();
        for _ in 0..24 {
            world.step();
        }
        assert_eq!(TimeOfDay::day_of_simulation(world.tick), 1);
    }
}
