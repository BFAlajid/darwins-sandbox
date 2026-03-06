pub mod brain;
pub mod config;
pub mod creature;
pub mod physics;
pub mod profile;
pub mod render_buffer;
pub mod spatial_hash;
pub mod speciation;
pub mod world;

use wasm_bindgen::prelude::*;

use config::SimConfig;
use world::World;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct Simulation {
    world: World,
}

#[wasm_bindgen]
impl Simulation {
    /// Create a new simulation with default config.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Simulation, JsValue> {
        let config = SimConfig::default();
        let world =
            World::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(Simulation { world })
    }

    /// Create a new simulation with a specific seed.
    pub fn with_seed(seed: u64) -> Result<Simulation, JsValue> {
        let mut config = SimConfig::default();
        config.seed = seed;
        let world =
            World::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(Simulation { world })
    }

    /// Create a new simulation from a JSON config string.
    pub fn with_config(config_json: &str) -> Result<Simulation, JsValue> {
        let config: SimConfig = serde_json::from_str(config_json).map_err(|e| {
            JsValue::from_str(&format!("Invalid config JSON: {}", e))
        })?;
        let world =
            World::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(Simulation { world })
    }

    /// Advance the simulation by one tick.
    pub fn step(&mut self) {
        self.world.step();
    }

    /// Get pointer to creature render data (flat f32 buffer).
    /// MUST re-acquire Float32Array view after every step() call.
    pub fn get_render_data_ptr(&self) -> *const f32 {
        self.world.render_data_ptr()
    }

    /// Get length of render data buffer (number of f32 values).
    pub fn get_render_data_len(&self) -> usize {
        self.world.render_data_len()
    }

    /// Get creature render data as a copied Vec<f32>.
    /// Simpler than pointer-based access — no memory management needed on JS side.
    pub fn get_creature_render_data(&self) -> Vec<f32> {
        let ptr = self.world.render_data_ptr();
        let len = self.world.render_data_len();
        if len == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
    }

    /// Get current creature count.
    pub fn get_creature_count(&self) -> usize {
        self.world.creature_count()
    }

    /// Get current food count.
    pub fn get_food_count(&self) -> usize {
        self.world.food_count()
    }

    /// Get current tick number.
    pub fn get_tick(&self) -> u64 {
        self.world.tick
    }

    /// Get maximum generation observed.
    pub fn get_generation_max(&self) -> u32 {
        self.world.generation_max
    }

    /// Get total births since start.
    pub fn get_total_births(&self) -> u64 {
        self.world.total_births
    }

    /// Get total deaths since start.
    pub fn get_total_deaths(&self) -> u64 {
        self.world.total_deaths
    }

    /// Get NaN death count (should always be 0 in normal operation).
    pub fn get_nan_deaths(&self) -> u32 {
        self.world.nan_deaths
    }

    /// Get energy drift percentage.
    pub fn get_energy_drift_pct(&self) -> f32 {
        self.world.energy_drift_pct()
    }

    /// Get tick profile as JSON string.
    pub fn get_profile_json(&self) -> String {
        serde_json::to_string(self.world.profile()).unwrap_or_default()
    }

    /// Get food render data as flat f32 array: [x, y, energy, ...]
    pub fn get_food_render_data(&self) -> Vec<f32> {
        self.world.food_render_data()
    }

    /// Get world width.
    pub fn get_world_width(&self) -> f32 {
        self.world.config.world_width
    }

    /// Get world height.
    pub fn get_world_height(&self) -> f32 {
        self.world.config.world_height
    }

    /// Get number of active species.
    pub fn get_species_count(&self) -> usize {
        self.world.species_count()
    }

    /// Get species data as JSON string.
    pub fn get_species_json(&self) -> String {
        self.world.species_json()
    }
}
