use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use slotmap::DenseSlotMap;

use crate::brain::NUM_INPUTS;
use crate::config::SimConfig;
use crate::creature::{Creature, CreatureKey};
use crate::physics;
use crate::profile::{TickProfile, Timer};
use crate::render_buffer::RenderBuffer;
use crate::spatial_hash::SpatialHash;
use crate::speciation::SpeciesTracker;

const MAX_CREATURES: usize = 10_000;
const POPULATION_SOFT_CAP: usize = 3_000;
const POPULATION_BLOCK_REPRO: usize = 5_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Food {
    pub x: f32,
    pub y: f32,
    pub energy: f32,
}

pub struct World {
    pub config: SimConfig,
    pub creatures: DenseSlotMap<CreatureKey, Creature>,
    pub food: Vec<Food>,
    pub tick: u64,
    pub generation_max: u32,
    pub total_births: u64,
    pub total_deaths: u64,
    pub nan_deaths: u32,
    pub next_species_id: u32,

    rng: SmallRng,
    spatial_hash: SpatialHash,
    food_spatial: FoodGrid,
    render_buffer: RenderBuffer,
    species_tracker: SpeciesTracker,
    last_profile: TickProfile,

    // Catastrophe state
    catastrophe_ticks_remaining: u32,

    // Energy audit
    energy_drift_pct: f32,
}

/// Simple grid for fast food lookup by creatures
struct FoodGrid {
    pub(crate) inv_cell_size: f32,
    pub(crate) grid_width: usize,
    pub(crate) cells: Vec<Vec<usize>>, // indices into World.food
}

impl FoodGrid {
    fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        let inv_cell_size = 1.0 / cell_size;
        let grid_width = (world_width * inv_cell_size).ceil() as usize;
        let grid_height = (world_height * inv_cell_size).ceil() as usize;
        Self {
            inv_cell_size,
            grid_width,
            cells: vec![Vec::new(); grid_width * grid_height],
        }
    }

    fn rebuild(&mut self, food: &[Food]) {
        for cell in self.cells.iter_mut() {
            cell.clear();
        }
        for (i, f) in food.iter().enumerate() {
            let cx = ((f.x * self.inv_cell_size) as usize).min(self.grid_width.saturating_sub(1));
            let cy = ((f.y * self.inv_cell_size) as usize)
                .min((self.cells.len() / self.grid_width).saturating_sub(1));
            let idx = cy * self.grid_width + cx;
            if idx < self.cells.len() {
                self.cells[idx].push(i);
            }
        }
    }

    fn query_nearest(
        &self,
        x: f32,
        y: f32,
        range_sq: f32,
        food: &[Food],
        world_width: f32,
        world_height: f32,
    ) -> Option<usize> {
        let cx = (x * self.inv_cell_size) as i32;
        let cy = (y * self.inv_cell_size) as i32;
        let gw = self.grid_width as i32;
        let gh = (self.cells.len() / self.grid_width) as i32;
        let cells_to_check = 2i32; // check 5x5 neighborhood

        let mut best_idx = None;
        let mut best_dist_sq = range_sq;

        for dy in -cells_to_check..=cells_to_check {
            for dx in -cells_to_check..=cells_to_check {
                let nx = ((cx + dx) % gw + gw) % gw;
                let ny = ((cy + dy) % gh + gh) % gh;
                let cell_idx = (ny * gw + nx) as usize;
                if cell_idx >= self.cells.len() {
                    continue;
                }
                for &fi in &self.cells[cell_idx] {
                    let d_sq = physics::distance_squared_toroidal(
                        x,
                        y,
                        food[fi].x,
                        food[fi].y,
                        world_width,
                        world_height,
                    );
                    if d_sq < best_dist_sq {
                        best_dist_sq = d_sq;
                        best_idx = Some(fi);
                    }
                }
            }
        }
        best_idx
    }
}

impl World {
    pub fn new(config: SimConfig) -> Result<Self, String> {
        config.validate()?;

        let mut rng = SmallRng::seed_from_u64(config.seed);
        let cell_size = config.max_vision_range;

        let mut creatures = DenseSlotMap::with_capacity_and_key(config.initial_population as usize);
        for _ in 0..config.initial_population {
            creatures.insert(Creature::new_random(&mut rng, &config, 0));
        }

        // Spawn initial food to fill energy budget
        let creature_energy: f32 = creatures.values().map(|c| c.energy).sum();
        let food_deficit = config.target_total_energy - creature_energy;
        let food_count = (food_deficit / config.food_energy).max(0.0) as usize;
        let mut food = Vec::with_capacity(food_count * 2);
        for _ in 0..food_count {
            food.push(Food {
                x: rng.gen_range(0.0..config.world_width),
                y: rng.gen_range(0.0..config.world_height),
                energy: config.food_energy,
            });
        }

        let spatial_hash = SpatialHash::new(
            config.world_width,
            config.world_height,
            cell_size,
            config.initial_population as usize * 2,
        );
        let food_spatial = FoodGrid::new(config.world_width, config.world_height, cell_size);
        let render_buffer = RenderBuffer::new(config.initial_population as usize * 2);
        let species_tracker = SpeciesTracker::new(&config);

        Ok(Self {
            config,
            creatures,
            food,
            tick: 0,
            generation_max: 0,
            total_births: 0,
            total_deaths: 0,
            nan_deaths: 0,
            next_species_id: 1,
            rng,
            spatial_hash,
            food_spatial,
            render_buffer,
            species_tracker,
            last_profile: TickProfile::default(),
            catastrophe_ticks_remaining: 0,
            energy_drift_pct: 0.0,
        })
    }

    pub fn step(&mut self) {
        let t_total = Timer::start();

        // --- Spatial hash rebuild (for food lookup during movement) ---
        let t_phase = Timer::start();
        self.spatial_hash.rebuild(
            self.creatures
                .iter()
                .map(|(key, c)| (key, c.x, c.y)),
        );
        self.food_spatial.rebuild(&self.food);
        let spatial_hash_us = t_phase.elapsed_us();

        // --- Physics: neural network driven movement ---
        let t_phase = Timer::start();
        let keys: Vec<CreatureKey> = self.creatures.keys().collect();

        // Pre-collect creature data for neighbor queries (avoid borrow conflict)
        let creature_data: Vec<(CreatureKey, f32, f32, f32, f32, f32)> = keys.iter()
            .filter_map(|k| self.creatures.get(*k).map(|c| (*k, c.x, c.y, c.rotation, c.size_trait, c.speed_trait)))
            .collect();

        for key in &keys {
            let creature = &self.creatures[*key];
            let cx = creature.x;
            let cy = creature.y;
            let crot = creature.rotation;
            let vision_range = creature.vision_range;
            let vision_sq = vision_range * vision_range;
            let half_cone = self.config.vision_cone_angle * 0.5;
            let fwd_x = crot.cos();
            let fwd_y = crot.sin();
            let ww = self.config.world_width;
            let wh = self.config.world_height;

            // --- Gather NN inputs ---
            let mut inputs = [0.0f32; NUM_INPUTS];

            // 0-1: nearest food distance (normalized) and relative angle
            let mut best_food_dist_sq = f32::MAX;
            let mut best_food_dx = 0.0f32;
            let mut best_food_dy = 0.0f32;
            let mut food_left = 0u32;
            let mut food_right = 0u32;

            if let Some(fi) = self.food_spatial.query_nearest(cx, cy, vision_sq, &self.food, ww, wh) {
                let f = &self.food[fi];
                let (dx, dy) = toroidal_delta(cx, cy, f.x, f.y, ww, wh);
                let dist_sq = dx * dx + dy * dy;
                if in_vision_cone(dx, dy, fwd_x, fwd_y, half_cone) {
                    best_food_dist_sq = dist_sq;
                    best_food_dx = dx;
                    best_food_dy = dy;
                }
            }

            // Count food in left/right vision sectors (sample from food grid)
            // Use a broader search for sector counting
            let food_check_range_sq = vision_sq;
            let fcx = (cx * self.food_spatial.inv_cell_size) as i32;
            let fcy = (cy * self.food_spatial.inv_cell_size) as i32;
            let gw = self.food_spatial.grid_width as i32;
            let gh = (self.food_spatial.cells.len() / self.food_spatial.grid_width.max(1)) as i32;
            for dy_cell in -2..=2 {
                for dx_cell in -2..=2 {
                    let nx = ((fcx + dx_cell) % gw + gw) % gw;
                    let ny = ((fcy + dy_cell) % gh + gh) % gh;
                    let cell_idx = (ny * gw + nx) as usize;
                    if cell_idx >= self.food_spatial.cells.len() { continue; }
                    for &fi in &self.food_spatial.cells[cell_idx] {
                        let f = &self.food[fi];
                        let (dx, dy) = toroidal_delta(cx, cy, f.x, f.y, ww, wh);
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq > food_check_range_sq { continue; }
                        if !in_vision_cone(dx, dy, fwd_x, fwd_y, half_cone) { continue; }
                        // Cross product: fwd × delta → positive = left, negative = right
                        let cross = fwd_x * dy - fwd_y * dx;
                        if cross > 0.0 { food_left += 1; } else { food_right += 1; }
                    }
                }
            }

            if best_food_dist_sq < f32::MAX {
                inputs[0] = 1.0 - (best_food_dist_sq.sqrt() / vision_range).min(1.0);
                let rel_angle = relative_angle(best_food_dx, best_food_dy, fwd_x, fwd_y);
                inputs[1] = rel_angle / std::f32::consts::PI; // normalize to [-1, 1]
            }
            inputs[2] = (food_left as f32 / 10.0).min(1.0);
            inputs[3] = (food_right as f32 / 10.0).min(1.0);

            // 4-7: nearest creature in vision cone
            let mut best_creature_dist_sq = f32::MAX;
            let mut best_creature_dx = 0.0f32;
            let mut best_creature_dy = 0.0f32;
            let mut best_creature_size = 0.0f32;
            let mut best_creature_speed = 0.0f32;

            for &(nk, nx, ny, _, nsize, nspeed) in &creature_data {
                if nk == *key { continue; }
                let (dx, dy) = toroidal_delta(cx, cy, nx, ny, ww, wh);
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > vision_sq { continue; }
                if !in_vision_cone(dx, dy, fwd_x, fwd_y, half_cone) { continue; }
                if dist_sq < best_creature_dist_sq {
                    best_creature_dist_sq = dist_sq;
                    best_creature_dx = dx;
                    best_creature_dy = dy;
                    best_creature_size = nsize;
                    best_creature_speed = nspeed;
                }
            }

            if best_creature_dist_sq < f32::MAX {
                inputs[4] = 1.0 - (best_creature_dist_sq.sqrt() / vision_range).min(1.0);
                let rel_angle = relative_angle(best_creature_dx, best_creature_dy, fwd_x, fwd_y);
                inputs[5] = rel_angle / std::f32::consts::PI;
                inputs[6] = (best_creature_size / creature.size_trait - 1.0).clamp(-1.0, 1.0);
                inputs[7] = (best_creature_speed / self.config.max_speed * 2.0 - 1.0).clamp(-1.0, 1.0);
            }

            // 8-10: own state
            inputs[8] = (creature.energy / self.config.max_energy) * 2.0 - 1.0;
            inputs[9] = (physics::speed_squared(creature).sqrt() / self.config.max_speed) * 2.0 - 1.0;
            inputs[10] = (creature.size_trait / self.config.max_size) * 2.0 - 1.0;

            // 11: random noise for exploration
            inputs[11] = self.rng.gen_range(-1.0..1.0);

            // --- Forward pass ---
            let output = creature.brain.forward(&inputs);
            let turn = output.turn * self.config.max_turn_rate;
            let thrust = output.thrust;

            let creature = self.creatures.get_mut(*key).unwrap();
            physics::apply_random_movement(creature, thrust, turn, &self.config);
        }
        let physics_us = t_phase.elapsed_us();

        // --- Energy: basal cost + movement cost ---
        let t_phase = Timer::start();
        for key in &keys {
            let creature = self.creatures.get_mut(*key).unwrap();
            let speed_sq = physics::speed_squared(creature);
            let cost = creature.basal_metabolic_cost(&self.config)
                + creature.movement_cost(speed_sq, &self.config);
            creature.energy -= cost;
            creature.energy = creature.energy.max(0.0);
            creature.age += 1;
            creature.ticks_since_reproduction += 1;
        }
        let energy_us = t_phase.elapsed_us();

        // --- Food consumption: each creature eats the nearest food within eat radius ---
        let t_phase = Timer::start();
        // Rebuild food grid after movement for accurate proximity checks
        self.food_spatial.rebuild(&self.food);
        let mut eaten_food: Vec<usize> = Vec::new();
        for key in &keys {
            if !self.creatures.contains_key(*key) { continue; }
            let creature = &self.creatures[*key];
            let eat_radius = creature.size_trait + 8.0;
            let eat_radius_sq = eat_radius * eat_radius;

            if let Some(food_idx) = self.food_spatial.query_nearest(
                creature.x,
                creature.y,
                eat_radius_sq,
                &self.food,
                self.config.world_width,
                self.config.world_height,
            ) {
                if !eaten_food.contains(&food_idx) {
                    let food_energy = self.food[food_idx].energy;
                    let creature = self.creatures.get_mut(*key).unwrap();
                    creature.energy =
                        (creature.energy + food_energy).min(self.config.max_energy);
                    eaten_food.push(food_idx);
                }
            }
        }
        // Remove eaten food (reverse order to preserve indices)
        eaten_food.sort_unstable();
        eaten_food.dedup();
        for &idx in eaten_food.iter().rev() {
            self.food.swap_remove(idx);
        }
        let food_us = t_phase.elapsed_us();

        // --- Reproduction ---
        let t_phase = Timer::start();
        let population = self.creatures.len();
        let can_reproduce = population < POPULATION_BLOCK_REPRO;
        let boosted_cost = population >= POPULATION_SOFT_CAP;

        let mut new_creatures: Vec<Creature> = Vec::new();
        for key in &keys {
            let creature = &self.creatures[*key];
            if can_reproduce && creature.can_reproduce(&self.config) && population + new_creatures.len() < MAX_CREATURES {
                let mut offspring = Creature::new_offspring(creature, &mut self.rng, &self.config);
                // Assign species via compatibility check
                let parent_species = creature.species_id;
                offspring.species_id = self.species_tracker.assign_species(
                    &offspring, parent_species, &mut self.rng,
                );
                if offspring.generation > self.generation_max {
                    self.generation_max = offspring.generation;
                }
                new_creatures.push(offspring);
                let parent = self.creatures.get_mut(*key).unwrap();
                parent.energy *= 1.0 - self.config.reproduction_energy_share;
                parent.ticks_since_reproduction = 0;
                parent.children_count += 1;
            }

            // Apply boosted metabolic cost at soft cap
            if boosted_cost {
                let creature = self.creatures.get_mut(*key).unwrap();
                creature.energy -= self.config.basal_cost * 0.5;
            }
        }
        for offspring in new_creatures {
            self.creatures.insert(offspring);
            self.total_births += 1;
        }
        let reproduction_us = t_phase.elapsed_us();

        // --- Cleanup: deaths + NaN guard ---
        let t_phase = Timer::start();
        let mut nan_count = 0u32;
        for key in &keys {
            if let Some(creature) = self.creatures.get(*key) {
                if creature.has_nan() {
                    self.creatures.remove(*key);
                    nan_count += 1;
                    self.total_deaths += 1;
                } else if creature.is_dead(&self.config) {
                    self.creatures.remove(*key);
                    self.total_deaths += 1;
                }
            }
        }
        self.nan_deaths += nan_count;
        let cleanup_us = t_phase.elapsed_us();

        // --- Food spawning (closed energy budget) ---
        let creature_energy: f32 = self.creatures.values().map(|c| c.energy).sum();
        let food_energy: f32 = self.food.iter().map(|f| f.energy).sum();
        let deficit = self.config.target_total_energy - creature_energy - food_energy;

        // Seasonal modifier
        let season_mod = 1.0
            + self.config.season_amplitude
                * (self.tick as f32 * std::f32::consts::TAU / self.config.season_period as f32)
                    .sin();

        // Catastrophe modifier
        let catastrophe_mod = if self.catastrophe_ticks_remaining > 0 {
            self.catastrophe_ticks_remaining -= 1;
            self.config.catastrophe_food_multiplier
        } else {
            1.0
        };

        // Auto-trigger catastrophe
        if self.config.auto_catastrophe_interval > 0
            && self.tick > 0
            && self.tick % self.config.auto_catastrophe_interval as u64 == 0
        {
            self.catastrophe_ticks_remaining = self.config.catastrophe_duration;
        }

        let effective_deficit = deficit * season_mod * catastrophe_mod;
        if effective_deficit > 0.0 {
            let food_to_spawn =
                (effective_deficit / self.config.food_energy).max(0.0) as usize;
            // Cap spawning per tick to prevent lag spikes
            let spawn_count = food_to_spawn.min(50);
            for _ in 0..spawn_count {
                self.food.push(Food {
                    x: self.rng.gen_range(0.0..self.config.world_width),
                    y: self.rng.gen_range(0.0..self.config.world_height),
                    energy: self.config.food_energy,
                });
            }
        }

        // Energy audit every 100 ticks
        if self.tick % 100 == 0 {
            let actual_energy: f32 = self.creatures.values().map(|c| c.energy).sum::<f32>()
                + self.food.iter().map(|f| f.energy).sum::<f32>();
            self.energy_drift_pct = ((actual_energy - self.config.target_total_energy).abs()
                / self.config.target_total_energy)
                * 100.0;
        }

        // --- Species tracking ---
        self.species_tracker.update_counts(
            self.creatures.values().map(|c| (
                c.species_id,
                c.speed_trait,
                c.size_trait,
                c.vision_range,
                c.brain.ih_weights.clone(),
                c.brain.ho_weights.clone(),
            ))
        );
        self.species_tracker.adjust_threshold(&self.config);

        // --- Pack render buffer ---
        let t_render = Timer::start();
        self.render_buffer.pack(
            &self.creatures,
            &self.species_tracker,
            self.config.max_energy,
            self.config.max_lifespan,
        );
        let render_pack_us = t_render.elapsed_us();

        self.tick += 1;

        // --- Update profile ---
        self.last_profile = TickProfile {
            spatial_hash_us,
            physics_us,
            energy_us,
            reproduction_us,
            food_us,
            cleanup_us,
            render_pack_us,
            total_us: t_total.elapsed_us(),
            creature_count: self.creatures.len() as u32,
            food_count: self.food.len() as u32,
        };
    }

    pub fn render_data_ptr(&self) -> *const f32 {
        self.render_buffer.as_ptr()
    }

    pub fn render_data_len(&self) -> usize {
        self.render_buffer.len()
    }

    pub fn creature_count(&self) -> usize {
        self.creatures.len()
    }

    pub fn food_count(&self) -> usize {
        self.food.len()
    }

    pub fn profile(&self) -> &TickProfile {
        &self.last_profile
    }

    pub fn energy_drift_pct(&self) -> f32 {
        self.energy_drift_pct
    }

    pub fn species_count(&self) -> usize {
        self.species_tracker.species_count()
    }

    pub fn species_json(&self) -> String {
        serde_json::to_string(&self.species_tracker.species).unwrap_or_default()
    }

    /// Pack food positions into a flat f32 buffer: [x, y, energy, ...]
    pub fn food_render_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity(self.food.len() * 3);
        for f in &self.food {
            data.push(f.x);
            data.push(f.y);
            data.push(f.energy);
        }
        data
    }
}

/// Compute toroidal delta from (ax, ay) to (bx, by)
#[inline(always)]
fn toroidal_delta(ax: f32, ay: f32, bx: f32, by: f32, ww: f32, wh: f32) -> (f32, f32) {
    let mut dx = bx - ax;
    let mut dy = by - ay;
    if dx.abs() > ww * 0.5 {
        dx -= dx.signum() * ww;
    }
    if dy.abs() > wh * 0.5 {
        dy -= dy.signum() * wh;
    }
    (dx, dy)
}

/// Check if a direction (dx, dy) falls within a vision cone defined by forward vector and half-angle
#[inline(always)]
fn in_vision_cone(dx: f32, dy: f32, fwd_x: f32, fwd_y: f32, half_cone: f32) -> bool {
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 { return true; } // very close = always visible
    let dot = dx * fwd_x + dy * fwd_y;
    // cos(angle) = dot / (|fwd| * |delta|), |fwd| = 1
    // angle < half_cone when cos(angle) > cos(half_cone)
    dot > half_cone.cos() * len_sq.sqrt()
}

/// Compute signed relative angle of (dx, dy) relative to forward direction
#[inline(always)]
fn relative_angle(dx: f32, dy: f32, fwd_x: f32, fwd_y: f32) -> f32 {
    let dot = dx * fwd_x + dy * fwd_y;
    let cross = fwd_x * dy - fwd_y * dx;
    cross.atan2(dot)
}
