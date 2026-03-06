use rand::Rng;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

use crate::config::SimConfig;

new_key_type! {
    pub struct CreatureKey;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Creature {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub rotation: f32,
    pub energy: f32,
    pub age: u32,
    pub generation: u32,

    // Physical traits (random in M1, genome-derived later)
    pub speed_trait: f32,
    pub size_trait: f32,
    pub vision_range: f32,

    // Lifecycle
    pub ticks_since_reproduction: u32,
    pub children_count: u32,
    pub species_id: u32,
}

impl Creature {
    pub fn new_random(rng: &mut impl Rng, config: &SimConfig, species_id: u32) -> Self {
        let x = rng.gen_range(0.0..config.world_width);
        let y = rng.gen_range(0.0..config.world_height);
        let rotation = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed_trait = rng.gen_range(config.min_speed..=config.max_speed);
        let size_trait = rng.gen_range(config.min_size..=config.max_size);
        let vision_range = rng.gen_range(config.min_vision_range..=config.max_vision_range);

        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            rotation,
            energy: config.max_energy * 0.5,
            age: 0,
            generation: 0,
            speed_trait,
            size_trait,
            vision_range,
            ticks_since_reproduction: 0,
            children_count: 0,
            species_id,
        }
    }

    pub fn new_offspring(
        parent: &Creature,
        rng: &mut impl Rng,
        config: &SimConfig,
    ) -> Self {
        // Place near parent with small random offset
        let offset_x = rng.gen_range(-10.0..10.0);
        let offset_y = rng.gen_range(-10.0..10.0);
        let x = (parent.x + offset_x).rem_euclid(config.world_width);
        let y = (parent.y + offset_y).rem_euclid(config.world_height);

        // Random trait variation in M1 (proper mutation in M4)
        let speed_trait = (parent.speed_trait + rng.gen_range(-0.5..0.5))
            .clamp(config.min_speed, config.max_speed);
        let size_trait = (parent.size_trait + rng.gen_range(-0.3..0.3))
            .clamp(config.min_size, config.max_size);
        let vision_range = (parent.vision_range + rng.gen_range(-5.0..5.0))
            .clamp(config.min_vision_range, config.max_vision_range);

        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            rotation: rng.gen_range(0.0..std::f32::consts::TAU),
            energy: parent.energy * config.reproduction_energy_share,
            age: 0,
            generation: parent.generation + 1,
            speed_trait,
            size_trait,
            vision_range,
            ticks_since_reproduction: 0,
            children_count: 0,
            species_id: parent.species_id,
        }
    }

    /// Mass derived from size (2D area proxy)
    #[inline(always)]
    pub fn mass(&self) -> f32 {
        self.size_trait * self.size_trait
    }

    /// Max thrust scales with cross-sectional area
    #[inline(always)]
    pub fn max_thrust(&self) -> f32 {
        self.size_trait.powf(1.5)
    }

    /// Whether this creature has reached maturity
    #[inline(always)]
    pub fn is_mature(&self, config: &SimConfig) -> bool {
        self.age >= config.maturation_period
    }

    /// Whether reproduction cooldown has elapsed
    #[inline(always)]
    pub fn can_reproduce(&self, config: &SimConfig) -> bool {
        self.is_mature(config)
            && self.energy >= config.reproduction_threshold
            && self.ticks_since_reproduction >= config.reproduction_cooldown
    }

    /// Basal metabolic cost with senescence
    #[inline(always)]
    pub fn basal_metabolic_cost(&self, config: &SimConfig) -> f32 {
        config.basal_cost * (1.0 + self.age as f32 / config.max_lifespan as f32)
    }

    /// Movement energy cost: speed² * mass * factor
    #[inline(always)]
    pub fn movement_cost(&self, speed_sq: f32, config: &SimConfig) -> f32 {
        speed_sq * self.mass() * config.movement_cost_factor
    }

    /// Returns true if the creature should die
    #[inline(always)]
    pub fn is_dead(&self, config: &SimConfig) -> bool {
        self.energy <= 0.0 || self.age >= config.max_lifespan
    }

    /// NaN guard — returns true if any field is NaN
    #[inline(always)]
    pub fn has_nan(&self) -> bool {
        self.x.is_nan()
            || self.y.is_nan()
            || self.vx.is_nan()
            || self.vy.is_nan()
            || self.energy.is_nan()
            || self.rotation.is_nan()
    }
}
