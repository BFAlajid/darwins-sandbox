use serde::{Deserialize, Serialize};

use crate::barangay::BarangayConfig;

/// Configuration for the STH infection and deworming simulation.
///
/// Default values are calibrated to reproduce thesis findings:
/// - 11.7% overall STH prevalence
/// - 20.3% urban vs 4.9% rural prevalence
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SthConfig {
    /// Random seed for deterministic simulation
    pub seed: u64,

    /// World width in pixels for rendering (800 single, 1600 comparison)
    pub world_width: f32,
    /// World height in pixels for rendering
    pub world_height: f32,

    // --- Barangay configs ---
    /// 1 for single mode, 2 for comparison mode
    pub barangays: Vec<BarangayConfig>,

    // --- Simulation speed ---
    /// Ticks per simulated day (always 24; 1 tick = 1 hour)
    pub ticks_per_day: u32,

    // --- Transmission parameters (species-specific lambda) ---
    /// Base transmission rate for Ascaris lumbricoides
    pub ascaris_lambda: f32,
    /// Base transmission rate for Trichuris trichiura
    pub trichuris_lambda: f32,
    /// Base transmission rate for hookworm
    pub hookworm_lambda: f32,

    // --- Contamination dynamics ---
    /// Base decay rate for soil contamination per tick
    pub contamination_decay_rate: f32,
    /// Contamination deposited per open defecation event (normalized)
    pub open_defecation_contamination: f32,
    /// Probability of rain event per day
    pub rain_frequency: f32,
    /// How much rain spreads contamination (multiplier)
    pub rain_contamination_spread: f32,

    // --- Intervention costs (budget currency units) ---
    pub mda_cost_per_child: f32,
    pub latrine_cost: f32,
    pub water_pump_cost: f32,
    pub handwash_station_cost: f32,
    pub education_session_cost: f32,
    pub bhw_visit_cost_per_household: f32,

    // --- Budget ---
    /// Total starting budget
    pub total_budget: f32,
    /// Monthly budget increment
    pub monthly_budget_increment: f32,

    // --- Automatic MDA scheduling ---
    /// Automatic biannual MDA interval in days (None = manual only, Some(180) = every 6 months)
    pub auto_mda_interval_days: Option<u32>,
    /// Automatic MDA drug (0 = Albendazole, 1 = Mebendazole)
    pub auto_mda_drug: u8,
    /// Automatic MDA coverage (0.0-1.0)
    pub auto_mda_coverage: f32,

    // --- COVID disruption (optional scenario) ---
    /// Month when COVID school closure starts (None = no COVID)
    pub covid_school_closure_start: Option<u32>,
    /// Month when COVID school closure ends
    pub covid_school_closure_end: Option<u32>,

    // --- Auto-WASH placement at simulation start ---
    /// Number of latrines to auto-build at simulation start (0 = none)
    pub auto_latrines: u32,
    /// Number of water pumps to auto-build at simulation start (0 = none)
    pub auto_water_pumps: u32,
    /// Whether to auto-launch education at start (0 = no, 1 = yes)
    pub auto_education: u8,

    // --- Initial KAP overrides (None = use barangay defaults) ---
    /// Override initial knowledge for all children (0.0-1.0, None = use barangay default)
    pub initial_knowledge: Option<f32>,
    /// Override initial attitude for all children (0.0-1.0, None = use barangay default)
    pub initial_attitude: Option<f32>,
    /// Override initial practice for all children (0.0-1.0, None = use barangay default)
    pub initial_practice: Option<f32>,
}

impl Default for SthConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            world_width: 800.0,
            world_height: 600.0,
            barangays: vec![BarangayConfig::urban_default()],
            ticks_per_day: 24,

            // Transmission lambdas (calibrated to R0 ordering: Trichuris > Ascaris > Hookworm)
            ascaris_lambda: 0.0005,
            trichuris_lambda: 0.0006,
            hookworm_lambda: 0.00003,

            // Contamination dynamics
            contamination_decay_rate: 0.005,
            open_defecation_contamination: 0.01,
            rain_frequency: 0.3,
            rain_contamination_spread: 0.2,

            // Intervention costs (relative currency units)
            mda_cost_per_child: 1.0,
            latrine_cost: 50.0,
            water_pump_cost: 100.0,
            handwash_station_cost: 20.0,
            education_session_cost: 10.0,
            bhw_visit_cost_per_household: 2.0,

            // Budget
            total_budget: 1000.0,
            monthly_budget_increment: 50.0,

            // No automatic MDA by default
            auto_mda_interval_days: None,
            auto_mda_drug: 0,
            auto_mda_coverage: 0.75,

            // No COVID by default
            covid_school_closure_start: None,
            covid_school_closure_end: None,

            // No auto-WASH placement by default
            auto_latrines: 0,
            auto_water_pumps: 0,
            auto_education: 0,

            // No KAP overrides by default
            initial_knowledge: None,
            initial_attitude: None,
            initial_practice: None,
        }
    }
}

impl SthConfig {
    /// Validate configuration parameters and return an error if any are out of bounds.
    pub fn validate(&self) -> Result<(), String> {
        // World dimensions
        if !self.world_width.is_finite() || self.world_width < 100.0 || self.world_width > 10000.0 {
            return Err(format!(
                "world_width = {} is out of valid range [100.0, 10000.0]",
                self.world_width
            ));
        }
        if !self.world_height.is_finite() || self.world_height < 100.0 || self.world_height > 10000.0 {
            return Err(format!(
                "world_height = {} is out of valid range [100.0, 10000.0]",
                self.world_height
            ));
        }

        // Barangays
        if self.barangays.is_empty() || self.barangays.len() > 2 {
            return Err(format!(
                "barangays count = {} must be 1 or 2",
                self.barangays.len()
            ));
        }

        // Ticks per day
        if self.ticks_per_day != 24 {
            return Err(format!(
                "ticks_per_day = {} must be 24 (1 tick = 1 hour)",
                self.ticks_per_day
            ));
        }

        // Transmission lambdas
        if !self.ascaris_lambda.is_finite() || self.ascaris_lambda < 0.0 || self.ascaris_lambda > 1.0 {
            return Err(format!(
                "ascaris_lambda = {} is out of valid range [0.0, 1.0]",
                self.ascaris_lambda
            ));
        }
        if !self.trichuris_lambda.is_finite() || self.trichuris_lambda < 0.0 || self.trichuris_lambda > 1.0 {
            return Err(format!(
                "trichuris_lambda = {} is out of valid range [0.0, 1.0]",
                self.trichuris_lambda
            ));
        }
        if !self.hookworm_lambda.is_finite() || self.hookworm_lambda < 0.0 || self.hookworm_lambda > 1.0 {
            return Err(format!(
                "hookworm_lambda = {} is out of valid range [0.0, 1.0]",
                self.hookworm_lambda
            ));
        }

        // Contamination dynamics
        if !self.contamination_decay_rate.is_finite()
            || self.contamination_decay_rate < 0.0
            || self.contamination_decay_rate > 1.0
        {
            return Err(format!(
                "contamination_decay_rate = {} is out of valid range [0.0, 1.0]",
                self.contamination_decay_rate
            ));
        }
        if !self.open_defecation_contamination.is_finite()
            || self.open_defecation_contamination < 0.0
            || self.open_defecation_contamination > 1.0
        {
            return Err(format!(
                "open_defecation_contamination = {} is out of valid range [0.0, 1.0]",
                self.open_defecation_contamination
            ));
        }
        if !self.rain_frequency.is_finite() || self.rain_frequency < 0.0 || self.rain_frequency > 1.0 {
            return Err(format!(
                "rain_frequency = {} is out of valid range [0.0, 1.0]",
                self.rain_frequency
            ));
        }
        if !self.rain_contamination_spread.is_finite()
            || self.rain_contamination_spread < 0.0
            || self.rain_contamination_spread > 1.0
        {
            return Err(format!(
                "rain_contamination_spread = {} is out of valid range [0.0, 1.0]",
                self.rain_contamination_spread
            ));
        }

        // Costs (must be non-negative and finite)
        for (name, value) in [
            ("mda_cost_per_child", self.mda_cost_per_child),
            ("latrine_cost", self.latrine_cost),
            ("water_pump_cost", self.water_pump_cost),
            ("handwash_station_cost", self.handwash_station_cost),
            ("education_session_cost", self.education_session_cost),
            ("bhw_visit_cost_per_household", self.bhw_visit_cost_per_household),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{} = {} must be non-negative and finite", name, value));
            }
        }

        // Budget
        if !self.total_budget.is_finite() || self.total_budget < 0.0 {
            return Err(format!(
                "total_budget = {} must be non-negative and finite",
                self.total_budget
            ));
        }
        if !self.monthly_budget_increment.is_finite() || self.monthly_budget_increment < 0.0 {
            return Err(format!(
                "monthly_budget_increment = {} must be non-negative and finite",
                self.monthly_budget_increment
            ));
        }

        // Auto-MDA parameters
        if let Some(interval) = self.auto_mda_interval_days {
            if interval == 0 || interval > 365 {
                return Err(format!(
                    "auto_mda_interval_days = {} is out of valid range [1, 365]",
                    interval
                ));
            }
        }
        if self.auto_mda_drug > 1 {
            return Err(format!(
                "auto_mda_drug = {} must be 0 (Albendazole) or 1 (Mebendazole)",
                self.auto_mda_drug
            ));
        }
        if !self.auto_mda_coverage.is_finite()
            || self.auto_mda_coverage < 0.0
            || self.auto_mda_coverage > 1.0
        {
            return Err(format!(
                "auto_mda_coverage = {} is out of valid range [0.0, 1.0]",
                self.auto_mda_coverage
            ));
        }

        // COVID parameters cross-validation
        if let (Some(start), Some(end)) = (self.covid_school_closure_start, self.covid_school_closure_end) {
            if start >= end {
                return Err(format!(
                    "covid_school_closure_start ({}) must be before covid_school_closure_end ({})",
                    start, end
                ));
            }
        }
        if self.covid_school_closure_start.is_some() != self.covid_school_closure_end.is_some() {
            return Err(
                "covid_school_closure_start and covid_school_closure_end must both be set or both be None"
                    .to_string(),
            );
        }

        // Auto-WASH limits
        if self.auto_latrines > 100 {
            return Err(format!(
                "auto_latrines = {} exceeds maximum of 100",
                self.auto_latrines
            ));
        }
        if self.auto_water_pumps > 100 {
            return Err(format!(
                "auto_water_pumps = {} exceeds maximum of 100",
                self.auto_water_pumps
            ));
        }
        if self.auto_education > 1 {
            return Err(format!(
                "auto_education = {} must be 0 or 1",
                self.auto_education
            ));
        }

        // Initial KAP overrides
        for (name, value) in [
            ("initial_knowledge", self.initial_knowledge),
            ("initial_attitude", self.initial_attitude),
            ("initial_practice", self.initial_practice),
        ] {
            if let Some(v) = value {
                if !v.is_finite() || v < 0.0 || v > 1.0 {
                    return Err(format!(
                        "{} = {} is out of valid range [0.0, 1.0]",
                        name, v
                    ));
                }
            }
        }

        // Memory budget estimate
        let max_agents: usize = self.barangays.iter()
            .map(|b| b.child_population as usize + b.num_households as usize + b.num_schools as usize * 5 + b.num_health_workers as usize)
            .sum();
        let agent_size_bytes = 256; // approximate bytes per Agent struct
        let estimated_bytes = max_agents * agent_size_bytes
            + (self.world_width as usize / 10) * (self.world_height as usize / 10) * 32; // environment grid
        if estimated_bytes > 128_000_000 {
            return Err(format!(
                "Config exceeds memory budget: estimated {} MB > 128 MB",
                estimated_bytes / 1_000_000
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = SthConfig::default();
        assert!(config.validate().is_ok(), "Default config should be valid: {:?}", config.validate());
    }

    #[test]
    fn test_default_has_one_barangay() {
        let config = SthConfig::default();
        assert_eq!(config.barangays.len(), 1);
    }

    #[test]
    fn test_default_ticks_per_day() {
        let config = SthConfig::default();
        assert_eq!(config.ticks_per_day, 24);
    }

    #[test]
    fn test_invalid_world_width() {
        let mut config = SthConfig::default();
        config.world_width = 50.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_zero_barangays() {
        let mut config = SthConfig::default();
        config.barangays.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_three_barangays() {
        let mut config = SthConfig::default();
        config.barangays.push(BarangayConfig::rural_default());
        config.barangays.push(BarangayConfig::rural_default());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_ticks_per_day() {
        let mut config = SthConfig::default();
        config.ticks_per_day = 12;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_lambda_negative() {
        let mut config = SthConfig::default();
        config.ascaris_lambda = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_lambda_nan() {
        let mut config = SthConfig::default();
        config.ascaris_lambda = f32::NAN;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_negative_cost() {
        let mut config = SthConfig::default();
        config.mda_cost_per_child = -5.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_covid_params_must_pair() {
        let mut config = SthConfig::default();
        config.covid_school_closure_start = Some(6);
        config.covid_school_closure_end = None;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_covid_start_before_end() {
        let mut config = SthConfig::default();
        config.covid_school_closure_start = Some(10);
        config.covid_school_closure_end = Some(5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_valid_covid_params() {
        let mut config = SthConfig::default();
        config.covid_school_closure_start = Some(6);
        config.covid_school_closure_end = Some(18);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_comparison_mode_config() {
        let mut config = SthConfig::default();
        config.world_width = 1600.0;
        config.barangays = vec![
            BarangayConfig::urban_default(),
            BarangayConfig::rural_default(),
        ];
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_lambdas_match_calibration() {
        let config = SthConfig::default();
        assert_eq!(config.ascaris_lambda, 0.0005);
        assert_eq!(config.trichuris_lambda, 0.0006);
        assert_eq!(config.hookworm_lambda, 0.00003);
    }

    #[test]
    fn test_default_auto_mda_is_none() {
        let config = SthConfig::default();
        assert!(config.auto_mda_interval_days.is_none());
        assert_eq!(config.auto_mda_drug, 0);
        assert_eq!(config.auto_mda_coverage, 0.75);
    }

    #[test]
    fn test_valid_auto_mda_config() {
        let mut config = SthConfig::default();
        config.auto_mda_interval_days = Some(180);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_auto_mda_interval_zero() {
        let mut config = SthConfig::default();
        config.auto_mda_interval_days = Some(0);
        assert!(config.validate().is_err());
    }
}
