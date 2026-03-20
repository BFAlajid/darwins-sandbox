use serde::{Deserialize, Serialize};

/// Urban vs rural setting type, encoding the thesis finding that
/// urban informal settlements paradoxically have higher STH prevalence
/// than rural areas due to worse WASH infrastructure and higher density.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingType {
    /// Guadalupe/Tisa: high density, informal settlements, poor WASH
    Urban,
    /// Sudlon II/Guba: lower density, agricultural, better WASH paradoxically
    Rural,
}

/// Configuration for a single barangay (smallest administrative division).
///
/// All default values are calibrated from thesis data on school-aged
/// children in Cebu City, Philippines.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarangayConfig {
    pub setting: SettingType,
    pub name: String,
    pub child_population: u16,
    pub num_households: u16,
    pub num_schools: u8,
    pub num_health_workers: u8,

    // --- WASH infrastructure (thesis findings encoded) ---
    /// Fraction of households with latrine access (Urban: 0.60, Rural: 0.75)
    pub latrine_coverage: f32,
    /// Fraction of households with water supply (Urban: 0.50 intermittent, Rural: 0.70)
    pub water_supply_coverage: f32,
    /// Fraction of schools/locations with handwashing stations
    pub handwash_station_coverage: f32,

    // --- Environmental factors ---
    /// Persons per grid cell (proxy for crowding)
    pub population_density: f32,
    /// Fraction of population practicing open defecation (Urban: 0.30, Rural: 0.15)
    pub open_defecation_rate: f32,
    /// Initial soil contamination level (Urban: 0.08, Rural: 0.02)
    pub baseline_soil_contamination: f32,

    // --- Programmatic factors ---
    /// Medicine supply reliability (0.0=constant stockout, 1.0=always available)
    pub medicine_supply_reliability: f32,
    /// Barangay health workers per 1000 population
    pub bhw_per_thousand: f32,

    // --- Initial KAP distributions (from thesis survey data) ---
    /// Mean knowledge score (Urban: 0.55, Rural: 0.70)
    pub mean_knowledge: f32,
    /// Mean attitude score (Urban: 0.65, Rural: 0.70)
    pub mean_attitude: f32,
    /// Mean practice score (Urban: 0.50, Rural: 0.65)
    pub mean_practice: f32,
}

impl BarangayConfig {
    /// Create an urban barangay config with thesis-calibrated defaults.
    ///
    /// Models Guadalupe-type informal settlements: high density,
    /// poor WASH, higher contamination, lower KAP practice scores.
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
            baseline_soil_contamination: 0.08,
            medicine_supply_reliability: 0.70,
            bhw_per_thousand: 2.0,
            mean_knowledge: 0.55,
            mean_attitude: 0.65,
            mean_practice: 0.50,
        }
    }

    /// Create a rural barangay config with thesis-calibrated defaults.
    ///
    /// Models Sudlon II-type rural areas: lower density, better WASH
    /// coverage, lower contamination, higher KAP scores.
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
            baseline_soil_contamination: 0.02,
            medicine_supply_reliability: 0.80,
            bhw_per_thousand: 3.0,
            mean_knowledge: 0.70,
            mean_attitude: 0.70,
            mean_practice: 0.65,
        }
    }

    /// Validate the barangay configuration parameters.
    pub fn validate(&self) -> Result<(), String> {
        if self.child_population == 0 {
            return Err("child_population must be > 0".to_string());
        }
        if self.num_households == 0 {
            return Err("num_households must be > 0".to_string());
        }
        if self.num_schools == 0 {
            return Err("num_schools must be > 0".to_string());
        }

        // Coverage fractions must be in [0.0, 1.0]
        for (name, value) in [
            ("latrine_coverage", self.latrine_coverage),
            ("water_supply_coverage", self.water_supply_coverage),
            ("handwash_station_coverage", self.handwash_station_coverage),
            ("open_defecation_rate", self.open_defecation_rate),
            ("baseline_soil_contamination", self.baseline_soil_contamination),
            ("medicine_supply_reliability", self.medicine_supply_reliability),
            ("mean_knowledge", self.mean_knowledge),
            ("mean_attitude", self.mean_attitude),
            ("mean_practice", self.mean_practice),
        ] {
            if !value.is_finite() || value < 0.0 || value > 1.0 {
                return Err(format!(
                    "{} = {} is out of valid range [0.0, 1.0]",
                    name, value
                ));
            }
        }

        if !self.population_density.is_finite() || self.population_density <= 0.0 {
            return Err(format!(
                "population_density = {} must be positive and finite",
                self.population_density
            ));
        }
        if !self.bhw_per_thousand.is_finite() || self.bhw_per_thousand < 0.0 {
            return Err(format!(
                "bhw_per_thousand = {} must be non-negative and finite",
                self.bhw_per_thousand
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urban_default_valid() {
        let config = BarangayConfig::urban_default();
        assert!(config.validate().is_ok());
        assert_eq!(config.setting, SettingType::Urban);
        assert_eq!(config.name, "Guadalupe");
    }

    #[test]
    fn test_rural_default_valid() {
        let config = BarangayConfig::rural_default();
        assert!(config.validate().is_ok());
        assert_eq!(config.setting, SettingType::Rural);
        assert_eq!(config.name, "Sudlon II");
    }

    #[test]
    fn test_urban_has_higher_contamination() {
        let urban = BarangayConfig::urban_default();
        let rural = BarangayConfig::rural_default();
        assert!(
            urban.baseline_soil_contamination > rural.baseline_soil_contamination,
            "Urban should have higher baseline contamination"
        );
    }

    #[test]
    fn test_urban_has_worse_wash() {
        let urban = BarangayConfig::urban_default();
        let rural = BarangayConfig::rural_default();
        assert!(urban.latrine_coverage < rural.latrine_coverage);
        assert!(urban.water_supply_coverage < rural.water_supply_coverage);
        assert!(urban.handwash_station_coverage < rural.handwash_station_coverage);
    }

    #[test]
    fn test_urban_has_higher_open_defecation() {
        let urban = BarangayConfig::urban_default();
        let rural = BarangayConfig::rural_default();
        assert!(urban.open_defecation_rate > rural.open_defecation_rate);
    }

    #[test]
    fn test_urban_has_lower_kap() {
        let urban = BarangayConfig::urban_default();
        let rural = BarangayConfig::rural_default();
        assert!(urban.mean_knowledge < rural.mean_knowledge);
        assert!(urban.mean_practice < rural.mean_practice);
    }

    #[test]
    fn test_urban_has_higher_density() {
        let urban = BarangayConfig::urban_default();
        let rural = BarangayConfig::rural_default();
        assert!(urban.population_density > rural.population_density);
    }

    #[test]
    fn test_invalid_zero_population() {
        let mut config = BarangayConfig::urban_default();
        config.child_population = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_zero_households() {
        let mut config = BarangayConfig::urban_default();
        config.num_households = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_zero_schools() {
        let mut config = BarangayConfig::urban_default();
        config.num_schools = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_coverage_out_of_range() {
        let mut config = BarangayConfig::urban_default();
        config.latrine_coverage = 1.5;
        assert!(config.validate().is_err());

        config.latrine_coverage = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_nan_coverage() {
        let mut config = BarangayConfig::urban_default();
        config.mean_knowledge = f32::NAN;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_negative_density() {
        let mut config = BarangayConfig::urban_default();
        config.population_density = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_setting_type_equality() {
        assert_eq!(SettingType::Urban, SettingType::Urban);
        assert_ne!(SettingType::Urban, SettingType::Rural);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let config = BarangayConfig::urban_default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BarangayConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.setting, config.setting);
        assert_eq!(deserialized.child_population, config.child_population);
    }
}
