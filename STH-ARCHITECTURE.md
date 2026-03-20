# STH Infection & Deworming Simulation -- Architecture Document

**Version:** 1.0
**Date:** 2026-03-15
**Author:** Staff Engineer Architect
**Status:** Draft -- awaiting review before implementation

---

## Table of Contents

1. [Problem Statement](#1-problem-statement)
2. [Assumptions & Unknowns](#2-assumptions--unknowns)
3. [Architecture Options](#3-architecture-options)
4. [Chosen Architecture](#4-chosen-architecture)
5. [Rust Module Design](#5-rust-module-design)
6. [Infection Transmission Model](#6-infection-transmission-model)
7. [Agent Behavior Model](#7-agent-behavior-model)
8. [Environment System](#8-environment-system)
9. [Intervention System](#9-intervention-system)
10. [Frontend Component Hierarchy](#10-frontend-component-hierarchy)
11. [Data Transfer Protocol](#11-data-transfer-protocol)
12. [State Management](#12-state-management)
13. [Rendering Pipeline](#13-rendering-pipeline)
14. [File-by-File Migration Map](#14-file-by-file-migration-map)
15. [Failure Modes](#15-failure-modes)
16. [Security & Performance](#16-security--performance)
17. [Testing & Observability](#17-testing--observability)
18. [Milestone Task Breakdown](#18-milestone-task-breakdown)
19. [Open Questions](#19-open-questions)

---

## 1. Problem Statement

Refactor the Darwin's Sandbox evolution simulator into an agent-based STH (Soil-Transmitted Helminth) infection and deworming simulation that models findings from a medical thesis on school-aged children in Cebu City, Philippines. The simulation must:

- Model individual agents (children, parents, teachers, health workers) moving through daily routines in urban and rural barangays.
- Simulate STH transmission dynamics (Ascaris, Trichuris, hookworm) through environmental contamination and fecal-oral routes.
- Encode the thesis finding that structural/environmental factors dominate over KAP (Knowledge, Attitudes, Practices) in determining infection outcomes.
- Allow users to deploy interventions (MDA, WASH, education, BHW visits) and observe their effects over simulated months/years.
- Support urban vs. rural side-by-side comparison reflecting the counterintuitive finding that urban informal settlements have higher prevalence.
- Retain the existing tech stack: Rust/WASM core, Next.js frontend, WebGL2+Canvas2D rendering, Web Worker bridge, Zustand state.

---

## 2. Assumptions & Unknowns

### Assumptions

1. **Single-page app** -- no server-side simulation. All computation in WASM via Web Worker.
2. **Agent count**: 200-500 children per barangay, plus ~100 adult agents (parents/teachers/BHWs). Two barangays max in comparison mode = ~1200 agents total. Well within the 5000-creature budget the existing renderer handles.
3. **Time scale**: 1 tick = 1 hour of simulated time. 24 ticks = 1 day. Users simulate 1-3 years (8,760-26,280 ticks). At 60 ticks/frame with 14ms budget, a 1-year simulation completes in ~2.5 minutes of wall-clock time.
4. **Deterministic**: Same seed produces identical runs. We keep SmallRng and polynomial approximations.
5. **No real geospatial data**: Barangay maps are procedurally generated from parameters (population density, number of schools, water sources, etc.), not GIS data.
6. **Thesis data as calibration targets**: The simulation should reproduce the thesis's aggregate findings (11.7% overall prevalence, 20.3% urban vs 4.9% rural) when default parameters are used.
7. **WASM binary budget**: Target <300KB (current is ~205KB; adding infection/environment logic should stay under budget).

### Unknowns

1. **Exact EPG-to-intensity mapping thresholds** -- use WHO standard cutoffs (Ascaris: light <5000, moderate 5000-49999, heavy >=50000; Trichuris: light <1000, moderate 1000-9999, heavy >=10000; Hookworm: light <2000, moderate 2000-3999, heavy >=4000).
2. **Drug efficacy curves** -- albendazole cure rates vary by species. Use published meta-analysis values: Ascaris ~95%, Trichuris ~30-50%, Hookworm ~72%.
3. **Exact reinfection rate functions** -- calibrate against thesis finding of 6-12 month return to baseline.
4. **Whether comparison mode requires two independent WASM instances or one simulation with two sub-worlds** -- decided below.

---

## 3. Architecture Options

### Option A: Deep Refactor -- Replace All Domain Types In-Place

Replace `Creature` with `Agent`, `Brain` with `BehaviorModel`, `Food` with `ContaminationSource`, etc. Rewrite every Rust file. The frontend also gets fully rewritten.

**Pros**: Clean domain model, no legacy cruft.
**Cons**: Massive blast radius. Every file changes simultaneously. No incremental testability. Weeks of broken builds.

### Option B: Parallel Domain -- New Modules Alongside Existing

Add new Rust modules (`agent.rs`, `infection.rs`, `environment.rs`, `intervention.rs`, `kap.rs`, `schedule.rs`) while keeping existing files intact. A new `SthWorld` struct replaces `World` as the simulation entry point. The old `Simulation` WASM API is replaced by `SthSimulation`. Frontend components get new STH-specific counterparts.

**Pros**: Incremental. Can keep the old simulation running during development. Each new module is independently testable. Git history stays meaningful per-file.
**Cons**: Temporary code duplication. Must eventually delete the old modules.

### Option C: Adapter Layer -- Wrap Existing Simulation

Keep the existing simulation engine and treat agents as "creatures" with STH-specific metadata bolted on. Encode infection state in the existing `energy` field, use `species_id` for agent type, etc.

**Pros**: Minimal Rust changes. Fastest to prototype.
**Cons**: Domain model is a leaky abstraction. Every concept gets shoehorned. Impossible to model daily schedules, multi-location movement, or intervention logistics. The existing neural network brain and food-seeking physics are irrelevant to STH behavior. Technical debt from day one.

---

## 4. Chosen Architecture

**Option B: Parallel Domain -- New Modules Alongside Existing.**

### Justification

- The domain shift is too large for an adapter (Option C) -- creatures seeking food has no meaningful mapping to children attending school and contracting helminth infections.
- A parallel approach (Option B) avoids the big-bang risk of Option A while achieving the same clean domain model.
- The existing infrastructure (WASM build, Worker bridge, WebGL renderer, Zustand store, double-buffer protocol) remains reusable. Only the simulation core and domain-specific UI change.
- Incremental delivery: we can run the old simulation alongside the new one during development, milestone by milestone.

### High-Level Component Diagram

```
+-----------------------------------------------------+
|                    Next.js Frontend                   |
|  +----------+  +---------+  +----------+  +--------+ |
|  | Barangay |  | Interv. |  | Dashboard|  | Agent  | |
|  | MapView  |  | Toolbar |  | Charts   |  |Inspector||
|  +----+-----+  +----+----+  +----+-----+  +---+----+ |
|       |              |            |             |      |
|       +-----------+--+------+-----+-------------+     |
|                   |  Zustand Store (sth-store)  |     |
|                   +-------------+---------------+     |
|                                 |                     |
|                   +-------------+---------------+     |
|                   |  Web Worker (sth.worker.ts) |     |
|                   +-------------+---------------+     |
+----------------------------|---------------------------+
                             | ArrayBuffer transfer
+----------------------------|---------------------------+
|                    Rust/WASM Core                      |
|  +--------+  +-----------+  +-------------+           |
|  | Agent  |  | Infection |  | Environment |           |
|  | System |  | Engine    |  | Grid        |           |
|  +--------+  +-----------+  +-------------+           |
|  +---------+  +-------------+  +-----------+          |
|  |  KAP    |  | Intervention|  | Schedule  |          |
|  |  Model  |  | System     |  | (DayCycle) |          |
|  +---------+  +-------------+  +-----------+          |
|  +------------+  +---------------+                    |
|  | SthWorld   |  | RenderBuffer  |                    |
|  | (step())   |  | (pack agents) |                    |
|  +------------+  +---------------+                    |
+-------------------------------------------------------+
```

---

## 5. Rust Module Design

### 5.1 Module Map

```
crates/simulation/src/
  lib.rs              -- SthSimulation wasm_bindgen API
  sth_world.rs        -- SthWorld: main simulation state + step()
  agent.rs            -- Agent struct, AgentKey, AgentType enum
  infection.rs        -- InfectionState, SthSpecies, transmission math
  kap.rs              -- KAP scores, HBM decision model
  environment.rs      -- EnvironmentGrid, ContaminationCell, facilities
  schedule.rs         -- DayCycle, TimeOfDay, location routing
  intervention.rs     -- MDA, WASH, Education, BHW campaigns
  barangay.rs         -- Barangay config, SettingType (urban/rural)
  event_tracker.rs    -- Domain events (outbreak, MDA complete, etc.)
  sth_config.rs       -- SthConfig replacing SimConfig
  sth_render_buffer.rs -- New render buffer layout
  spatial_hash.rs     -- REUSE existing (unchanged)
  profile.rs          -- REUSE existing (unchanged)
```

### 5.2 `agent.rs` -- Agent Struct

```rust
use slotmap::new_key_type;

new_key_type! { pub struct AgentKey; }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    Child,       // 6-12 years, primary simulation subject
    Parent,      // influences child KAP, household WASH
    Teacher,     // school-based education delivery
    HealthWorker,// BHW: house-to-house visits, MDA delivery
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    Home(u16),          // household_id
    School(u8),         // school_id
    Community,          // public area (market, playground, creek)
    HealthCenter,
    WaterSource(u8),    // water_source_id
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    // Identity
    pub agent_type: AgentType,
    pub age_years: u8,          // 6-12 for children
    pub household_id: u16,
    pub school_id: u8,

    // Position (for rendering)
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub current_location: Location,

    // Infection state (per STH species)
    pub ascaris: InfectionState,
    pub trichuris: InfectionState,
    pub hookworm: InfectionState,

    // KAP scores (0.0 - 1.0)
    pub knowledge: f32,
    pub attitude: f32,   // 0.0 = negative, 1.0 = positive
    pub practice: f32,   // 0.0 = high-risk, 1.0 = low-risk

    // Behavioral state
    pub wears_shoes: bool,
    pub washes_hands_before_eating: bool,
    pub uses_latrine: bool,
    pub days_since_last_deworming: u16,

    // SEM layer: which factors influence this agent
    pub household_has_latrine: bool,
    pub household_has_water: bool,
    pub school_has_wash_facility: bool,
}
```

### 5.3 `infection.rs` -- Infection Engine

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SthSpecies {
    Ascaris,
    Trichuris,
    Hookworm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intensity {
    Negative,
    Light,
    Moderate,
    Heavy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfectionState {
    pub epg: f32,        // eggs per gram (continuous for smooth simulation)
    pub worm_burden: f32, // estimated adult worm count
    pub days_infected: u16,
}

impl InfectionState {
    pub fn intensity(&self, species: SthSpecies) -> Intensity {
        match species {
            SthSpecies::Ascaris => match self.epg as u32 {
                0 => Intensity::Negative,
                1..=4999 => Intensity::Light,
                5000..=49999 => Intensity::Moderate,
                _ => Intensity::Heavy,
            },
            SthSpecies::Trichuris => match self.epg as u32 {
                0 => Intensity::Negative,
                1..=999 => Intensity::Light,
                1000..=9999 => Intensity::Moderate,
                _ => Intensity::Heavy,
            },
            SthSpecies::Hookworm => match self.epg as u32 {
                0 => Intensity::Negative,
                1..=1999 => Intensity::Light,
                2000..=3999 => Intensity::Moderate,
                _ => Intensity::Heavy,
            },
        }
    }

    pub fn is_infected(&self) -> bool {
        self.epg > 0.0
    }
}

/// Transmission probability per hour of exposure to contaminated environment.
/// Core formula: P(infection) = 1 - exp(-lambda * contamination * exposure_factor)
///
/// Where:
///   lambda = species-specific base transmission rate
///   contamination = local soil contamination level (0.0-1.0)
///   exposure_factor = product of behavioral risk factors
pub fn transmission_probability(
    species: SthSpecies,
    soil_contamination: f32,
    wears_shoes: bool,
    washes_hands: bool,
    uses_latrine: bool,  // whether agent used latrine (reduces soil contact)
    location_risk: f32,   // location-specific modifier (creek=high, school=medium, home=low)
) -> f32 {
    let lambda = match species {
        SthSpecies::Ascaris => 0.003,   // highest transmission (fecal-oral, soil, food)
        SthSpecies::Trichuris => 0.002, // similar to Ascaris but lower
        SthSpecies::Hookworm => 0.001,  // requires skin contact with soil
    };

    // Behavioral exposure reduction
    let shoe_factor = if wears_shoes { 0.3 } else { 1.0 }; // shoes mainly affect hookworm
    let hand_factor = if washes_hands { 0.4 } else { 1.0 }; // affects Ascaris/Trichuris
    let latrine_factor = if uses_latrine { 0.5 } else { 1.0 };

    // Species-specific behavioral weighting
    let exposure = match species {
        SthSpecies::Ascaris => hand_factor * latrine_factor * location_risk,
        SthSpecies::Trichuris => hand_factor * latrine_factor * location_risk,
        SthSpecies::Hookworm => shoe_factor * latrine_factor * location_risk,
    };

    // Core transmission formula
    let rate = lambda * soil_contamination * exposure;
    1.0 - (-rate).exp()  // NOTE: exp() is transcendental -- use poly approx in WASM
}

/// Worm burden dynamics: worms grow, produce eggs, die naturally.
/// Models reinfection cycle: burden increases with continued exposure,
/// decreases after deworming but returns to baseline in 6-12 months.
pub fn update_worm_burden(
    state: &mut InfectionState,
    new_infection_probability: f32,
    rng: &mut impl Rng,
    species: SthSpecies,
) {
    // Natural worm death rate (per day)
    let worm_death_rate = match species {
        SthSpecies::Ascaris => 1.0 / 365.0,    // ~1 year lifespan
        SthSpecies::Trichuris => 1.0 / 730.0,  // ~2 year lifespan
        SthSpecies::Hookworm => 1.0 / 1095.0,  // ~3 year lifespan
    };

    // Natural worm attrition (per hour = per tick)
    state.worm_burden *= 1.0 - (worm_death_rate / 24.0);

    // New infection acquisition
    if rng.gen::<f32>() < new_infection_probability {
        let new_worms = match species {
            SthSpecies::Ascaris => rng.gen_range(1.0..5.0),
            SthSpecies::Trichuris => rng.gen_range(1.0..3.0),
            SthSpecies::Hookworm => rng.gen_range(0.5..2.0),
        };
        state.worm_burden += new_worms;
    }

    // EPG is proportional to worm burden (fecundity per worm)
    let fecundity = match species {
        SthSpecies::Ascaris => 200_000.0 / 20.0, // ~200k eggs/day / ~20 worms avg
        SthSpecies::Trichuris => 10_000.0 / 10.0,
        SthSpecies::Hookworm => 10_000.0 / 10.0,
    };
    state.epg = state.worm_burden * fecundity;

    if state.worm_burden < 0.01 {
        state.worm_burden = 0.0;
        state.epg = 0.0;
        state.days_infected = 0;
    } else {
        state.days_infected = state.days_infected.saturating_add(1);
    }
}
```

**IMPORTANT: exp() approximation.** The existing codebase uses `poly_tanh` to avoid transcendental functions. For `exp(-x)` where x is small and positive, use the Pade approximant:

```rust
/// Polynomial exp(-x) approximation for x in [0, 5].
/// Pade(2,2): (1 - x/2 + x^2/12) / (1 + x/2 + x^2/12)
/// Max error < 0.3% in [0, 3], good enough for probability calculations.
#[inline(always)]
fn poly_exp_neg(x: f32) -> f32 {
    let x = x.clamp(0.0, 5.0);
    let x2 = x * x;
    let num = 1.0 - x * 0.5 + x2 / 12.0;
    let den = 1.0 + x * 0.5 + x2 / 12.0;
    (num / den).max(0.0)
}
```

### 5.4 `kap.rs` -- Knowledge, Attitudes, Practices Model

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KapScores {
    pub knowledge: f32,  // 0.0-1.0 (maps to 0-100%)
    pub attitude: f32,   // 0.0-1.0 (maps to 1.0-5.0 scale)
    pub practice: f32,   // 0.0-1.0 (maps to 0-100%)
}

#[derive(Clone, Copy, Debug)]
pub enum KapLevel {
    Poor,      // 0-49%
    Moderate,  // 50-69%
    Good,      // 70-89%
    Excellent, // 90-100%
}

impl KapScores {
    pub fn knowledge_level(&self) -> KapLevel {
        match (self.knowledge * 100.0) as u32 {
            0..=49 => KapLevel::Poor,
            50..=69 => KapLevel::Moderate,
            70..=89 => KapLevel::Good,
            _ => KapLevel::Excellent,
        }
    }

    /// KEY THESIS FINDING: KAP modifies behavior probabilistically,
    /// but structural factors (latrine access, water quality) dominate outcomes.
    /// The practice_to_behavior() method converts KAP into behavioral probabilities,
    /// but the actual infection transmission weights structural factors 3x more.
    pub fn handwashing_probability(&self) -> f32 {
        // Base probability from practice score, attenuated by knowledge
        let base = self.practice * 0.6 + self.knowledge * 0.2 + self.attitude * 0.2;
        base.clamp(0.05, 0.95) // never 0% or 100%
    }

    pub fn shoe_wearing_probability(&self) -> f32 {
        let base = self.practice * 0.5 + self.attitude * 0.3 + self.knowledge * 0.2;
        base.clamp(0.05, 0.95)
    }

    pub fn latrine_use_probability(&self, has_latrine: bool) -> f32 {
        if !has_latrine { return 0.0; } // STRUCTURAL FACTOR: can't use what doesn't exist
        let base = self.practice * 0.5 + self.attitude * 0.3 + self.knowledge * 0.2;
        base.clamp(0.1, 0.95)
    }
}

/// Health Belief Model integration: modifies KAP over time based on
/// perceived susceptibility, severity, benefits, barriers.
pub fn update_kap_from_experience(
    kap: &mut KapScores,
    is_currently_infected: bool,
    received_education: bool,
    parent_kap: &KapScores,
    community_avg_kap: f32,
    rng: &mut impl Rng,
) {
    // Education events provide immediate knowledge boost
    if received_education {
        kap.knowledge = (kap.knowledge + rng.gen_range(0.05..0.15)).min(1.0);
        kap.attitude = (kap.attitude + rng.gen_range(0.02..0.08)).min(1.0);
    }

    // Personal infection experience increases perceived susceptibility
    if is_currently_infected {
        kap.attitude = (kap.attitude + 0.01).min(1.0); // slow attitude shift
    }

    // Social influence: drift toward community average (SEM interpersonal level)
    let community_pull = 0.001;
    kap.knowledge += (community_avg_kap - kap.knowledge) * community_pull;
    kap.attitude += (parent_kap.attitude - kap.attitude) * community_pull * 2.0;

    // Practice decays without reinforcement (thesis finding: education effects fade)
    kap.practice *= 0.9997; // ~10% decay per month (720 ticks)

    // Clamp
    kap.knowledge = kap.knowledge.clamp(0.0, 1.0);
    kap.attitude = kap.attitude.clamp(0.0, 1.0);
    kap.practice = kap.practice.clamp(0.0, 1.0);
}
```

### 5.5 `environment.rs` -- Environmental Grid

```rust
/// Each cell represents a ~10m x 10m area of the barangay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContaminationCell {
    pub soil_contamination: f32,  // 0.0-1.0, from open defecation
    pub water_contamination: f32, // 0.0-1.0, from runoff
    pub has_latrine: bool,
    pub has_water_source: bool,
    pub terrain: TerrainType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainType {
    Residential,
    School,
    HealthCenter,
    Creek,         // high contamination risk
    Market,
    Road,
    OpenField,     // open defecation risk
}

pub struct EnvironmentGrid {
    cells: Vec<ContaminationCell>,
    grid_width: usize,
    grid_height: usize,
    cell_size: f32,
}

impl EnvironmentGrid {
    /// Contamination dynamics per tick (1 hour):
    /// - Open defecation adds contamination to nearby cells
    /// - Rain events spread contamination (runoff to creek areas)
    /// - UV/desiccation slowly reduces contamination
    /// - Latrines prevent contamination deposit
    pub fn update_contamination(&mut self, config: &SthConfig) {
        for cell in &mut self.cells {
            // Natural decay: eggs die in soil over weeks-months
            // Ascaris eggs are extremely resilient (years), but
            // we model "infectious dose" decay, not absolute egg death
            let decay_rate = match cell.terrain {
                TerrainType::Creek => 0.0001,     // wet = slower decay
                TerrainType::OpenField => 0.0005,  // UV exposure
                _ => 0.0003,
            };
            cell.soil_contamination *= 1.0 - decay_rate;

            // Water contamination decays faster if water source is improved
            if cell.has_water_source {
                cell.water_contamination *= 0.999; // slow decay for improved
            } else {
                cell.water_contamination *= 0.9999; // very slow for unimproved
            }
        }
    }

    /// Agent defecating in open adds contamination
    pub fn deposit_contamination(&mut self, x: f32, y: f32, amount: f32) {
        let (cx, cy) = self.world_to_cell(x, y);
        if let Some(cell) = self.get_cell_mut(cx, cy) {
            cell.soil_contamination = (cell.soil_contamination + amount).min(1.0);
        }
        // Spread to neighbors
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if let Some(cell) = self.get_cell_mut(cx + dx, cy + dy) {
                cell.soil_contamination =
                    (cell.soil_contamination + amount * 0.3).min(1.0);
            }
        }
    }

    pub fn sample_contamination(&self, x: f32, y: f32) -> (f32, f32) {
        let (cx, cy) = self.world_to_cell(x, y);
        self.get_cell(cx, cy)
            .map(|c| (c.soil_contamination, c.water_contamination))
            .unwrap_or((0.0, 0.0))
    }
}
```

### 5.6 `schedule.rs` -- Daily Cycle

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeOfDay {
    EarlyMorning, // 5-7: wake, hygiene
    Morning,      // 7-12: school
    Lunch,        // 12-13: eating (risk: unwashed food)
    Afternoon,    // 13-17: school or community play
    Evening,      // 17-20: home, eating
    Night,        // 20-5: sleep (no movement)
}

impl TimeOfDay {
    pub fn from_tick(tick: u64) -> Self {
        let hour = (tick % 24) as u8;
        match hour {
            5..=6 => TimeOfDay::EarlyMorning,
            7..=11 => TimeOfDay::Morning,
            12 => TimeOfDay::Lunch,
            13..=16 => TimeOfDay::Afternoon,
            17..=19 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    pub fn day_of_simulation(tick: u64) -> u32 {
        (tick / 24) as u32
    }

    pub fn month_of_simulation(tick: u64) -> u32 {
        Self::day_of_simulation(tick) / 30
    }

    pub fn is_school_day(tick: u64) -> bool {
        let day_of_week = (tick / 24) % 7;
        day_of_week < 5 // Mon-Fri
    }
}

/// Determine where an agent should be at the current time.
pub fn target_location(
    agent: &Agent,
    time: TimeOfDay,
    is_school_day: bool,
) -> Location {
    match agent.agent_type {
        AgentType::Child => match time {
            TimeOfDay::Night | TimeOfDay::EarlyMorning | TimeOfDay::Evening =>
                Location::Home(agent.household_id),
            TimeOfDay::Morning | TimeOfDay::Lunch =>
                if is_school_day { Location::School(agent.school_id) }
                else { Location::Community },
            TimeOfDay::Afternoon =>
                if is_school_day { Location::School(agent.school_id) }
                else { Location::Community },
        },
        AgentType::Parent => match time {
            TimeOfDay::Night | TimeOfDay::EarlyMorning | TimeOfDay::Evening =>
                Location::Home(agent.household_id),
            _ => Location::Community, // market, work
        },
        AgentType::Teacher => match time {
            TimeOfDay::Morning | TimeOfDay::Lunch | TimeOfDay::Afternoon =>
                if is_school_day { Location::School(agent.school_id) }
                else { Location::Home(agent.household_id) },
            _ => Location::Home(agent.household_id),
        },
        AgentType::HealthWorker => match time {
            TimeOfDay::Morning | TimeOfDay::Afternoon =>
                Location::HealthCenter, // or on house visits
            _ => Location::Home(agent.household_id),
        },
    }
}
```

### 5.7 `intervention.rs` -- Intervention System

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterventionType {
    /// Mass Drug Administration: school-based biannual deworming
    MDA {
        target_school: Option<u8>,   // None = all schools
        drug: Drug,
        coverage_pct: f32,          // 0.0-1.0, depends on BHW availability
    },
    /// WASH infrastructure improvement
    WashImprovement {
        improvement: WashType,
        target_area: (f32, f32, f32), // x, y, radius
    },
    /// Health education campaign
    Education {
        method: EducationMethod,
        target_school: Option<u8>,
    },
    /// BHW house-to-house visits
    BhwVisits {
        coverage_pct: f32,
        duration_days: u16,
    },
    /// Policy: increase health budget (improves medicine supply, staffing)
    PolicyBudgetIncrease {
        multiplier: f32,  // 1.5 = 50% increase
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Drug {
    Albendazole,
    Mebendazole,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WashType {
    Latrine,
    WaterPump,
    HandwashStation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EducationMethod {
    Cartoon,        // visual learning
    BoardGame,      // interactive
    TeacherLed,     // classroom session
    ParentMeeting,  // community
}

#[derive(Clone, Debug)]
pub struct ActiveIntervention {
    pub intervention: InterventionType,
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub cost: f32,
    pub progress: f32, // 0.0-1.0
}

/// Drug efficacy: probability of cure (worm burden -> 0) per species.
/// From published meta-analyses.
pub fn drug_efficacy(drug: Drug, species: SthSpecies) -> f32 {
    match (drug, species) {
        (Drug::Albendazole, SthSpecies::Ascaris) => 0.95,
        (Drug::Albendazole, SthSpecies::Trichuris) => 0.30,
        (Drug::Albendazole, SthSpecies::Hookworm) => 0.72,
        (Drug::Mebendazole, SthSpecies::Ascaris) => 0.96,
        (Drug::Mebendazole, SthSpecies::Trichuris) => 0.42,
        (Drug::Mebendazole, SthSpecies::Hookworm) => 0.15,
    }
}

/// Apply MDA to a set of children. Returns number treated.
pub fn apply_mda(
    agents: &mut DenseSlotMap<AgentKey, Agent>,
    drug: Drug,
    target_school: Option<u8>,
    coverage_pct: f32,
    medicine_supply: f32,  // 0.0-1.0, affected by procurement delays
    rng: &mut impl Rng,
) -> u32 {
    let effective_coverage = coverage_pct * medicine_supply;
    let mut treated = 0u32;

    for agent in agents.values_mut() {
        if agent.agent_type != AgentType::Child { continue; }
        if let Some(school) = target_school {
            if agent.school_id != school { continue; }
        }
        if rng.gen::<f32>() > effective_coverage { continue; }

        // Apply drug effect per species
        for (species, state) in [
            (SthSpecies::Ascaris, &mut agent.ascaris),
            (SthSpecies::Trichuris, &mut agent.trichuris),
            (SthSpecies::Hookworm, &mut agent.hookworm),
        ] {
            if state.is_infected() && rng.gen::<f32>() < drug_efficacy(drug, species) {
                state.worm_burden = 0.0;
                state.epg = 0.0;
            }
        }
        agent.days_since_last_deworming = 0;
        treated += 1;
    }
    treated
}
```

### 5.8 `barangay.rs` -- Barangay Configuration

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingType {
    Urban,   // Guadalupe/Tisa: high density, informal settlements, poor WASH
    Rural,   // Sudlon II/Guba: lower density, agricultural, better WASH paradoxically
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarangayConfig {
    pub setting: SettingType,
    pub name: String,
    pub child_population: u16,
    pub num_households: u16,
    pub num_schools: u8,
    pub num_health_workers: u8,

    // WASH infrastructure (thesis findings encoded)
    pub latrine_coverage: f32,     // Urban: 0.6, Rural: 0.75
    pub water_supply_coverage: f32, // Urban: 0.5 (intermittent), Rural: 0.7
    pub handwash_station_coverage: f32,

    // Environmental factors
    pub population_density: f32,    // persons per cell
    pub open_defecation_rate: f32,  // Urban: 0.3, Rural: 0.15
    pub baseline_soil_contamination: f32, // Urban: 0.4, Rural: 0.15

    // Programmatic factors
    pub medicine_supply_reliability: f32, // 0.0-1.0 (stockout probability)
    pub bhw_per_thousand: f32,      // health workers per 1000 pop

    // Initial KAP distributions (thesis data)
    pub mean_knowledge: f32,    // Urban: 0.55, Rural: 0.70
    pub mean_attitude: f32,     // Urban: 0.65, Rural: 0.70
    pub mean_practice: f32,     // Urban: 0.50, Rural: 0.65
}

impl BarangayConfig {
    pub fn urban_default() -> Self {
        Self {
            setting: SettingType::Urban,
            name: "Guadalupe".to_string(),
            child_population: 300,
            num_households: 150,
            num_schools: 2,
            num_health_workers: 3,
            latrine_coverage: 0.60,
            water_supply_coverage: 0.50,
            handwash_station_coverage: 0.30,
            population_density: 5.0,
            open_defecation_rate: 0.30,
            baseline_soil_contamination: 0.40,
            medicine_supply_reliability: 0.70,
            bhw_per_thousand: 2.0,
            mean_knowledge: 0.55,
            mean_attitude: 0.65,
            mean_practice: 0.50,
        }
    }

    pub fn rural_default() -> Self {
        Self {
            setting: SettingType::Rural,
            name: "Sudlon II".to_string(),
            child_population: 200,
            num_households: 100,
            num_schools: 1,
            num_health_workers: 2,
            latrine_coverage: 0.75,
            water_supply_coverage: 0.70,
            handwash_station_coverage: 0.50,
            population_density: 1.5,
            open_defecation_rate: 0.15,
            baseline_soil_contamination: 0.15,
            medicine_supply_reliability: 0.80,
            bhw_per_thousand: 3.0,
            mean_knowledge: 0.70,
            mean_attitude: 0.70,
            mean_practice: 0.65,
        }
    }
}
```

### 5.9 `sth_config.rs` -- Simulation Configuration

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SthConfig {
    pub seed: u64,
    pub world_width: f32,       // pixels for rendering (800 for single, 1600 for comparison)
    pub world_height: f32,

    // Barangay configs
    pub barangays: Vec<BarangayConfig>, // 1 for single mode, 2 for comparison

    // Simulation speed
    pub ticks_per_day: u32,     // always 24 (1 tick = 1 hour)

    // Transmission parameters (tunable)
    pub ascaris_lambda: f32,
    pub trichuris_lambda: f32,
    pub hookworm_lambda: f32,

    // Contamination dynamics
    pub contamination_decay_rate: f32,
    pub open_defecation_contamination: f32,
    pub rain_frequency: f32,    // probability of rain event per day
    pub rain_contamination_spread: f32,

    // Intervention costs (budget currency units)
    pub mda_cost_per_child: f32,
    pub latrine_cost: f32,
    pub water_pump_cost: f32,
    pub handwash_station_cost: f32,
    pub education_session_cost: f32,
    pub bhw_visit_cost_per_household: f32,

    // Budget
    pub total_budget: f32,
    pub monthly_budget_increment: f32,

    // COVID disruption (optional scenario)
    pub covid_school_closure_start: Option<u32>, // month
    pub covid_school_closure_end: Option<u32>,
}
```

### 5.10 `sth_world.rs` -- Main World Step

```rust
pub struct SthWorld {
    pub config: SthConfig,
    pub agents: DenseSlotMap<AgentKey, Agent>,
    pub environment: EnvironmentGrid,
    pub active_interventions: Vec<ActiveIntervention>,
    pub tick: u64,
    pub budget_spent: f32,
    pub budget_remaining: f32,

    // Stats tracking
    pub total_mda_rounds: u32,
    pub total_children_treated: u32,
    pub total_latrines_built: u32,
    pub total_water_sources: u32,

    // Per-barangay stats (for comparison mode)
    pub barangay_stats: Vec<BarangayStats>,

    rng: SmallRng,
    spatial_hash: SpatialHash,
    render_buffer: SthRenderBuffer,
    event_tracker: SthEventTracker,
    last_profile: TickProfile,
}

pub struct BarangayStats {
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

impl SthWorld {
    pub fn step(&mut self) {
        let t_total = Timer::start();

        let time_of_day = TimeOfDay::from_tick(self.tick);
        let is_school_day = TimeOfDay::is_school_day(self.tick);
        let day = TimeOfDay::day_of_simulation(self.tick);
        let month = TimeOfDay::month_of_simulation(self.tick);

        // --- Phase 1: Schedule & Movement ---
        // Determine target locations, move agents toward them
        let keys: Vec<AgentKey> = self.agents.keys().collect();
        for &key in &keys {
            let agent = &self.agents[key];
            let target = schedule::target_location(agent, time_of_day, is_school_day);
            let (tx, ty) = self.location_to_position(target);
            let agent = self.agents.get_mut(key).unwrap();
            agent.current_location = target;
            // Smooth movement toward target (not instant teleportation)
            let dx = tx - agent.x;
            let dy = ty - agent.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 2.0 {
                let speed = 5.0; // pixels per tick
                agent.x += dx / dist * speed.min(dist);
                agent.y += dy / dist * speed.min(dist);
            }
            agent.target_x = tx;
            agent.target_y = ty;
        }

        // --- Phase 2: Behavioral Decisions (hourly) ---
        for &key in &keys {
            let agent = &mut self.agents[key];
            if agent.agent_type != AgentType::Child { continue; }

            // Decide hygiene behaviors based on KAP + structural access
            let kap = KapScores {
                knowledge: agent.knowledge,
                attitude: agent.attitude,
                practice: agent.practice,
            };
            agent.washes_hands_before_eating =
                self.rng.gen::<f32>() < kap.handwashing_probability()
                && agent.school_has_wash_facility; // STRUCTURAL GATE
            agent.wears_shoes =
                self.rng.gen::<f32>() < kap.shoe_wearing_probability();
            agent.uses_latrine =
                self.rng.gen::<f32>() < kap.latrine_use_probability(agent.household_has_latrine);
        }

        // --- Phase 3: Contamination Deposit ---
        // Agents who don't use latrines contaminate the environment
        for &key in &keys {
            let agent = &self.agents[key];
            if !agent.uses_latrine && time_of_day != TimeOfDay::Night {
                // Only infected agents deposit contaminated material
                let total_epg = agent.ascaris.epg + agent.trichuris.epg + agent.hookworm.epg;
                if total_epg > 0.0 {
                    let amount = (total_epg / 100_000.0).min(0.1); // normalize
                    self.environment.deposit_contamination(agent.x, agent.y, amount);
                }
            }
        }

        // --- Phase 4: Transmission ---
        for &key in &keys {
            let agent = &self.agents[key];
            if agent.agent_type != AgentType::Child { continue; }
            if time_of_day == TimeOfDay::Night { continue; } // no exposure while sleeping

            let (soil_c, _water_c) = self.environment.sample_contamination(agent.x, agent.y);
            let location_risk = match agent.current_location {
                Location::Community => 1.2,
                Location::School(_) => 0.8,
                Location::Home(_) => 0.6,
                Location::WaterSource(_) => 1.5,
                Location::HealthCenter => 0.3,
            };

            for (species, state) in [
                (SthSpecies::Ascaris, &agent.ascaris),
                (SthSpecies::Trichuris, &agent.trichuris),
                (SthSpecies::Hookworm, &agent.hookworm),
            ] {
                let prob = infection::transmission_probability(
                    species, soil_c,
                    agent.wears_shoes, agent.washes_hands_before_eating,
                    agent.uses_latrine, location_risk,
                );
                // Collect updates to apply after immutable borrow ends
                // (actual mutation in separate pass below)
            }
            // NOTE: Builder must restructure this to avoid borrow conflicts,
            // same pattern as existing creature physics pass.
        }
        // Apply infection updates (separate mutable pass)
        for &key in &keys {
            let agent = &mut self.agents[key];
            if agent.agent_type != AgentType::Child { continue; }
            if time_of_day == TimeOfDay::Night { continue; }

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
                let prob = infection::transmission_probability(
                    species, soil_c,
                    agent.wears_shoes, agent.washes_hands_before_eating,
                    agent.uses_latrine, location_risk,
                );
                infection::update_worm_burden(state, prob, &mut self.rng, species);
            }
        }

        // --- Phase 5: Environment Decay ---
        self.environment.update_contamination(&self.config);

        // --- Phase 6: Interventions ---
        self.process_interventions();

        // --- Phase 7: KAP Evolution (daily at midnight) ---
        if self.tick % 24 == 0 {
            self.update_kap_scores();
            self.update_barangay_stats();

            // Monthly budget increment
            if day % 30 == 0 && day > 0 {
                self.budget_remaining += self.config.monthly_budget_increment;
            }
        }

        // --- Phase 8: Rain events (probabilistic, daily check) ---
        if self.tick % 24 == 0 {
            if self.rng.gen::<f32>() < self.config.rain_frequency {
                self.environment.spread_contamination_rain(
                    self.config.rain_contamination_spread,
                );
            }
        }

        // --- Phase 9: Pack Render Buffer ---
        self.render_buffer.pack(&self.agents, &self.environment);

        self.tick += 1;
    }
}
```

### 5.11 `sth_render_buffer.rs` -- Render Data Layout

```rust
/// 16 floats per agent (expanded from 12 to carry infection/KAP data):
/// [x, y, target_x, target_y, agent_type, infection_status, epg_norm,
///  knowledge, attitude, practice, household_id, school_id,
///  r, g, b, barangay_id]
pub const FLOATS_PER_AGENT: usize = 16;

pub struct SthRenderBuffer {
    agent_data: Vec<f32>,
    env_data: Vec<f32>,    // contamination grid for heatmap overlay
    facility_data: Vec<f32>, // [x, y, type, ...]
}

impl SthRenderBuffer {
    pub fn pack(
        &mut self,
        agents: &DenseSlotMap<AgentKey, Agent>,
        environment: &EnvironmentGrid,
    ) {
        self.agent_data.clear();
        let needed = agents.len() * FLOATS_PER_AGENT;
        if self.agent_data.capacity() < needed {
            self.agent_data.reserve(needed - self.agent_data.capacity());
        }

        for agent in agents.values() {
            self.agent_data.push(agent.x);
            self.agent_data.push(agent.y);
            self.agent_data.push(agent.target_x);
            self.agent_data.push(agent.target_y);
            self.agent_data.push(agent.agent_type as u8 as f32);

            // Infection: 0=negative, 1=light, 2=moderate, 3=heavy
            // Take worst infection across all species
            let worst = [
                agent.ascaris.intensity(SthSpecies::Ascaris),
                agent.trichuris.intensity(SthSpecies::Trichuris),
                agent.hookworm.intensity(SthSpecies::Hookworm),
            ].iter().map(|i| match i {
                Intensity::Negative => 0,
                Intensity::Light => 1,
                Intensity::Moderate => 2,
                Intensity::Heavy => 3,
            }).max().unwrap_or(0);
            self.agent_data.push(worst as f32);

            // Normalized EPG (log scale for visual differentiation)
            let total_epg = agent.ascaris.epg + agent.trichuris.epg + agent.hookworm.epg;
            let epg_norm = (total_epg.max(1.0).log10() / 5.0).clamp(0.0, 1.0); // log10(100000)=5
            self.agent_data.push(epg_norm);

            self.agent_data.push(agent.knowledge);
            self.agent_data.push(agent.attitude);
            self.agent_data.push(agent.practice);
            self.agent_data.push(agent.household_id as f32);
            self.agent_data.push(agent.school_id as f32);

            // Color: derived from agent type
            let (r, g, b) = agent_color(agent);
            self.agent_data.push(r);
            self.agent_data.push(g);
            self.agent_data.push(b);

            self.agent_data.push(0.0); // barangay_id (0 or 1 for comparison mode)
        }

        // Pack environment contamination grid
        self.env_data = environment.contamination_grid_flat();

        // Pack facility positions
        self.facility_data = environment.facility_positions_flat();
    }
}

fn agent_color(agent: &Agent) -> (f32, f32, f32) {
    // Base color by type
    let base = match agent.agent_type {
        AgentType::Child => (0.3, 0.7, 0.9),       // blue
        AgentType::Parent => (0.5, 0.8, 0.5),      // green
        AgentType::Teacher => (0.9, 0.7, 0.2),     // gold
        AgentType::HealthWorker => (0.9, 0.3, 0.3),// red
    };
    // Tint toward red based on infection severity for children
    if agent.agent_type == AgentType::Child {
        let infection_max = [agent.ascaris.epg, agent.trichuris.epg, agent.hookworm.epg]
            .iter().cloned().fold(0.0f32, f32::max);
        let severity = (infection_max.max(1.0).log10() / 5.0).clamp(0.0, 1.0);
        (
            base.0 + (0.9 - base.0) * severity,
            base.1 * (1.0 - severity * 0.6),
            base.2 * (1.0 - severity * 0.6),
        )
    } else {
        base
    }
}
```

### 5.12 `lib.rs` -- WASM API

```rust
pub mod agent;
pub mod infection;
pub mod kap;
pub mod environment;
pub mod schedule;
pub mod intervention;
pub mod barangay;
pub mod sth_config;
pub mod sth_world;
pub mod sth_render_buffer;
pub mod event_tracker;  // new STH events
pub mod spatial_hash;   // reused
pub mod profile;        // reused

#[wasm_bindgen]
pub struct SthSimulation {
    world: SthWorld,
}

#[wasm_bindgen]
impl SthSimulation {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<SthSimulation, JsValue> { ... }

    pub fn with_seed(seed: u64) -> Result<SthSimulation, JsValue> { ... }

    pub fn with_config(config_json: &str) -> Result<SthSimulation, JsValue> { ... }

    pub fn step(&mut self) { self.world.step(); }

    // --- Render data ---
    pub fn get_agent_render_data(&self) -> Vec<f32> { ... }
    pub fn get_env_render_data(&self) -> Vec<f32> { ... }
    pub fn get_facility_render_data(&self) -> Vec<f32> { ... }
    pub fn get_agent_count(&self) -> usize { ... }

    // --- Stats ---
    pub fn get_tick(&self) -> u64 { ... }
    pub fn get_day(&self) -> u32 { ... }
    pub fn get_month(&self) -> u32 { ... }
    pub fn get_stats_json(&self) -> String { ... } // BarangayStats + global stats
    pub fn get_prevalence_json(&self) -> String { ... }

    // --- Interventions ---
    pub fn launch_mda(&mut self, school_id: i8, drug: u8) { ... }
    pub fn build_latrine(&mut self, x: f32, y: f32) { ... }
    pub fn build_water_pump(&mut self, x: f32, y: f32) { ... }
    pub fn build_handwash_station(&mut self, x: f32, y: f32) { ... }
    pub fn launch_education(&mut self, method: u8, school_id: i8) { ... }
    pub fn launch_bhw_visits(&mut self, coverage: f32) { ... }
    pub fn increase_budget(&mut self, multiplier: f32) { ... }

    // --- Agent inspection ---
    pub fn get_agent_detail_json(&self, index: usize) -> String { ... }

    // --- Events ---
    pub fn drain_events_json(&mut self) -> String { ... }

    // --- World dimensions ---
    pub fn get_world_width(&self) -> f32 { ... }
    pub fn get_world_height(&self) -> f32 { ... }

    // --- Environment grid ---
    pub fn get_env_grid_dims(&self) -> Vec<f32> { ... } // [w, h, cell_size]

    // --- Budget ---
    pub fn get_budget_remaining(&self) -> f32 { ... }
    pub fn get_budget_spent(&self) -> f32 { ... }

    pub fn get_seed(&self) -> u64 { ... }
}
```

---

## 6. Infection Transmission Model

### Mathematical Summary

The core transmission model is a **stochastic SIS (Susceptible-Infected-Susceptible)** model with continuous worm burden, operating at hourly time steps.

#### Acquisition

```
P(new_infection | tick) = 1 - exp(-lambda_s * C(x,y) * E_behavioral * L_location)
```

Where:
- `lambda_s` = species-specific base transmission rate (Ascaris: 0.003, Trichuris: 0.002, Hookworm: 0.001)
- `C(x,y)` = soil contamination at agent position (0.0-1.0)
- `E_behavioral` = product of behavioral exposure factors (shoes, handwashing, latrine use)
- `L_location` = location risk modifier (creek: 1.5, community: 1.2, school: 0.8, home: 0.6, health center: 0.3)

#### Worm Burden Dynamics

```
W(t+1) = W(t) * (1 - mu_s/24) + N_new(t)
EPG(t) = W(t) * F_s
```

Where:
- `mu_s` = natural worm death rate per day (Ascaris: 1/365, Trichuris: 1/730, Hookworm: 1/1095)
- `N_new(t)` = new worm acquisition (stochastic, drawn when infection event occurs)
- `F_s` = fecundity (EPG per worm) by species

#### Environmental Contamination

```
C(x,y,t+1) = C(x,y,t) * (1 - delta) + sum(D_agent) + R(t) * spread
```

Where:
- `delta` = natural decay rate (terrain-dependent)
- `D_agent` = contamination deposited by infected agents who defecate openly
- `R(t)` = rain event spreads contamination to adjacent cells

#### Key Model Properties

1. **Structural dominance**: Behavioral factors (KAP-driven) are multiplied by structural gates. A child with excellent handwashing knowledge but no handwash station gets no benefit. This encodes the thesis finding.
2. **Reinfection cycle**: After MDA clears worm burden, continued environmental contamination re-exposes children. Without WASH improvements, prevalence returns to baseline in ~6-12 months (calibrated via lambda values).
3. **Urban-rural disparity**: Higher population density in urban areas means more contamination deposit per cell, higher baseline contamination, and lower WASH coverage -- reproducing the thesis finding that urban prevalence (20.3%) exceeds rural (4.9%).

---

## 7. Agent Behavior Model

### Daily Cycle (replaces neural network brain)

Agents do not have neural networks. Instead, behavior is driven by:

1. **Schedule**: `TimeOfDay` determines location. Children go to school on weekdays, community on weekends.
2. **KAP-driven decisions**: Each behavioral decision (handwashing, shoe-wearing, latrine use) is a Bernoulli trial with probability derived from KAP scores.
3. **Structural gates**: Even high-KAP agents cannot use facilities that don't exist.
4. **Social influence**: KAP drifts toward community average (SEM interpersonal level).
5. **Education events**: Temporary KAP boost from interventions, but practice decays over time.

### Movement Model (replaces physics.rs)

Agents move toward their scheduled location each tick. Movement is simple linear interpolation (no physics simulation, drag, or velocity):

```
dx = target_x - x
dy = target_y - y
dist = sqrt(dx^2 + dy^2)
if dist > threshold:
    x += dx/dist * min(speed, dist)
    y += dy/dist * min(speed, dist)
```

No toroidal wrapping -- the barangay is a bounded map.

---

## 8. Environment System

### Grid Structure

The `EnvironmentGrid` replaces the existing `ZoneMap` and `PheromoneGrid`. Each cell (10m x 10m) tracks:

- **Soil contamination** (0.0-1.0): deposits from open defecation, decays naturally, spreads with rain.
- **Water contamination** (0.0-1.0): linked to nearby soil contamination and water source quality.
- **Terrain type**: determines location semantics and risk levels.
- **Facilities**: latrines, water pumps, handwash stations placed by interventions.

### Facility Placement

Facilities are point objects on the grid. When placed:
- **Latrine**: all households within radius get `household_has_latrine = true`. Reduces open defecation for those households.
- **Water pump**: sets `has_water_source = true` for the cell. Improves water quality in radius.
- **Handwash station**: sets `school_has_wash_facility = true` for nearby school.

### Procedural Map Generation

Each barangay is generated from its `BarangayConfig`:

1. Place residential zones (dense grid for urban, scattered for rural).
2. Place 1-2 schools in central area.
3. Place health center.
4. Place creek/water features (urban: drainage channels, rural: streams).
5. Place roads connecting areas.
6. Assign households to residential cells.
7. Assign children to households and schools.
8. Initialize contamination from `baseline_soil_contamination`.

---

## 9. Intervention System

### User-Triggered Interventions

The user triggers interventions via the toolbar. Each intervention:
1. Checks budget (deducts cost).
2. Creates an `ActiveIntervention` with duration.
3. Executes effects progressively or instantly.

| Intervention | Cost | Duration | Effect |
|---|---|---|---|
| MDA (school) | 5 per child | 1 day | Clears worms (species-dependent efficacy) |
| MDA (all) | 5 per child | 3 days | Same, all schools |
| Build latrine | 500 | Instant | Adds latrine at location |
| Water pump | 800 | Instant | Adds improved water source |
| Handwash station | 200 | Instant | Adds station at school |
| Education (cartoon) | 100 per session | 1 day | Knowledge boost +5-15% |
| Education (board game) | 150 per session | 1 day | Knowledge +5-15%, Practice +5-10% |
| BHW visits | 20 per household | 7 days | Knowledge +3-8%, attitude +2-5% |
| Budget increase | N/A (policy) | Permanent | Increases monthly increment |

### Programmatic Barriers

The thesis identifies barriers that reduce intervention effectiveness:
- **Medicine stockouts**: `medicine_supply_reliability` modifies MDA coverage.
- **BHW shortage**: limits house-to-house visit coverage.
- **COVID closure**: if enabled, school-based MDA and education cannot run during closure months.
- **Competing priorities**: random events can temporarily reduce health worker availability.

---

## 10. Frontend Component Hierarchy

```
src/app/page.tsx (SthHome)
|
+-- src/components/controls/sth-controls.tsx
|   Play/Pause/Step/Speed, Time display (Day X, Month Y)
|
+-- src/components/controls/intervention-toolbar.tsx
|   MDA button, WASH dropdown (latrine/pump/station),
|   Education dropdown, BHW visits, Budget display
|
+-- Main content area (flex row)
|   |
|   +-- src/components/canvas/sth-canvas.tsx
|   |   Dual canvas: WebGL2 agents + Canvas2D overlay
|   |   Click to place facilities, click agent to inspect
|   |   Comparison mode: split view with divider
|   |
|   +-- Sidebar (flex col)
|       |
|       +-- src/components/sidebar/agent-inspector.tsx
|       |   KAP scores, infection status, household, school
|       |
|       +-- src/components/charts/prevalence-chart.tsx
|       |   STH prevalence over time (line chart, per species)
|       |
|       +-- src/components/charts/kap-chart.tsx
|       |   KAP distribution (bar chart or radar)
|       |
|       +-- src/components/charts/intervention-timeline.tsx
|       |   Timeline of interventions + effects
|       |
|       +-- src/components/sidebar/barangay-stats.tsx
|       |   Current prevalence, coverage, KAP averages
|       |
|       +-- src/components/sidebar/sth-event-log.tsx
|       |   MDA completed, outbreak detected, etc.
|       |
|       +-- src/components/charts/cost-tracker.tsx
|           Budget remaining, spending breakdown
|
+-- src/components/ui/toast.tsx (reuse)
+-- src/components/ui/keyboard-help.tsx (adapt)
```

### Rendering Changes

The WebGL renderer gets modified, not replaced:

1. **Creature sprites** become **agent icons**: circles colored by type, with infection severity as border/glow.
2. **Food sprites** become **facility icons**: latrine (brown square), pump (blue circle), handwash (green cross).
3. **Pheromone/zone overlay** becomes **contamination heatmap**: red overlay showing soil contamination grid.
4. **Minimap**: shows full barangay with agent dots and facility markers.
5. **Comparison mode**: two WebGL contexts side by side (or one wide context with viewport split).

---

## 11. Data Transfer Protocol

### Agent Buffer (replaces creature buffer)

```
FLOATS_PER_AGENT = 16

Per agent (f32 x 16):
[0]  x              -- world position
[1]  y              -- world position
[2]  target_x       -- destination (for movement animation)
[3]  target_y       -- destination
[4]  agent_type     -- 0=Child, 1=Parent, 2=Teacher, 3=HealthWorker
[5]  infection_status -- 0=negative, 1=light, 2=moderate, 3=heavy (worst across species)
[6]  epg_norm       -- 0.0-1.0 (log-scaled total EPG)
[7]  knowledge      -- 0.0-1.0
[8]  attitude       -- 0.0-1.0
[9]  practice       -- 0.0-1.0
[10] household_id   -- for household highlighting
[11] school_id      -- for school grouping
[12] r              -- color red (0.0-1.0)
[13] g              -- color green
[14] b              -- color blue
[15] barangay_id    -- 0 or 1 (for comparison mode split)
```

### Environment Buffer (new)

Flat f32 array, one float per grid cell = soil contamination level (0.0-1.0).
Dimensions sent separately as `[grid_width, grid_height, cell_size]`.

### Facility Buffer (new)

```
FLOATS_PER_FACILITY = 4
[x, y, type, active]
```

Where type: 0=latrine, 1=water_pump, 2=handwash_station, 3=school, 4=health_center.

### Stats (JSON, throttled to 5Hz)

```typescript
interface SthStats {
    tick: number;
    day: number;
    month: number;
    agentCount: number;
    prevalenceAscaris: number;
    prevalenceTrichuris: number;
    prevalenceHookworm: number;
    prevalenceAny: number;
    meanEpg: number;
    meanKnowledge: number;
    meanAttitude: number;
    meanPractice: number;
    latrineCoverage: number;
    waterCoverage: number;
    budgetRemaining: number;
    budgetSpent: number;
    mdaRounds: number;
    childrenTreated: number;
    // Per-barangay breakdown (comparison mode)
    barangays: BarangayStatsJson[];
}
```

### Worker Commands (new)

```typescript
type SthCommand =
    | { type: 'init'; seed?: number; config?: string }
    | { type: 'step'; count: number }
    | { type: 'pause' }
    | { type: 'reset'; seed?: number; config?: string }
    | { type: 'launchMDA'; schoolId: number; drug: number }
    | { type: 'buildLatrine'; x: number; y: number }
    | { type: 'buildWaterPump'; x: number; y: number }
    | { type: 'buildHandwashStation'; x: number; y: number }
    | { type: 'launchEducation'; method: number; schoolId: number }
    | { type: 'launchBhwVisits'; coverage: number }
    | { type: 'increaseBudget'; multiplier: number }
    | { type: 'selectAgent'; index: number | null }
    | { type: 'toggleComparisonMode' };
```

---

## 12. State Management

### New Zustand Store: `sth-store.ts`

Replaces `simulation-store.ts`. Key differences:

- No `species`, `phyloTree`, `brainWeights`, `pheromoneData`, `zoneData` fields.
- Added: `prevalenceHistory`, `kapDistribution`, `interventionLog`, `budgetState`, `comparisonMode`, `selectedBarangay`, `facilityPlacementMode`.

```typescript
interface SthStore {
    // Sim state
    state: SimState;
    stats: SthStats | null;
    speed: number;
    error: string | null;

    // Time
    currentDay: number;
    currentMonth: number;
    timeOfDay: string;

    // Map interaction
    selectedAgent: number | null;
    facilityPlacementMode: FacilityType | null; // latrine/pump/station
    comparisonMode: boolean;

    // Data
    prevalenceHistory: PrevalencePoint[];
    kapDistribution: KapDistribution;
    interventionLog: InterventionEvent[];
    budgetRemaining: number;
    budgetSpent: number;

    // UI toggles
    showContaminationHeatmap: boolean;
    showAgentPaths: boolean;
    showFacilities: boolean;
    dataLens: SthDataLens; // 'infection' | 'kap' | 'contamination' | 'agents'
}
```

---

## 13. Rendering Pipeline

### WebGL2 Changes

The instanced sprite renderer is adapted:

1. **Sprite atlas**: Generate new atlas with 4 agent type icons (child circle, parent triangle, teacher diamond, BHW cross) + 3 facility icons. Same 2x2 atlas grid approach, expanded to 4x2.
2. **Vertex shader**: Remove velocity elongation and wobble. Add infection glow (pulsing red border for infected agents based on `infection_status`). Agent size is uniform (no `size_trait`).
3. **Fragment shader**: Infection severity as ring color (green=negative, yellow=light, orange=moderate, red=heavy).
4. **Instance data**: 16 floats per instance instead of 12. Update VAO attribute layout.

### Canvas2D Overlay Changes

1. **Contamination heatmap**: Replace pheromone overlay. Red-channel opacity proportional to `soil_contamination`.
2. **Facility markers**: Labeled icons for latrines, pumps, stations.
3. **Building placement preview**: When `facilityPlacementMode` is active, show ghost icon at cursor with placement radius.
4. **School/home labels**: Labeled rectangles for key locations.
5. **Day/night cycle**: Subtle background tint to indicate time of day.

### Minimap

Adapt existing minimap to show barangay boundaries, agent clusters, and facility positions.

---

## 14. File-by-File Migration Map

### Rust Files

| Old File | New File | Action |
|---|---|---|
| `lib.rs` | `lib.rs` | **Replace**: new module declarations, `SthSimulation` API |
| `world.rs` | `sth_world.rs` | **Replace**: `SthWorld` with STH step logic |
| `creature.rs` | `agent.rs` | **Replace**: `Agent` struct, `AgentType`, `Location` |
| `brain.rs` | `kap.rs` | **Replace**: KAP model replaces neural network |
| `physics.rs` | `schedule.rs` | **Replace**: daily cycle + linear movement replaces physics |
| `speciation.rs` | `infection.rs` | **Replace**: infection tracking replaces species tracking |
| `config.rs` | `sth_config.rs` | **Replace**: `SthConfig` with STH parameters |
| `render_buffer.rs` | `sth_render_buffer.rs` | **Replace**: 16-float agent layout |
| `events.rs` | `event_tracker.rs` | **Replace**: STH domain events |
| `zones.rs` | `environment.rs` | **Replace**: contamination grid |
| `pheromone.rs` | _(removed)_ | **Delete**: no pheromones in STH sim |
| `spatial_hash.rs` | `spatial_hash.rs` | **Keep**: reuse for agent proximity queries |
| `profile.rs` | `profile.rs` | **Keep**: reuse tick profiling |
| _(new)_ | `intervention.rs` | **Create**: MDA, WASH, education |
| _(new)_ | `barangay.rs` | **Create**: barangay config + generation |

### Frontend Files

| Old File | New File | Action |
|---|---|---|
| `src/lib/types.ts` | `src/lib/sth-types.ts` | **Replace**: new type definitions |
| `src/stores/simulation-store.ts` | `src/stores/sth-store.ts` | **Replace**: new store |
| `src/workers/simulation.worker.ts` | `src/workers/sth.worker.ts` | **Replace**: new worker commands |
| `src/hooks/use-simulation.ts` | `src/hooks/use-sth-simulation.ts` | **Replace**: new hook |
| `src/hooks/use-canvas.ts` | `src/hooks/use-canvas.ts` | **Adapt**: camera + click handlers |
| `src/hooks/use-keyboard.ts` | `src/hooks/use-keyboard.ts` | **Adapt**: new shortcuts |
| `src/app/page.tsx` | `src/app/page.tsx` | **Replace**: new layout |
| `src/components/canvas/webgl-renderer.ts` | `src/components/canvas/sth-webgl-renderer.ts` | **Adapt**: new shaders + instance layout |
| `src/components/canvas/overlay-renderer.ts` | `src/components/canvas/sth-overlay.ts` | **Replace**: contamination + facilities |
| `src/components/canvas/simulation-canvas.tsx` | `src/components/canvas/sth-canvas.tsx` | **Replace**: new canvas component |
| `src/components/canvas/minimap.ts` | `src/components/canvas/sth-minimap.ts` | **Adapt**: barangay minimap |
| `src/components/canvas/sprite-atlas.ts` | `src/components/canvas/sth-sprite-atlas.ts` | **Replace**: new agent/facility sprites |
| `src/components/canvas/particles.ts` | _(removed)_ | **Delete**: no particles needed |
| `src/components/controls/simulation-controls.tsx` | `src/components/controls/sth-controls.tsx` | **Replace**: time display + controls |
| `src/components/controls/intervention-tools.tsx` | `src/components/controls/intervention-toolbar.tsx` | **Replace**: MDA/WASH/education buttons |
| `src/components/controls/parameter-panel.tsx` | `src/components/controls/sth-parameter-panel.tsx` | **Replace**: STH config params |
| `src/components/sidebar/species-panel.tsx` | `src/components/sidebar/barangay-stats.tsx` | **Replace**: prevalence + coverage stats |
| `src/components/sidebar/creature-inspector.tsx` | `src/components/sidebar/agent-inspector.tsx` | **Replace**: KAP + infection detail |
| `src/components/sidebar/event-log.tsx` | `src/components/sidebar/sth-event-log.tsx` | **Replace**: STH events |
| `src/components/sidebar/phylo-tree.tsx` | _(removed)_ | **Delete**: no phylogeny |
| `src/components/sidebar/extinction-timeline.tsx` | `src/components/charts/intervention-timeline.tsx` | **Replace**: intervention timeline |
| `src/components/charts/population-chart.tsx` | `src/components/charts/prevalence-chart.tsx` | **Replace**: prevalence over time |
| `src/components/charts/trait-chart.tsx` | `src/components/charts/kap-chart.tsx` | **Replace**: KAP distribution |
| _(new)_ | `src/components/charts/cost-tracker.tsx` | **Create**: budget visualization |
| `src/components/ui/toast.tsx` | `src/components/ui/toast.tsx` | **Keep**: reuse |
| `src/components/ui/keyboard-help.tsx` | `src/components/ui/keyboard-help.tsx` | **Adapt**: new shortcuts |
| `src/components/ui/save-load-modal.tsx` | `src/components/ui/save-load-modal.tsx` | **Adapt**: save/load STH state |
| `src/stores/toast-store.ts` | `src/stores/toast-store.ts` | **Keep**: reuse |
| `src/lib/save-system.ts` | `src/lib/save-system.ts` | **Adapt**: STH save format |

---

## 15. Failure Modes

### Simulation Stability

| Failure Mode | Cause | Mitigation |
|---|---|---|
| Population collapse to 0 | Over-tuned transmission leads to 100% heavy infection | Agents don't die from STH in this model (thesis scope is morbidity, not mortality). Population is static. |
| Contamination runaway (all cells at 1.0) | Decay rate too low relative to deposit rate | Decay rate floor of 0.0001/tick. Cap contamination deposit per tick. |
| EPG overflow | Worm burden accumulates without bound | Cap worm_burden at 500 (produces EPG ~5M, well beyond "heavy"). |
| NaN propagation | Division by zero in probability calculations | Guard all divisions. `poly_exp_neg` clamps input. Log-scale EPG uses `max(1.0)` before log. |
| Budget underflow | User overspends | All intervention launches check `budget_remaining >= cost` before executing. Return error if insufficient. |

### Worker Failure

- **Watchdog**: Existing heartbeat mechanism (5s timeout) reused unchanged.
- **WASM panic**: `console_error_panic_hook` catches and reports via `postMessage({ type: 'error' })`.
- **Buffer detach race**: Existing double-buffer protocol handles this.

---

## 16. Security & Performance

### Security

- **No external data**: All computation is local. No API calls, no user data collection.
- **Config validation**: `SthConfig::validate()` bounds-checks all parameters before creating world.
- **No file system access**: Save/load uses IndexedDB only.
- Same security posture as existing simulation -- browser sandbox is the perimeter.

### Performance

**Target**: 500 agents, 60x60 environment grid, 60 ticks/frame at <14ms per frame.

| Phase | Estimated Cost | Notes |
|---|---|---|
| Schedule + Movement | ~0.2ms | Simple arithmetic, 500 agents |
| Behavioral Decisions | ~0.1ms | Bernoulli trials, 300 children |
| Contamination Deposit | ~0.05ms | Grid writes, ~100 non-latrine agents |
| Transmission | ~0.3ms | 300 children x 3 species x probability calc |
| Environment Decay | ~0.1ms | 3600 grid cells |
| Interventions | ~0.05ms | Sparse events |
| KAP Update | ~0.05ms | Daily only (every 24th tick) |
| Render Buffer Pack | ~0.1ms | 500 agents x 16 floats |
| **Total** | **~1.0ms** | Well within 14ms budget |

The STH simulation is computationally simpler than the evolution simulation (no neural network forward pass, no food grid query, no reproduction). Performance is not a concern.

### Memory

- 500 agents x ~128 bytes = 64KB
- 60x60 grid x 16 bytes = 58KB
- Render buffer: 500 x 16 x 4 = 32KB
- Total: ~200KB simulation state (much less than current ~2MB with neural networks)

---

## 17. Testing & Observability

### Rust Unit Tests

| Test | What it validates |
|---|---|
| `test_transmission_probability_bounds` | P always in [0, 1] for all input combinations |
| `test_worm_burden_decay` | Without new infections, burden decays to 0 |
| `test_mda_clears_infection` | After MDA with efficacy=1.0, EPG drops to 0 |
| `test_reinfection_cycle` | After MDA, prevalence returns to baseline in 6-12 months |
| `test_urban_higher_prevalence` | Urban default config produces ~20% prevalence, rural ~5% |
| `test_wash_reduces_transmission` | Adding latrines reduces prevalence |
| `test_kap_decay` | Practice score decays without reinforcement |
| `test_structural_dominance` | High KAP + no latrine = same prevalence as low KAP + no latrine |
| `test_poly_exp_neg_accuracy` | Approximation within 1% of std exp for x in [0, 3] |
| `test_schedule_coverage` | All 24 hours map to a valid TimeOfDay |
| `test_config_validation` | Invalid configs rejected |
| `test_deterministic_seed` | Same seed produces identical 100-day runs |
| `test_comparison_mode_independence` | Two barangays don't contaminate each other |

### Integration Tests

- **Calibration test**: Run 365-day simulation with default urban config, assert prevalence in [15%, 25%] range (thesis: 20.3%).
- **MDA effectiveness test**: Run MDA, assert prevalence drops >50% immediately, returns to baseline within 6-12 months.
- **WASH effectiveness test**: Run latrine intervention, assert long-term prevalence reduction.

### Frontend Tests

- Component rendering tests with mock data (Vitest + React Testing Library).
- Worker message round-trip test.
- Intervention toolbar interaction tests.

### Observability

- **Tick profiler**: Reuse existing `TickProfile` with STH-specific phase names.
- **Stats throttle**: 5Hz update to Zustand store (existing pattern).
- **Event log**: Domain events surface in UI: "MDA completed: 245 children treated", "Outbreak detected in Guadalupe", "Rainy season: contamination spreading".

---

## 18. Milestone Task Breakdown

### M1: Rust Core -- Agent + Infection Engine (3-4 days)
1. Create `agent.rs`, `infection.rs`, `sth_config.rs`, `barangay.rs`
2. Create `SthWorld` with stub `step()` that only moves agents
3. Implement infection transmission and worm burden dynamics
4. Implement `InfectionState` intensity thresholds (WHO cutoffs)
5. Write unit tests for transmission math
6. Verify `poly_exp_neg` accuracy

### M2: Rust Core -- Environment + Schedule (2-3 days)
1. Create `environment.rs` with contamination grid
2. Create `schedule.rs` with daily cycle
3. Implement contamination deposit and decay
4. Implement procedural barangay map generation
5. Wire schedule into `SthWorld::step()`
6. Write tests for environment dynamics

### M3: Rust Core -- KAP + Behavioral Model (2 days)
1. Create `kap.rs` with KAP scores and behavioral probabilities
2. Implement structural gates (latrine access, water source)
3. Implement KAP evolution (education boost, decay, social influence)
4. Write tests verifying structural dominance over KAP
5. Calibration run: tune parameters to match thesis findings

### M4: Rust Core -- Interventions (2-3 days)
1. Create `intervention.rs` with all intervention types
2. Implement MDA with drug efficacy
3. Implement WASH facility placement
4. Implement education campaigns
5. Implement BHW visits
6. Implement budget system
7. Write tests for each intervention

### M5: Rust Core -- Render Buffer + WASM API (1-2 days)
1. Create `sth_render_buffer.rs` with 16-float layout
2. Create `sth_event_tracker.rs`
3. Rewrite `lib.rs` with `SthSimulation` API
4. Build WASM and verify binary size <300KB
5. Manual smoke test via browser console

### M6: Frontend -- Worker Bridge + Store (2 days)
1. Create `sth-types.ts` with all TypeScript types
2. Create `sth.worker.ts` with new command handling
3. Create `sth-store.ts` Zustand store
4. Create `use-sth-simulation.ts` hook
5. Verify worker-main thread round-trip

### M7: Frontend -- Rendering (3-4 days)
1. Create `sth-sprite-atlas.ts` with agent/facility sprites
2. Adapt WebGL renderer for 16-float instances
3. Create contamination heatmap overlay
4. Create facility marker rendering
5. Create barangay map background (roads, zones)
6. Adapt minimap for barangay view

### M8: Frontend -- UI Shell (2-3 days)
1. Create `sth-controls.tsx` with time display
2. Create `intervention-toolbar.tsx`
3. Create `sth-canvas.tsx` with click-to-place facility
4. Create sidebar layout with stats, charts, event log
5. Wire all components in `page.tsx`

### M9: Frontend -- Dashboard Charts (2 days)
1. Create `prevalence-chart.tsx` (line chart per species)
2. Create `kap-chart.tsx` (distribution)
3. Create `intervention-timeline.tsx`
4. Create `cost-tracker.tsx` (budget breakdown)
5. Create `barangay-stats.tsx` panel

### M10: Frontend -- Agent Inspector + Interaction (1-2 days)
1. Create `agent-inspector.tsx` (KAP, infection, household detail)
2. Implement click-to-select agent
3. Implement facility placement mode
4. Adapt keyboard shortcuts

### M11: Comparison Mode (2-3 days)
1. Add second barangay to `SthConfig`
2. Split render viewport for two barangays
3. Independent stats per barangay in dashboard
4. Side-by-side prevalence comparison charts

### M12: Calibration + Polish (2-3 days)
1. Calibrate transmission parameters to match thesis findings
2. Verify urban (20.3%) vs rural (4.9%) prevalence
3. Verify 6-12 month reinfection cycle
4. Verify MDA reduces burden short-term
5. Add COVID disruption scenario
6. Deploy to Vercel
7. Test mobile responsiveness

**Total estimated effort: 22-31 days**

---

## 19. Open Questions

1. **Comparison mode implementation**: Should we run two independent `SthSimulation` instances in the same Worker (simpler but doubles memory), or one `SthSimulation` with two sub-worlds (shared code but more complex step logic)? **Recommendation**: Two independent instances. Memory cost is negligible (<500KB total). Code is simpler.

2. **Hookworm inclusion**: The thesis found 0% hookworm prevalence in Cebu City. Should we still model it for the simulation to be general-purpose, or drop it to simplify? **Recommendation**: Keep it with near-zero base transmission rate. Users can increase the rate to explore "what-if" scenarios.

3. **COVID scenario**: Should COVID school closure be a built-in scenario or user-triggered event? **Recommendation**: User-triggered toggle in parameter panel. Default off.

4. **Save/load compatibility**: The save system currently stores evolution sim state. Should we maintain backward compatibility or clean break? **Recommendation**: Clean break. Different domain, different save format. Old saves are irrelevant.

5. **Deployment**: Same Vercel deployment or separate project? **Recommendation**: Same repo, same deployment. The old simulation code can be kept on a `legacy/evolution` branch if needed for reference.

6. **Mobile**: The thesis data suggests this could be an educational tool. Should we invest more in mobile UX (touch-friendly facility placement, simplified controls)? **Recommendation**: Basic mobile support (responsive layout, touch events for placement). Full mobile polish as a post-launch item.

7. **Validation dashboard**: Should we add a "calibration mode" that overlays thesis data points on the simulation output charts? **Recommendation**: Yes, as a developer tool (hidden behind URL param `?calibration=true`). Shows thesis prevalence/KAP values as dashed reference lines on charts.
