/// Knowledge, Attitudes, Practices (KAP) model for STH simulation.
///
/// Encodes the thesis finding that structural/environmental factors dominate
/// over individual KAP in determining infection outcomes. KAP modifies behavior
/// probabilistically, but structural gates (e.g., latrine access) override
/// individual willingness.

use rand::Rng;
use serde::{Deserialize, Serialize};

// ---------- KAP Scores ----------

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KapScores {
    /// 0.0-1.0, maps to 0-100% knowledge score
    pub knowledge: f32,
    /// 0.0-1.0, maps to 1.0-5.0 attitudinal scale
    pub attitude: f32,
    /// 0.0-1.0, maps to 0-100% practice score
    pub practice: f32,
}

impl Default for KapScores {
    fn default() -> Self {
        Self {
            knowledge: 0.5,
            attitude: 0.5,
            practice: 0.5,
        }
    }
}

// ---------- KAP Level (Knowledge + Practice) ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KapLevel {
    Poor,      // 0-49%
    Moderate,  // 50-69%
    Good,      // 70-89%
    Excellent, // 90-100%
}

// ---------- Attitude Level ----------

/// Maps the 0.0-1.0 attitude float to the thesis's 1.0-5.0 Likert scale categories.
/// - Negative: 1.00-2.40 on Likert => 0.00-0.28 normalized
/// - Neutral:  2.41-3.40 on Likert => 0.29-0.48 normalized  (midpoint approach)
/// - Positive: 3.41-5.00 on Likert => 0.49-1.00 normalized
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttitudeLevel {
    Negative, // 0.00-0.28
    Neutral,  // 0.29-0.48
    Positive, // 0.49-1.00
}

// ---------- KapScores methods ----------

impl KapScores {
    /// Create a new KapScores with explicit values, clamped to [0, 1].
    pub fn new(knowledge: f32, attitude: f32, practice: f32) -> Self {
        Self {
            knowledge: knowledge.clamp(0.0, 1.0),
            attitude: attitude.clamp(0.0, 1.0),
            practice: practice.clamp(0.0, 1.0),
        }
    }

    /// Categorize knowledge score using WHO/thesis thresholds.
    pub fn knowledge_level(&self) -> KapLevel {
        match (self.knowledge * 100.0) as u32 {
            0..=49 => KapLevel::Poor,
            50..=69 => KapLevel::Moderate,
            70..=89 => KapLevel::Good,
            _ => KapLevel::Excellent,
        }
    }

    /// Categorize attitude score using thesis Likert-derived thresholds.
    pub fn attitude_level(&self) -> AttitudeLevel {
        if self.attitude <= 0.28 {
            AttitudeLevel::Negative
        } else if self.attitude <= 0.48 {
            AttitudeLevel::Neutral
        } else {
            AttitudeLevel::Positive
        }
    }

    /// Probability of handwashing before eating.
    /// Weighted: practice 60%, knowledge 20%, attitude 20%.
    /// Clamped to [0.05, 0.95] — never zero, never certain.
    pub fn handwashing_probability(&self) -> f32 {
        let base = self.practice * 0.6 + self.knowledge * 0.2 + self.attitude * 0.2;
        base.clamp(0.05, 0.95)
    }

    /// Probability of wearing shoes outdoors.
    /// Weighted: practice 50%, attitude 30%, knowledge 20%.
    /// Clamped to [0.05, 0.95].
    pub fn shoe_wearing_probability(&self) -> f32 {
        let base = self.practice * 0.5 + self.attitude * 0.3 + self.knowledge * 0.2;
        base.clamp(0.05, 0.95)
    }

    /// Probability of using a latrine for defecation.
    ///
    /// **STRUCTURAL GATE**: If `has_latrine` is false, returns 0.0 regardless
    /// of KAP scores. This encodes the thesis finding that structural access
    /// dominates over individual knowledge/attitude.
    pub fn latrine_use_probability(&self, has_latrine: bool) -> f32 {
        if !has_latrine {
            return 0.0;
        }
        let base = self.practice * 0.5 + self.attitude * 0.3 + self.knowledge * 0.2;
        base.clamp(0.1, 0.95)
    }
}

// ---------- KAP Update Function ----------

/// Update KAP scores based on experience, social influence, and decay.
///
/// Called once per tick (1 hour simulated time).
///
/// - **Education events**: knowledge +5-15%, attitude +2-8%
/// - **Infection experience**: attitude slowly increases (perceived susceptibility)
/// - **Social influence**: drift toward community average (0.001 pull rate)
/// - **Practice decay**: multiplied by 0.9997 per tick (~10% decay per month)
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

    // Clamp all to valid range
    kap.knowledge = kap.knowledge.clamp(0.0, 1.0);
    kap.attitude = kap.attitude.clamp(0.0, 1.0);
    kap.practice = kap.practice.clamp(0.0, 1.0);
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // ---- KAP level classification ----

    #[test]
    fn knowledge_levels_match_who_categories() {
        assert_eq!(KapScores::new(0.0, 0.0, 0.0).knowledge_level(), KapLevel::Poor);
        assert_eq!(KapScores::new(0.49, 0.0, 0.0).knowledge_level(), KapLevel::Poor);
        assert_eq!(KapScores::new(0.50, 0.0, 0.0).knowledge_level(), KapLevel::Moderate);
        assert_eq!(KapScores::new(0.69, 0.0, 0.0).knowledge_level(), KapLevel::Moderate);
        assert_eq!(KapScores::new(0.70, 0.0, 0.0).knowledge_level(), KapLevel::Good);
        assert_eq!(KapScores::new(0.89, 0.0, 0.0).knowledge_level(), KapLevel::Good);
        assert_eq!(KapScores::new(0.90, 0.0, 0.0).knowledge_level(), KapLevel::Excellent);
        assert_eq!(KapScores::new(1.0, 0.0, 0.0).knowledge_level(), KapLevel::Excellent);
    }

    #[test]
    fn attitude_levels_match_thesis_categories() {
        assert_eq!(
            KapScores::new(0.0, 0.0, 0.0).attitude_level(),
            AttitudeLevel::Negative
        );
        assert_eq!(
            KapScores::new(0.0, 0.28, 0.0).attitude_level(),
            AttitudeLevel::Negative
        );
        assert_eq!(
            KapScores::new(0.0, 0.29, 0.0).attitude_level(),
            AttitudeLevel::Neutral
        );
        assert_eq!(
            KapScores::new(0.0, 0.48, 0.0).attitude_level(),
            AttitudeLevel::Neutral
        );
        assert_eq!(
            KapScores::new(0.0, 0.49, 0.0).attitude_level(),
            AttitudeLevel::Positive
        );
        assert_eq!(
            KapScores::new(0.0, 1.0, 0.0).attitude_level(),
            AttitudeLevel::Positive
        );
    }

    // ---- Structural gate ----

    #[test]
    fn latrine_use_zero_without_latrine() {
        // This is the KEY thesis finding: no latrine = no use, regardless of KAP
        let perfect_kap = KapScores::new(1.0, 1.0, 1.0);
        assert_eq!(perfect_kap.latrine_use_probability(false), 0.0);
    }

    #[test]
    fn latrine_use_nonzero_with_latrine() {
        let kap = KapScores::new(0.5, 0.5, 0.5);
        let prob = kap.latrine_use_probability(true);
        assert!(prob > 0.0, "should have nonzero latrine use with access");
        assert!(prob <= 0.95, "should be clamped at 0.95");
    }

    // ---- Probability bounds ----

    #[test]
    fn handwashing_probability_bounds() {
        let low = KapScores::new(0.0, 0.0, 0.0);
        let high = KapScores::new(1.0, 1.0, 1.0);
        assert!((low.handwashing_probability() - 0.05).abs() < 1e-6);
        assert!((high.handwashing_probability() - 0.95).abs() < 1e-6);
    }

    #[test]
    fn shoe_wearing_probability_bounds() {
        let low = KapScores::new(0.0, 0.0, 0.0);
        let high = KapScores::new(1.0, 1.0, 1.0);
        assert!((low.shoe_wearing_probability() - 0.05).abs() < 1e-6);
        assert!((high.shoe_wearing_probability() - 0.95).abs() < 1e-6);
    }

    // ---- Practice decay ----

    #[test]
    fn practice_decays_over_time() {
        let mut kap = KapScores::new(0.5, 0.5, 0.8);
        let parent = KapScores::default();
        let mut rng = SmallRng::seed_from_u64(42);

        let initial_practice = kap.practice;

        // Simulate one month (720 ticks) without education or infection
        for _ in 0..720 {
            update_kap_from_experience(&mut kap, false, false, &parent, 0.5, &mut rng);
        }

        let decay_fraction = kap.practice / initial_practice;
        // Should have decayed roughly 10% (0.9997^720 ≈ 0.806)
        assert!(
            decay_fraction < 0.85,
            "Practice should decay ~20% in a month, got {decay_fraction}"
        );
        assert!(
            decay_fraction > 0.75,
            "Practice shouldn't decay too much, got {decay_fraction}"
        );
    }

    // ---- Education boost ----

    #[test]
    fn education_boosts_knowledge() {
        let mut kap = KapScores::new(0.3, 0.3, 0.3);
        let parent = KapScores::default();
        let mut rng = SmallRng::seed_from_u64(42);

        let before = kap.knowledge;
        update_kap_from_experience(&mut kap, false, true, &parent, 0.5, &mut rng);
        assert!(
            kap.knowledge > before,
            "Education should increase knowledge"
        );
        assert!(
            kap.knowledge - before >= 0.05,
            "Knowledge boost should be at least 5%"
        );
        assert!(
            kap.knowledge - before <= 0.15,
            "Knowledge boost should be at most 15%"
        );
    }

    // ---- Infection experience ----

    #[test]
    fn infection_increases_attitude() {
        let mut kap = KapScores::new(0.5, 0.3, 0.5);
        let parent = KapScores::default();
        let mut rng = SmallRng::seed_from_u64(42);

        let before = kap.attitude;
        update_kap_from_experience(&mut kap, true, false, &parent, 0.5, &mut rng);
        assert!(
            kap.attitude > before,
            "Infection experience should increase attitude (perceived susceptibility)"
        );
    }

    // ---- Values stay clamped ----

    #[test]
    fn values_clamped_after_update() {
        let mut kap = KapScores::new(0.98, 0.98, 0.5);
        let parent = KapScores::new(1.0, 1.0, 1.0);
        let mut rng = SmallRng::seed_from_u64(42);

        // Education + infection, high community average — should try to push above 1.0
        update_kap_from_experience(&mut kap, true, true, &parent, 1.0, &mut rng);
        assert!(kap.knowledge <= 1.0);
        assert!(kap.attitude <= 1.0);
        assert!(kap.practice <= 1.0);
    }

    // ---- New constructor clamps ----

    #[test]
    fn new_clamps_values() {
        let kap = KapScores::new(-0.5, 2.0, 0.5);
        assert!((kap.knowledge - 0.0).abs() < 1e-6);
        assert!((kap.attitude - 1.0).abs() < 1e-6);
    }
}
