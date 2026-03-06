use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    // World
    pub world_width: f32,
    pub world_height: f32,
    pub target_total_energy: f32,
    pub seed: u64,

    // Food
    pub food_energy: f32,
    pub food_cluster_count: u32,
    pub food_cluster_shift_period: u32,
    pub season_period: u32,
    pub season_amplitude: f32,

    // Creatures
    pub initial_population: u32,
    pub max_energy: f32,
    pub basal_cost: f32,
    pub movement_cost_factor: f32,
    pub reproduction_threshold: f32,
    pub reproduction_energy_share: f32,
    pub maturation_period: u32,
    pub reproduction_cooldown: u32,
    pub max_lifespan: u32,

    // Physics
    pub drag_coefficient: f32,
    pub max_turn_rate: f32,

    // Vision
    pub vision_cone_angle: f32,
    pub min_vision_range: f32,
    pub max_vision_range: f32,

    // Traits
    pub min_speed: f32,
    pub max_speed: f32,
    pub min_size: f32,
    pub max_size: f32,

    // Brain (not used in Milestone 1, but validated)
    pub hidden_neurons: u32,
    pub weight_clamp: f32,

    // Mutation (not used in Milestone 1, but validated)
    pub base_mutation_rate: f32,
    pub base_mutation_strength: f32,
    pub trait_mutation_scale: f32,
    pub cauchy_probability: f32,
    pub mutation_rate_clamp_min: f32,
    pub mutation_rate_clamp_max: f32,

    // Speciation (not used in Milestone 1)
    pub compatibility_threshold: f32,
    pub target_species_min: u32,
    pub target_species_max: u32,

    // Catastrophes
    pub auto_catastrophe_interval: u32,
    pub catastrophe_duration: u32,
    pub catastrophe_food_multiplier: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            world_width: 800.0,
            world_height: 600.0,
            target_total_energy: 30000.0,
            seed: 42,

            food_energy: 15.0,
            food_cluster_count: 4,
            food_cluster_shift_period: 500,
            season_period: 1000,
            season_amplitude: 0.4,

            initial_population: 100,
            max_energy: 200.0,
            basal_cost: 0.15,
            movement_cost_factor: 0.02,
            reproduction_threshold: 140.0,
            reproduction_energy_share: 0.5,
            maturation_period: 40,
            reproduction_cooldown: 120,
            max_lifespan: 3000,

            drag_coefficient: 0.08,
            max_turn_rate: PI / 4.0,

            vision_cone_angle: 2.0 * PI / 3.0,
            min_vision_range: 30.0,
            max_vision_range: 120.0,

            min_speed: 1.0,
            max_speed: 8.0,
            min_size: 2.0,
            max_size: 10.0,

            hidden_neurons: 8,
            weight_clamp: 5.0,

            base_mutation_rate: 0.08,
            base_mutation_strength: 0.2,
            trait_mutation_scale: 0.5,
            cauchy_probability: 0.1,
            mutation_rate_clamp_min: 0.001,
            mutation_rate_clamp_max: 0.5,

            compatibility_threshold: 2.0,
            target_species_min: 3,
            target_species_max: 10,

            auto_catastrophe_interval: 7500,
            catastrophe_duration: 200,
            catastrophe_food_multiplier: 0.5,
        }
    }
}

impl SimConfig {
    pub fn validate(&self) -> Result<(), String> {
        macro_rules! check_range {
            ($field:ident, $min:expr, $max:expr) => {
                if !self.$field.is_finite() || self.$field < $min as f32 || self.$field > $max as f32
                {
                    return Err(format!(
                        "{} = {} is out of valid range [{}, {}]",
                        stringify!($field),
                        self.$field,
                        $min,
                        $max
                    ));
                }
            };
        }

        macro_rules! check_range_u32 {
            ($field:ident, $min:expr, $max:expr) => {
                if self.$field < $min || self.$field > $max {
                    return Err(format!(
                        "{} = {} is out of valid range [{}, {}]",
                        stringify!($field),
                        self.$field,
                        $min,
                        $max
                    ));
                }
            };
        }

        // World
        check_range!(world_width, 100.0, 10000.0);
        check_range!(world_height, 100.0, 10000.0);
        check_range!(target_total_energy, 100.0, 1000000.0);

        // Food
        check_range!(food_energy, 1.0, 1000.0);
        if self.food_cluster_count > 20 {
            return Err(format!("food_cluster_count = {} is out of valid range [0, 20]", self.food_cluster_count));
        }
        check_range_u32!(food_cluster_shift_period, 50, 50000);
        check_range_u32!(season_period, 100, 100000);
        check_range!(season_amplitude, 0.0, 0.9);

        // Creatures
        check_range_u32!(initial_population, 2, 5000);
        check_range!(max_energy, 10.0, 10000.0);
        check_range!(basal_cost, 0.01, 100.0);
        check_range!(movement_cost_factor, 0.0, 10.0);
        check_range!(reproduction_threshold, 1.0, 10000.0);
        check_range!(reproduction_energy_share, 0.1, 0.9);
        check_range_u32!(maturation_period, 1, 1000);
        check_range_u32!(reproduction_cooldown, 1, 10000);
        check_range_u32!(max_lifespan, 100, 100000);

        // Physics
        check_range!(drag_coefficient, 0.001, 1.0);
        check_range!(max_turn_rate, 0.01, PI);

        // Vision
        check_range!(vision_cone_angle, 0.1, 2.0 * PI);
        check_range!(min_vision_range, 1.0, 1000.0);
        check_range!(max_vision_range, 1.0, 1000.0);

        // Traits
        check_range!(min_speed, 0.1, 100.0);
        check_range!(max_speed, 0.1, 100.0);
        check_range!(min_size, 0.5, 100.0);
        check_range!(max_size, 0.5, 100.0);

        // Brain
        check_range_u32!(hidden_neurons, 2, 32);
        check_range!(weight_clamp, 0.1, 100.0);

        // Mutation
        check_range!(base_mutation_rate, 0.0, 1.0);
        check_range!(base_mutation_strength, 0.0, 5.0);
        check_range!(trait_mutation_scale, 0.0, 5.0);
        check_range!(cauchy_probability, 0.0, 1.0);
        check_range!(mutation_rate_clamp_min, 0.0, 1.0);
        check_range!(mutation_rate_clamp_max, 0.0, 1.0);

        // Speciation
        check_range!(compatibility_threshold, 0.01, 100.0);
        check_range_u32!(target_species_min, 1, 100);
        check_range_u32!(target_species_max, 1, 100);

        // Catastrophes
        check_range_u32!(auto_catastrophe_interval, 100, 1000000);
        check_range_u32!(catastrophe_duration, 10, 10000);
        check_range!(catastrophe_food_multiplier, 0.0, 1.0);

        // Cross-parameter validation
        if self.reproduction_threshold <= self.food_energy * 2.0 {
            return Err(format!(
                "reproduction_threshold ({}) must be > food_energy * 2 ({})",
                self.reproduction_threshold,
                self.food_energy * 2.0
            ));
        }
        if self.reproduction_energy_share < 0.3 {
            return Err(format!(
                "reproduction_energy_share ({}) must be >= 0.3 to prevent rapid-fire reproduction",
                self.reproduction_energy_share
            ));
        }
        if self.reproduction_cooldown < self.maturation_period / 2 {
            return Err(format!(
                "reproduction_cooldown ({}) must be >= maturation_period / 2 ({})",
                self.reproduction_cooldown,
                self.maturation_period / 2
            ));
        }
        if self.min_speed > self.max_speed {
            return Err("min_speed must be <= max_speed".to_string());
        }
        if self.min_size > self.max_size {
            return Err("min_size must be <= max_size".to_string());
        }
        if self.min_vision_range > self.max_vision_range {
            return Err("min_vision_range must be <= max_vision_range".to_string());
        }
        if self.mutation_rate_clamp_min > self.mutation_rate_clamp_max {
            return Err("mutation_rate_clamp_min must be <= mutation_rate_clamp_max".to_string());
        }
        if self.target_species_min > self.target_species_max {
            return Err("target_species_min must be <= target_species_max".to_string());
        }
        if self.reproduction_threshold > self.max_energy {
            return Err(format!(
                "reproduction_threshold ({}) must be <= max_energy ({})",
                self.reproduction_threshold, self.max_energy
            ));
        }

        // Memory budget gate
        let creature_size = 128; // approximate bytes per creature struct
        let nn_size = self.hidden_neurons as usize * 12 * 4;
        let cell_size_px = self.max_vision_range;
        let grid_w = (self.world_width / cell_size_px).ceil() as usize;
        let grid_h = (self.world_height / cell_size_px).ceil() as usize;
        let estimated_bytes = self.initial_population as usize * (creature_size + nn_size)
            + grid_w * grid_h * 32
            + self.initial_population as usize * 12 * 4; // render buffer
        if estimated_bytes > 512_000_000 {
            return Err(format!(
                "Config exceeds memory budget: estimated {} MB > 512 MB",
                estimated_bytes / 1_000_000
            ));
        }

        Ok(())
    }
}
