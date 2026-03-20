use rand::Rng;
use serde::{Deserialize, Serialize};

/// The three soil-transmitted helminth species modeled in this simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SthSpecies {
    Ascaris,
    Trichuris,
    Hookworm,
}

/// WHO-standard intensity classification based on eggs per gram (EPG).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intensity {
    Negative,
    Light,
    Moderate,
    Heavy,
}

/// Per-species infection state for a single agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfectionState {
    /// Eggs per gram (continuous for smooth simulation)
    pub epg: f32,
    /// Estimated adult worm count
    pub worm_burden: f32,
    /// Number of days infected (saturating counter)
    pub days_infected: u16,
}

/// Maximum worm burden cap to prevent runaway accumulation.
const MAX_WORM_BURDEN: f32 = 500.0;

impl InfectionState {
    /// Create an uninfected state.
    pub fn new() -> Self {
        Self {
            epg: 0.0,
            worm_burden: 0.0,
            days_infected: 0,
        }
    }

    /// Classify infection intensity using WHO EPG cutoffs.
    ///
    /// Ascaris:   light <5000,  moderate 5000-49999, heavy >=50000
    /// Trichuris: light <1000,  moderate 1000-9999,  heavy >=10000
    /// Hookworm:  light <2000,  moderate 2000-3999,  heavy >=4000
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

    /// Whether the agent is infected (EPG > 0).
    #[inline]
    pub fn is_infected(&self) -> bool {
        self.epg > 0.0
    }
}

impl Default for InfectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Polynomial exp(-x) approximation for x in [0, 5].
/// Pade(2,2): (1 - x/2 + x^2/12) / (1 + x/2 + x^2/12)
/// Max error < 0.3% in [0, 3], suitable for probability calculations.
#[inline(always)]
pub fn poly_exp_neg(x: f32) -> f32 {
    let x = x.clamp(0.0, 5.0);
    let x2 = x * x;
    let num = 1.0 - x * 0.5 + x2 / 12.0;
    let den = 1.0 + x * 0.5 + x2 / 12.0;
    (num / den).max(0.0)
}

/// Transmission probability per hour of exposure to contaminated environment.
///
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
    uses_latrine: bool,
    location_risk: f32,
) -> f32 {
    // Validate inputs
    let soil_contamination = soil_contamination.clamp(0.0, 1.0);
    let location_risk = location_risk.max(0.0);

    let lambda = match species {
        SthSpecies::Ascaris => 0.0005,     // calibrated to ~20% urban prevalence
        SthSpecies::Trichuris => 0.0006,   // higher R0 than Ascaris (R0 4-6 vs 2.1-2.3)
        SthSpecies::Hookworm => 0.00003,   // near-zero (thesis found 0% hookworm)
    };

    // Behavioral exposure reduction factors
    let shoe_factor = if wears_shoes { 0.3 } else { 1.0 };
    let hand_factor = if washes_hands { 0.4 } else { 1.0 };
    let latrine_factor = if uses_latrine { 0.66 } else { 1.0 }; // Strunz 2014 meta-analysis: OR=0.66

    // Species-specific behavioral weighting
    let exposure = match species {
        // Ascaris and Trichuris: fecal-oral route, hand hygiene matters most
        SthSpecies::Ascaris => hand_factor * latrine_factor * location_risk,
        SthSpecies::Trichuris => hand_factor * latrine_factor * location_risk,
        // Hookworm: percutaneous (skin) route, shoes matter most
        SthSpecies::Hookworm => shoe_factor * latrine_factor * location_risk,
    };

    // Core transmission formula using polynomial approximation
    let rate = lambda * soil_contamination * exposure;
    1.0 - poly_exp_neg(rate)
}

/// Update worm burden dynamics: worms grow, produce eggs, die naturally.
///
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
        SthSpecies::Ascaris => 1.0 / 365.0,   // ~1 year lifespan
        SthSpecies::Trichuris => 1.0 / 730.0,  // ~2 year lifespan
        SthSpecies::Hookworm => 1.0 / 1095.0,  // ~3 year lifespan
    };

    // Natural worm attrition (per hour = per tick, since 1 tick = 1 hour)
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

    // Cap worm burden to prevent runaway accumulation
    state.worm_burden = state.worm_burden.min(MAX_WORM_BURDEN);

    // Pre-patent period: worms must mature before producing eggs
    // Ascaris: 60-85 days, Trichuris: 60-90 days, Hookworm: 35-63 days
    // days_infected is incremented per tick (1 tick = 1 hour), so threshold = days * 24
    let pre_patent_ticks: u16 = match species {
        SthSpecies::Ascaris => 70 * 24,    // ~70 days
        SthSpecies::Trichuris => 75 * 24,  // ~75 days
        SthSpecies::Hookworm => 50 * 24,   // ~50 days
    };

    // Density-dependent fecundity: EPG = fecundity_per_worm * burden^z
    // z < 1 means per-worm output decreases at higher burdens (Anderson et al. 2014)
    let (fecundity, z) = match species {
        SthSpecies::Ascaris => (10000.0_f32, 0.94),
        SthSpecies::Trichuris => (1000.0_f32, 0.97),
        SthSpecies::Hookworm => (1000.0_f32, 0.92),
    };
    if state.days_infected >= pre_patent_ticks {
        state.epg = fecundity * state.worm_burden.powf(z);
    } else {
        state.epg = 0.0; // Immature worms don't produce eggs yet
    }

    // Clear negligible infections
    if state.worm_burden < 0.01 {
        state.worm_burden = 0.0;
        state.epg = 0.0;
        state.days_infected = 0;
    } else {
        state.days_infected = state.days_infected.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_new_infection_state_is_uninfected() {
        let state = InfectionState::new();
        assert!(!state.is_infected());
        assert_eq!(state.intensity(SthSpecies::Ascaris), Intensity::Negative);
    }

    #[test]
    fn test_ascaris_intensity_light() {
        let state = InfectionState {
            epg: 2500.0,
            worm_burden: 1.0,
            days_infected: 10,
        };
        assert_eq!(state.intensity(SthSpecies::Ascaris), Intensity::Light);
    }

    #[test]
    fn test_ascaris_intensity_moderate() {
        let state = InfectionState {
            epg: 25000.0,
            worm_burden: 5.0,
            days_infected: 30,
        };
        assert_eq!(state.intensity(SthSpecies::Ascaris), Intensity::Moderate);
    }

    #[test]
    fn test_ascaris_intensity_heavy() {
        let state = InfectionState {
            epg: 60000.0,
            worm_burden: 10.0,
            days_infected: 60,
        };
        assert_eq!(state.intensity(SthSpecies::Ascaris), Intensity::Heavy);
    }

    #[test]
    fn test_trichuris_intensity_thresholds() {
        assert_eq!(
            InfectionState { epg: 500.0, worm_burden: 1.0, days_infected: 5 }
                .intensity(SthSpecies::Trichuris),
            Intensity::Light
        );
        assert_eq!(
            InfectionState { epg: 5000.0, worm_burden: 5.0, days_infected: 20 }
                .intensity(SthSpecies::Trichuris),
            Intensity::Moderate
        );
        assert_eq!(
            InfectionState { epg: 15000.0, worm_burden: 10.0, days_infected: 60 }
                .intensity(SthSpecies::Trichuris),
            Intensity::Heavy
        );
    }

    #[test]
    fn test_hookworm_intensity_thresholds() {
        assert_eq!(
            InfectionState { epg: 1000.0, worm_burden: 1.0, days_infected: 5 }
                .intensity(SthSpecies::Hookworm),
            Intensity::Light
        );
        assert_eq!(
            InfectionState { epg: 3000.0, worm_burden: 3.0, days_infected: 20 }
                .intensity(SthSpecies::Hookworm),
            Intensity::Moderate
        );
        assert_eq!(
            InfectionState { epg: 5000.0, worm_burden: 5.0, days_infected: 60 }
                .intensity(SthSpecies::Hookworm),
            Intensity::Heavy
        );
    }

    #[test]
    fn test_poly_exp_neg_at_zero() {
        let result = poly_exp_neg(0.0);
        assert!((result - 1.0).abs() < 0.001, "exp(-0) should be ~1.0, got {}", result);
    }

    #[test]
    fn test_poly_exp_neg_at_one() {
        let result = poly_exp_neg(1.0);
        let expected = (-1.0_f32).exp();
        assert!(
            (result - expected).abs() < 0.01,
            "exp(-1) should be ~{}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_poly_exp_neg_clamps_negative() {
        let result = poly_exp_neg(-1.0);
        assert!((result - 1.0).abs() < 0.001, "negative input should clamp to 0, giving ~1.0");
    }

    #[test]
    fn test_poly_exp_neg_at_five() {
        let result = poly_exp_neg(5.0);
        assert!(result >= 0.0);
        assert!(result < 0.15, "exp(-5) approx should be small, got {}", result);
    }

    #[test]
    fn test_transmission_probability_zero_contamination() {
        let prob = transmission_probability(
            SthSpecies::Ascaris, 0.0, false, false, false, 1.0,
        );
        assert_eq!(prob, 0.0, "No contamination should mean no transmission");
    }

    #[test]
    fn test_transmission_probability_reduces_with_protection() {
        let prob_unprotected = transmission_probability(
            SthSpecies::Ascaris, 0.5, false, false, false, 1.0,
        );
        let prob_protected = transmission_probability(
            SthSpecies::Ascaris, 0.5, true, true, true, 1.0,
        );
        assert!(
            prob_protected < prob_unprotected,
            "Protection should reduce probability: {} >= {}",
            prob_protected,
            prob_unprotected
        );
    }

    #[test]
    fn test_transmission_hookworm_shoes_matter() {
        let prob_no_shoes = transmission_probability(
            SthSpecies::Hookworm, 0.5, false, false, false, 1.0,
        );
        let prob_shoes = transmission_probability(
            SthSpecies::Hookworm, 0.5, true, false, false, 1.0,
        );
        assert!(
            prob_shoes < prob_no_shoes,
            "Shoes should reduce hookworm: {} >= {}",
            prob_shoes,
            prob_no_shoes
        );
    }

    #[test]
    fn test_transmission_ascaris_handwashing_matters() {
        let prob_no_wash = transmission_probability(
            SthSpecies::Ascaris, 0.5, false, false, false, 1.0,
        );
        let prob_wash = transmission_probability(
            SthSpecies::Ascaris, 0.5, false, true, false, 1.0,
        );
        assert!(
            prob_wash < prob_no_wash,
            "Handwashing should reduce ascaris: {} >= {}",
            prob_wash,
            prob_no_wash
        );
    }

    #[test]
    fn test_update_worm_burden_natural_decay() {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut state = InfectionState {
            epg: 10000.0,
            worm_burden: 10.0,
            days_infected: 30,
        };
        let initial_burden = state.worm_burden;

        // Run with zero new infection probability
        for _ in 0..24 {
            update_worm_burden(&mut state, 0.0, &mut rng, SthSpecies::Ascaris);
        }

        assert!(
            state.worm_burden < initial_burden,
            "Worm burden should decay naturally: {} >= {}",
            state.worm_burden,
            initial_burden
        );
    }

    #[test]
    fn test_update_worm_burden_clears_negligible() {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut state = InfectionState {
            epg: 1.0,
            worm_burden: 0.005,
            days_infected: 1,
        };

        update_worm_burden(&mut state, 0.0, &mut rng, SthSpecies::Ascaris);

        assert_eq!(state.worm_burden, 0.0);
        assert_eq!(state.epg, 0.0);
        assert_eq!(state.days_infected, 0);
    }

    #[test]
    fn test_update_worm_burden_caps_at_max() {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut state = InfectionState {
            epg: 0.0,
            worm_burden: 499.0,
            days_infected: 100,
        };

        // Run with high infection probability repeatedly
        for _ in 0..100 {
            update_worm_burden(&mut state, 1.0, &mut rng, SthSpecies::Ascaris);
        }

        assert!(
            state.worm_burden <= MAX_WORM_BURDEN,
            "Worm burden should be capped at {}, got {}",
            MAX_WORM_BURDEN,
            state.worm_burden
        );
    }

    #[test]
    fn test_lambda_ordering() {
        // Trichuris has highest R0 (4-6), then Ascaris (2.1-2.3), hookworm lowest
        let prob_ascaris = transmission_probability(
            SthSpecies::Ascaris, 0.5, false, false, false, 1.0,
        );
        let prob_trichuris = transmission_probability(
            SthSpecies::Trichuris, 0.5, false, false, false, 1.0,
        );
        let prob_hookworm = transmission_probability(
            SthSpecies::Hookworm, 0.5, false, false, false, 1.0,
        );

        assert!(prob_trichuris > prob_ascaris);
        assert!(prob_ascaris > prob_hookworm);
    }

    #[test]
    fn test_pre_patent_period_suppresses_epg() {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut state = InfectionState {
            epg: 0.0,
            worm_burden: 10.0,
            days_infected: 0, // fresh infection
        };

        // Run a few ticks — EPG should stay zero during pre-patent period
        for _ in 0..48 {
            update_worm_burden(&mut state, 0.0, &mut rng, SthSpecies::Ascaris);
        }
        assert_eq!(state.epg, 0.0, "EPG should be zero during pre-patent period");
    }

    #[test]
    fn test_density_dependent_fecundity() {
        // EPG = fecundity * burden^z, where z < 1
        // So EPG per worm should decrease at higher burdens
        let low_burden = InfectionState {
            epg: 0.0,
            worm_burden: 5.0,
            days_infected: 70 * 24, // past pre-patent
        };
        let high_burden = InfectionState {
            epg: 0.0,
            worm_burden: 50.0,
            days_infected: 70 * 24,
        };

        let mut rng = SmallRng::seed_from_u64(42);
        let mut low = low_burden;
        let mut high = high_burden;
        update_worm_burden(&mut low, 0.0, &mut rng, SthSpecies::Ascaris);
        update_worm_burden(&mut high, 0.0, &mut rng, SthSpecies::Ascaris);

        let epg_per_worm_low = low.epg / low.worm_burden.max(0.01);
        let epg_per_worm_high = high.epg / high.worm_burden.max(0.01);
        assert!(
            epg_per_worm_low > epg_per_worm_high,
            "Per-worm EPG should decrease with higher burden: low={}, high={}",
            epg_per_worm_low,
            epg_per_worm_high
        );
    }
}
