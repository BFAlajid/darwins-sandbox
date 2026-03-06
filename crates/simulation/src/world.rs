use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use slotmap::DenseSlotMap;

use crate::config::SimConfig;
use crate::creature::{Creature, CreatureKey};
use crate::physics;
use crate::profile::{TickProfile, Timer};
use crate::render_buffer::RenderBuffer;
use crate::spatial_hash::SpatialHash;

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
    last_profile: TickProfile,

    // Catastrophe state
    catastrophe_ticks_remaining: u32,

    // Energy audit
    energy_drift_pct: f32,
}

/// Simple grid for fast food lookup by creatures
struct FoodGrid {
    inv_cell_size: f32,
    grid_width: usize,
    cells: Vec<Vec<usize>>, // indices into World.food
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

        // --- Physics: movement with simple food seeking (M1: no brain) ---
        let t_phase = Timer::start();
        let keys: Vec<CreatureKey> = self.creatures.keys().collect();
        for key in &keys {
            let creature = &self.creatures[*key];

            // Find nearest food within vision range for simple seek behavior
            let vision_sq = creature.vision_range * creature.vision_range;
            let nearest_food_idx = self.food_spatial.query_nearest(
                creature.x, creature.y, vision_sq,
                &self.food, self.config.world_width, self.config.world_height,
            );
            let nearest_food_angle = nearest_food_idx.map(|fi| {
                let f = &self.food[fi];
                let mut dx = f.x - creature.x;
                let mut dy = f.y - creature.y;
                if dx.abs() > self.config.world_width * 0.5 {
                    dx -= dx.signum() * self.config.world_width;
                }
                if dy.abs() > self.config.world_height * 0.5 {
                    dy -= dy.signum() * self.config.world_height;
                }
                dy.atan2(dx)
            });

            // M1: Simple food-seeking + random exploration
            let (thrust, turn) = if let Some(food_angle) = nearest_food_angle {
                // Steer toward food
                let mut angle_diff = food_angle - creature.rotation;
                // Normalize to [-PI, PI]
                while angle_diff > std::f32::consts::PI { angle_diff -= std::f32::consts::TAU; }
                while angle_diff < -std::f32::consts::PI { angle_diff += std::f32::consts::TAU; }
                let turn = angle_diff.clamp(-self.config.max_turn_rate, self.config.max_turn_rate);
                (0.8, turn)
            } else {
                // Random exploration when no food visible
                let thrust = self.rng.gen_range(0.3..1.0);
                let turn = self.rng.gen_range(-self.config.max_turn_rate..self.config.max_turn_rate);
                (thrust, turn)
            };

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
                // Decide to reproduce (M1: always reproduce when able)
                let offspring = Creature::new_offspring(creature, &mut self.rng, &self.config);
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
                creature.energy -= self.config.basal_cost * 0.5; // extra 50% cost
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

        // --- Pack render buffer ---
        let t_render = Timer::start();
        self.render_buffer.pack(
            &self.creatures,
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
