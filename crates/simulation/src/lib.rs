//! STH (Soil-Transmitted Helminths) Simulation — WASM API
//!
//! Agent-based model of STH transmission, prevention, and treatment
//! in Philippine barangay settings. Exposes a wasm_bindgen API for
//! the Next.js frontend to drive the simulation via Web Workers.

// --- Domain modules ---
pub mod agent;
pub mod barangay;
pub mod environment;
pub mod infection;
pub mod intervention;
pub mod kap;
pub mod schedule;
pub mod sth_config;
pub mod sth_event_tracker;
pub mod sth_render_buffer;
pub mod sth_world;

// --- Reusable infrastructure ---
pub mod profile;
pub mod spatial_hash;

use wasm_bindgen::prelude::*;

use schedule::TimeOfDay;
use sth_config::SthConfig;
use sth_world::SthWorld;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

// ===========================================================================
// SthSimulation — new WASM API
// ===========================================================================

#[wasm_bindgen]
pub struct SthSimulation {
    world: SthWorld,
}

#[wasm_bindgen]
impl SthSimulation {
    /// Create a new simulation with default configuration.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<SthSimulation, JsValue> {
        let config = SthConfig::default();
        let world = SthWorld::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(SthSimulation { world })
    }

    /// Create a new simulation with a specific random seed.
    pub fn with_seed(seed: u64) -> Result<SthSimulation, JsValue> {
        let mut config = SthConfig::default();
        config.seed = seed;
        let world = SthWorld::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(SthSimulation { world })
    }

    /// Create a new simulation from a JSON configuration string.
    pub fn with_config(config_json: &str) -> Result<SthSimulation, JsValue> {
        let config: SthConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
        let world = SthWorld::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(SthSimulation { world })
    }

    /// Advance the simulation by one tick (1 hour of simulated time).
    pub fn step(&mut self) {
        self.world.step();
    }

    // --- Render data ---

    /// Get agent render data as a flat f32 array (16 floats per agent).
    pub fn get_agent_render_data(&self) -> Vec<f32> {
        self.world.agent_render_data().to_vec()
    }

    /// Get environment contamination grid as a flat f32 array.
    pub fn get_env_render_data(&self) -> Vec<f32> {
        self.world.env_render_data().to_vec()
    }

    /// Get facility positions as a flat f32 array (4 floats per facility).
    pub fn get_facility_render_data(&self) -> Vec<f32> {
        self.world.facility_render_data().to_vec()
    }

    /// Get the number of agents in the simulation.
    pub fn get_agent_count(&self) -> usize {
        self.world.agent_count()
    }

    // --- Time ---

    /// Get the current tick number.
    pub fn get_tick(&self) -> u64 {
        self.world.tick
    }

    /// Get the current day of simulation (0-indexed).
    pub fn get_day(&self) -> u32 {
        TimeOfDay::day_of_simulation(self.world.tick)
    }

    /// Get the current month of simulation (0-indexed, 30-day months).
    pub fn get_month(&self) -> u32 {
        TimeOfDay::month_of_simulation(self.world.tick)
    }

    // --- Stats ---

    /// Get simulation statistics as a JSON string.
    pub fn get_stats_json(&self) -> String {
        self.world.get_stats_json()
    }

    // --- Interventions ---

    /// Launch Mass Drug Administration.
    /// school_id: -1 for all schools, 0+ for specific school.
    /// drug: 0 = Albendazole, 1 = Mebendazole.
    pub fn launch_mda(&mut self, school_id: i8, drug: u8) {
        self.world.launch_mda(school_id, drug);
    }

    /// Build a latrine at the given world position.
    pub fn build_latrine(&mut self, x: f32, y: f32) {
        self.world.build_latrine(x, y);
    }

    /// Build a water pump at the given world position.
    pub fn build_water_pump(&mut self, x: f32, y: f32) {
        self.world.build_water_pump(x, y);
    }

    /// Build a handwash station at the given world position.
    pub fn build_handwash_station(&mut self, x: f32, y: f32) {
        self.world.build_handwash_station(x, y);
    }

    /// Launch a health education campaign.
    /// method: 0=Cartoon, 1=BoardGame, 2=TeacherLed, 3=ParentMeeting.
    /// school_id: -1 for all schools, 0+ for specific school.
    pub fn launch_education(&mut self, method: u8, school_id: i8) {
        self.world.launch_education(method, school_id);
    }

    /// Launch BHW house-to-house visits.
    /// coverage: fraction of households to visit (0.0-1.0).
    pub fn launch_bhw_visits(&mut self, coverage: f32) {
        self.world.launch_bhw_visits(coverage);
    }

    /// Increase the monthly budget increment by a multiplier.
    pub fn increase_budget(&mut self, multiplier: f32) {
        self.world.increase_budget(multiplier);
    }

    // --- Agent inspection ---

    /// Get detailed information about a specific agent by index.
    pub fn get_agent_detail_json(&self, index: usize) -> String {
        self.world.get_agent_detail_json(index)
    }

    // --- Events ---

    /// Get new events since last call as JSON (incremental).
    pub fn drain_events_json(&mut self) -> String {
        self.world.drain_events_json()
    }

    // --- World info ---

    /// Get the world width in pixels.
    pub fn get_world_width(&self) -> f32 {
        self.world.config.world_width
    }

    /// Get the world height in pixels.
    pub fn get_world_height(&self) -> f32 {
        self.world.config.world_height
    }

    /// Get environment grid dimensions: [width, height, cell_size].
    pub fn get_env_grid_dims(&self) -> Vec<f32> {
        vec![
            self.world.environment.grid_width() as f32,
            self.world.environment.grid_height() as f32,
            self.world.environment.cell_size(),
        ]
    }

    // --- Budget ---

    /// Get the remaining budget.
    pub fn get_budget_remaining(&self) -> f32 {
        self.world.budget_remaining
    }

    /// Get the total budget spent.
    pub fn get_budget_spent(&self) -> f32 {
        self.world.budget_spent
    }

    /// Get the random seed used for this simulation.
    pub fn get_seed(&self) -> u64 {
        self.world.config.seed
    }
}

