use serde::{Deserialize, Serialize};

/// Tick phase timing instrumentation. Exposed to JS via stats.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TickProfile {
    pub spatial_hash_us: u32,
    pub physics_us: u32,
    pub energy_us: u32,
    pub reproduction_us: u32,
    pub food_us: u32,
    pub cleanup_us: u32,
    pub render_pack_us: u32,
    pub total_us: u32,
    pub creature_count: u32,
    pub food_count: u32,
}

/// Simple microsecond timer.
/// Uses js_sys::Date on WASM, std::time::Instant on native (for tests).
pub struct Timer {
    #[cfg(target_arch = "wasm32")]
    start: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Timer {
    #[inline]
    pub fn start() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            start: js_sys::Date::now(),
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }

    #[inline]
    pub fn elapsed_us(&self) -> u32 {
        #[cfg(target_arch = "wasm32")]
        {
            ((js_sys::Date::now() - self.start) * 1000.0) as u32
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_micros() as u32
        }
    }
}
