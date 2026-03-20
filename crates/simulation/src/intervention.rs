//! Intervention system for STH simulation.
//!
//! Implements Mass Drug Administration (MDA), WASH infrastructure improvements,
//! health education campaigns, BHW house-to-house visits, and policy budget changes.
//! Based on published meta-analysis drug efficacy data and WHO deworming guidelines.

use rand::Rng;
use serde::{Deserialize, Serialize};
use slotmap::DenseSlotMap;

use crate::agent::{Agent, AgentKey, AgentType};
use crate::infection::SthSpecies;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Anthelmintic drug options for MDA campaigns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drug {
    Albendazole,
    Mebendazole,
}

/// WASH infrastructure improvement types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WashType {
    Latrine,
    WaterPump,
    HandwashStation,
}

/// Health education delivery methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EducationMethod {
    /// Visual learning (videos/posters)
    Cartoon,
    /// Interactive board game
    BoardGame,
    /// Classroom session led by teacher
    TeacherLed,
    /// Community parent meeting
    ParentMeeting,
}

// ---------------------------------------------------------------------------
// Intervention types
// ---------------------------------------------------------------------------

/// All available intervention types with their parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterventionType {
    /// Mass Drug Administration: school-based biannual deworming.
    MDA {
        /// Target a specific school (None = all schools).
        target_school: Option<u8>,
        /// Drug to administer.
        drug: Drug,
        /// Intended coverage fraction (0.0-1.0), depends on BHW availability.
        coverage_pct: f32,
    },
    /// WASH infrastructure improvement at a location.
    WashImprovement {
        /// Type of improvement to install.
        improvement: WashType,
        /// X coordinate of the installation site.
        x: f32,
        /// Y coordinate of the installation site.
        y: f32,
    },
    /// Health education campaign.
    Education {
        /// Delivery method.
        method: EducationMethod,
        /// Target a specific school (None = all schools).
        target_school: Option<u8>,
    },
    /// Barangay Health Worker house-to-house visits.
    BhwVisits {
        /// Coverage fraction (0.0-1.0).
        coverage_pct: f32,
        /// Campaign duration in days.
        duration_days: u16,
    },
    /// Policy: increase health budget (improves medicine supply, staffing).
    PolicyBudgetIncrease {
        /// Budget multiplier (e.g., 1.5 = 50% increase).
        multiplier: f32,
    },
}

// ---------------------------------------------------------------------------
// Active intervention tracking
// ---------------------------------------------------------------------------

/// Tracks a currently active (in-progress or completed) intervention.
#[derive(Clone, Debug)]
pub struct ActiveIntervention {
    /// The intervention being executed.
    pub intervention: InterventionType,
    /// Tick when the intervention started.
    pub start_tick: u64,
    /// Total duration in ticks.
    pub duration_ticks: u64,
    /// Monetary cost of the intervention.
    pub cost: f32,
    /// Progress fraction (0.0 = just started, 1.0 = complete).
    pub progress: f32,
}

// ---------------------------------------------------------------------------
// Drug efficacy
// ---------------------------------------------------------------------------

/// Returns the cure probability for a given drug-species combination.
///
/// Values are drawn from published meta-analyses:
/// - Albendazole: Ascaris 95%, Trichuris 30%, Hookworm 72%
/// - Mebendazole: Ascaris 96%, Trichuris 42%, Hookworm 15%
pub fn drug_efficacy(drug: Drug, species: SthSpecies) -> f32 {
    match (drug, species) {
        (Drug::Albendazole, SthSpecies::Ascaris) => 0.95,
        (Drug::Albendazole, SthSpecies::Trichuris) => 0.30,
        (Drug::Albendazole, SthSpecies::Hookworm) => 0.72,
        (Drug::Mebendazole, SthSpecies::Ascaris) => 0.96,
        (Drug::Mebendazole, SthSpecies::Trichuris) => 0.42,
        (Drug::Mebendazole, SthSpecies::Hookworm) => 0.15,
    }
}

// ---------------------------------------------------------------------------
// MDA
// ---------------------------------------------------------------------------

/// Apply Mass Drug Administration to eligible children.
///
/// Iterates child agents, optionally filtering by `target_school`.
/// Each child is treated with probability `coverage_pct * medicine_supply`.
/// For treated children, each species' infection is cleared independently
/// based on `drug_efficacy`. Resets `days_since_last_deworming` on treatment.
///
/// Returns the number of children treated.
pub fn apply_mda(
    agents: &mut DenseSlotMap<AgentKey, Agent>,
    drug: Drug,
    target_school: Option<u8>,
    coverage_pct: f32,
    medicine_supply: f32,
    rng: &mut impl Rng,
) -> u32 {
    let coverage_pct = coverage_pct.clamp(0.0, 1.0);
    let medicine_supply = medicine_supply.clamp(0.0, 1.0);
    let effective_coverage = coverage_pct * medicine_supply;
    let mut treated = 0u32;

    for (_key, agent) in agents.iter_mut() {
        if agent.agent_type != AgentType::Child {
            continue;
        }
        if let Some(school) = target_school {
            if agent.school_id != school {
                continue;
            }
        }
        if rng.gen::<f32>() > effective_coverage {
            continue;
        }

        // Apply drug effect per species
        for (species, state) in [
            (SthSpecies::Ascaris, &mut agent.ascaris),
            (SthSpecies::Trichuris, &mut agent.trichuris),
            (SthSpecies::Hookworm, &mut agent.hookworm),
        ] {
            if state.is_infected() && rng.gen::<f32>() < drug_efficacy(drug, species) {
                state.worm_burden = 0.0;
                state.epg = 0.0;
            }
        }
        agent.days_since_last_deworming = 0;
        treated += 1;
    }
    treated
}

// ---------------------------------------------------------------------------
// Education
// ---------------------------------------------------------------------------

/// Apply a health education campaign.
///
/// Iterates children (and parents for `ParentMeeting`), optionally filtering
/// by `target_school`. Boosts KAP scores based on method:
///
/// - Cartoon: knowledge +0.05-0.15
/// - BoardGame: knowledge +0.05-0.15, practice +0.05-0.10
/// - TeacherLed: knowledge +0.05-0.12, attitude +0.02-0.05
/// - ParentMeeting: affects parents, knowledge +0.03-0.08, attitude +0.02-0.05
///
/// Returns the number of agents affected.
pub fn apply_education(
    agents: &mut DenseSlotMap<AgentKey, Agent>,
    method: EducationMethod,
    target_school: Option<u8>,
    rng: &mut impl Rng,
) -> u32 {
    let mut affected = 0u32;

    for (_key, agent) in agents.iter_mut() {
        match method {
            EducationMethod::ParentMeeting => {
                // Parent meetings affect parents only
                if agent.agent_type != AgentType::Parent {
                    continue;
                }
                if let Some(school) = target_school {
                    if agent.school_id != school {
                        continue;
                    }
                }
                agent.knowledge = (agent.knowledge + rng.gen_range(0.03..=0.08)).min(1.0);
                agent.attitude = (agent.attitude + rng.gen_range(0.02..=0.05)).min(1.0);
                affected += 1;
            }
            _ => {
                // Cartoon, BoardGame, TeacherLed affect children
                if agent.agent_type != AgentType::Child {
                    continue;
                }
                if let Some(school) = target_school {
                    if agent.school_id != school {
                        continue;
                    }
                }
                match method {
                    EducationMethod::Cartoon => {
                        agent.knowledge =
                            (agent.knowledge + rng.gen_range(0.05..=0.15)).min(1.0);
                    }
                    EducationMethod::BoardGame => {
                        agent.knowledge =
                            (agent.knowledge + rng.gen_range(0.05..=0.15)).min(1.0);
                        agent.practice =
                            (agent.practice + rng.gen_range(0.05..=0.10)).min(1.0);
                    }
                    EducationMethod::TeacherLed => {
                        agent.knowledge =
                            (agent.knowledge + rng.gen_range(0.05..=0.12)).min(1.0);
                        agent.attitude =
                            (agent.attitude + rng.gen_range(0.02..=0.05)).min(1.0);
                    }
                    EducationMethod::ParentMeeting => unreachable!(),
                }
                affected += 1;
            }
        }
    }
    affected
}

// ---------------------------------------------------------------------------
// BHW Visits
// ---------------------------------------------------------------------------

/// Apply Barangay Health Worker house-to-house visits.
///
/// Iterates all agents (house-to-house). Each agent is visited with
/// probability `coverage_pct`. Visited agents receive:
/// - knowledge +0.03-0.08
/// - attitude +0.02-0.05
///
/// Returns the number of agents visited.
pub fn apply_bhw_visits(
    agents: &mut DenseSlotMap<AgentKey, Agent>,
    coverage_pct: f32,
    rng: &mut impl Rng,
) -> u32 {
    let coverage_pct = coverage_pct.clamp(0.0, 1.0);
    let mut visited = 0u32;

    for (_key, agent) in agents.iter_mut() {
        if rng.gen::<f32>() > coverage_pct {
            continue;
        }
        agent.knowledge = (agent.knowledge + rng.gen_range(0.03..=0.08)).min(1.0);
        agent.attitude = (agent.attitude + rng.gen_range(0.02..=0.05)).min(1.0);
        visited += 1;
    }
    visited
}

// ---------------------------------------------------------------------------
// Cost calculation
// ---------------------------------------------------------------------------

/// Calculate the monetary cost (USD) of an intervention.
///
/// - MDA: $5.00 per child
/// - Latrine: $500.00 per installation
/// - WaterPump: $800.00 per installation
/// - HandwashStation: $200.00 per installation
/// - Education (Cartoon/BoardGame): $100.00 per session
/// - Education (TeacherLed/ParentMeeting): $150.00 per session
/// - BhwVisits: $20.00 per household * coverage
/// - PolicyBudgetIncrease: $0.00 (administrative action)
pub fn intervention_cost(
    intervention: &InterventionType,
    child_count: u16,
    household_count: u16,
) -> f32 {
    match intervention {
        InterventionType::MDA { .. } => 5.0 * child_count as f32,
        InterventionType::WashImprovement { improvement, .. } => match improvement {
            WashType::Latrine => 500.0,
            WashType::WaterPump => 800.0,
            WashType::HandwashStation => 200.0,
        },
        InterventionType::Education { method, .. } => match method {
            EducationMethod::Cartoon | EducationMethod::BoardGame => 100.0,
            EducationMethod::TeacherLed | EducationMethod::ParentMeeting => 150.0,
        },
        InterventionType::BhwVisits { coverage_pct, .. } => {
            let coverage = coverage_pct.clamp(0.0, 1.0);
            20.0 * household_count as f32 * coverage
        }
        InterventionType::PolicyBudgetIncrease { .. } => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Duration calculation
// ---------------------------------------------------------------------------

/// Calculate the duration (in ticks = hours) of an intervention.
///
/// - MDA (single school): 24 ticks (1 day)
/// - MDA (all schools): 72 ticks (3 days)
/// - Education: 24 ticks (1 day)
/// - BhwVisits: duration_days * 24 ticks
/// - WashImprovement: 0 (instant)
/// - PolicyBudgetIncrease: 0 (permanent, instant effect)
pub fn intervention_duration_ticks(intervention: &InterventionType) -> u64 {
    match intervention {
        InterventionType::MDA { target_school, .. } => {
            if target_school.is_some() {
                24 // 1 day for a single school
            } else {
                72 // 3 days for all schools
            }
        }
        InterventionType::Education { .. } => 24,
        InterventionType::BhwVisits { duration_days, .. } => *duration_days as u64 * 24,
        InterventionType::WashImprovement { .. } => 0,
        InterventionType::PolicyBudgetIncrease { .. } => 0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::infection::InfectionState;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    // --- Drug efficacy tests ---

    #[test]
    fn test_albendazole_efficacy_ascaris() {
        assert!((drug_efficacy(Drug::Albendazole, SthSpecies::Ascaris) - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_albendazole_efficacy_trichuris() {
        assert!((drug_efficacy(Drug::Albendazole, SthSpecies::Trichuris) - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn test_albendazole_efficacy_hookworm() {
        assert!((drug_efficacy(Drug::Albendazole, SthSpecies::Hookworm) - 0.72).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mebendazole_efficacy_ascaris() {
        assert!((drug_efficacy(Drug::Mebendazole, SthSpecies::Ascaris) - 0.96).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mebendazole_efficacy_trichuris() {
        assert!((drug_efficacy(Drug::Mebendazole, SthSpecies::Trichuris) - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mebendazole_efficacy_hookworm() {
        assert!((drug_efficacy(Drug::Mebendazole, SthSpecies::Hookworm) - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn test_albendazole_best_for_ascaris() {
        // Albendazole and Mebendazole both highly effective for Ascaris
        assert!(drug_efficacy(Drug::Albendazole, SthSpecies::Ascaris) > 0.90);
        assert!(drug_efficacy(Drug::Mebendazole, SthSpecies::Ascaris) > 0.90);
    }

    #[test]
    fn test_mebendazole_better_for_trichuris() {
        assert!(
            drug_efficacy(Drug::Mebendazole, SthSpecies::Trichuris)
                > drug_efficacy(Drug::Albendazole, SthSpecies::Trichuris)
        );
    }

    #[test]
    fn test_albendazole_better_for_hookworm() {
        assert!(
            drug_efficacy(Drug::Albendazole, SthSpecies::Hookworm)
                > drug_efficacy(Drug::Mebendazole, SthSpecies::Hookworm)
        );
    }

    // --- Helper to create test agents ---

    fn make_infected_child(school_id: u8, household_id: u16) -> Agent {
        let mut agent = Agent::new_child(
            household_id, school_id, 0, 8,
            0.0, 0.0,
            0.3, 0.3, 0.3,
            true, true, true,
        );
        // Infect with all three species
        agent.ascaris = InfectionState {
            epg: 5000.0,
            worm_burden: 10.0,
            days_infected: 30,
        };
        agent.trichuris = InfectionState {
            epg: 2000.0,
            worm_burden: 5.0,
            days_infected: 20,
        };
        agent.hookworm = InfectionState {
            epg: 1000.0,
            worm_burden: 3.0,
            days_infected: 15,
        };
        agent.days_since_last_deworming = 180;
        agent
    }

    fn make_parent(school_id: u8, household_id: u16) -> Agent {
        Agent::new_adult(
            AgentType::Parent,
            household_id, school_id, 0, 35,
            0.0, 0.0,
            true, true,
        )
    }

    // --- MDA tests ---

    #[test]
    fn test_mda_treats_children() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_infected_child(1, 1));
        agents.insert(make_infected_child(1, 2));

        let mut rng = SmallRng::seed_from_u64(42);
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, None, 1.0, 1.0, &mut rng,
        );

        // With 100% coverage and 100% supply, all 3 children should be treated
        assert_eq!(treated, 3);

        // All treated children should have days_since_last_deworming reset
        for (_key, agent) in agents.iter() {
            assert_eq!(agent.days_since_last_deworming, 0);
        }
    }

    #[test]
    fn test_mda_filters_by_school() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_infected_child(2, 1));
        agents.insert(make_infected_child(1, 2));

        let mut rng = SmallRng::seed_from_u64(42);
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, Some(1), 1.0, 1.0, &mut rng,
        );

        // Only school_id=1 children (2 of 3) should be treated
        assert_eq!(treated, 2);
    }

    #[test]
    fn test_mda_skips_non_children() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_parent(1, 1));

        let mut rng = SmallRng::seed_from_u64(42);
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, None, 1.0, 1.0, &mut rng,
        );

        assert_eq!(treated, 1);
    }

    #[test]
    fn test_mda_zero_coverage_treats_none() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_infected_child(1, 1));

        let mut rng = SmallRng::seed_from_u64(42);
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, None, 0.0, 1.0, &mut rng,
        );

        assert_eq!(treated, 0);
    }

    #[test]
    fn test_mda_zero_supply_treats_none() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_infected_child(1, 1));

        let mut rng = SmallRng::seed_from_u64(42);
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, None, 1.0, 0.0, &mut rng,
        );

        assert_eq!(treated, 0);
    }

    #[test]
    fn test_mda_clears_infection_based_on_efficacy() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let key = agents.insert(make_infected_child(1, 0));

        // Run MDA many times with same seed to verify statistical behavior
        // Albendazole: Ascaris 95% cleared, Trichuris 30% cleared, Hookworm 72% cleared
        let mut ascaris_cleared = 0;
        let mut trichuris_cleared = 0;
        let mut hookworm_cleared = 0;
        let trials = 1000;

        for seed in 0..trials {
            // Reset the agent's infection
            let agent = agents.get_mut(key).unwrap();
            agent.ascaris = InfectionState {
                epg: 5000.0, worm_burden: 10.0, days_infected: 30,
            };
            agent.trichuris = InfectionState {
                epg: 2000.0, worm_burden: 5.0, days_infected: 20,
            };
            agent.hookworm = InfectionState {
                epg: 1000.0, worm_burden: 3.0, days_infected: 15,
            };

            let mut rng = SmallRng::seed_from_u64(seed);
            apply_mda(&mut agents, Drug::Albendazole, None, 1.0, 1.0, &mut rng);

            let agent = agents.get(key).unwrap();
            if !agent.ascaris.is_infected() { ascaris_cleared += 1; }
            if !agent.trichuris.is_infected() { trichuris_cleared += 1; }
            if !agent.hookworm.is_infected() { hookworm_cleared += 1; }
        }

        // Check within reasonable tolerance (10% margin for statistical tests)
        let ascaris_rate = ascaris_cleared as f32 / trials as f32;
        let trichuris_rate = trichuris_cleared as f32 / trials as f32;
        let hookworm_rate = hookworm_cleared as f32 / trials as f32;

        assert!(
            (ascaris_rate - 0.95).abs() < 0.10,
            "Ascaris cure rate {ascaris_rate} should be ~0.95"
        );
        assert!(
            (trichuris_rate - 0.30).abs() < 0.10,
            "Trichuris cure rate {trichuris_rate} should be ~0.30"
        );
        assert!(
            (hookworm_rate - 0.72).abs() < 0.10,
            "Hookworm cure rate {hookworm_rate} should be ~0.72"
        );
    }

    #[test]
    fn test_mda_clamps_coverage_above_one() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));

        let mut rng = SmallRng::seed_from_u64(42);
        // Should clamp to 1.0, not panic
        let treated = apply_mda(
            &mut agents, Drug::Albendazole, None, 1.5, 1.0, &mut rng,
        );
        assert_eq!(treated, 1);
    }

    // --- Education tests ---

    #[test]
    fn test_education_cartoon_boosts_knowledge() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let key = agents.insert(make_infected_child(1, 0));
        let initial_knowledge = agents.get(key).unwrap().knowledge;

        let mut rng = SmallRng::seed_from_u64(42);
        let affected = apply_education(
            &mut agents, EducationMethod::Cartoon, None, &mut rng,
        );

        assert_eq!(affected, 1);
        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge > initial_knowledge);
        // Boost should be between 0.05 and 0.15
        let boost = agent.knowledge - initial_knowledge;
        assert!(boost >= 0.05 && boost <= 0.15, "Cartoon knowledge boost {boost} out of range");
    }

    #[test]
    fn test_education_boardgame_boosts_knowledge_and_practice() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let key = agents.insert(make_infected_child(1, 0));
        let initial_knowledge = agents.get(key).unwrap().knowledge;
        let initial_practice = agents.get(key).unwrap().practice;

        let mut rng = SmallRng::seed_from_u64(42);
        apply_education(&mut agents, EducationMethod::BoardGame, None, &mut rng);

        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge > initial_knowledge);
        assert!(agent.practice > initial_practice);
    }

    #[test]
    fn test_education_teacherled_boosts_knowledge_and_attitude() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let key = agents.insert(make_infected_child(1, 0));
        let initial_knowledge = agents.get(key).unwrap().knowledge;
        let initial_attitude = agents.get(key).unwrap().attitude;

        let mut rng = SmallRng::seed_from_u64(42);
        apply_education(&mut agents, EducationMethod::TeacherLed, None, &mut rng);

        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge > initial_knowledge);
        assert!(agent.attitude > initial_attitude);
    }

    #[test]
    fn test_education_parent_meeting_affects_parents() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let child_key = agents.insert(make_infected_child(1, 0));
        let parent_key = agents.insert(make_parent(1, 1));

        let initial_parent_knowledge = agents.get(parent_key).unwrap().knowledge;
        let initial_child_knowledge = agents.get(child_key).unwrap().knowledge;

        let mut rng = SmallRng::seed_from_u64(42);
        let affected = apply_education(
            &mut agents, EducationMethod::ParentMeeting, None, &mut rng,
        );

        // Only the parent should be affected
        assert_eq!(affected, 1);
        assert!(agents.get(parent_key).unwrap().knowledge > initial_parent_knowledge);
        assert_eq!(agents.get(child_key).unwrap().knowledge, initial_child_knowledge);
    }

    #[test]
    fn test_education_filters_by_school() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_infected_child(2, 1));
        agents.insert(make_infected_child(1, 2));

        let mut rng = SmallRng::seed_from_u64(42);
        let affected = apply_education(
            &mut agents, EducationMethod::Cartoon, Some(1), &mut rng,
        );

        assert_eq!(affected, 2);
    }

    #[test]
    fn test_education_kap_capped_at_one() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let mut child = make_infected_child(1, 0);
        child.knowledge = 0.98;
        child.attitude = 0.99;
        child.practice = 0.97;
        let key = agents.insert(child);

        let mut rng = SmallRng::seed_from_u64(42);
        apply_education(&mut agents, EducationMethod::BoardGame, None, &mut rng);

        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge <= 1.0);
        assert!(agent.practice <= 1.0);
    }

    // --- BHW visits tests ---

    #[test]
    fn test_bhw_visits_boosts_kap() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let key = agents.insert(make_infected_child(1, 0));
        let initial_knowledge = agents.get(key).unwrap().knowledge;
        let initial_attitude = agents.get(key).unwrap().attitude;

        let mut rng = SmallRng::seed_from_u64(42);
        let visited = apply_bhw_visits(&mut agents, 1.0, &mut rng);

        assert_eq!(visited, 1);
        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge > initial_knowledge);
        assert!(agent.attitude > initial_attitude);
    }

    #[test]
    fn test_bhw_visits_zero_coverage() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_parent(1, 1));

        let mut rng = SmallRng::seed_from_u64(42);
        let visited = apply_bhw_visits(&mut agents, 0.0, &mut rng);

        assert_eq!(visited, 0);
    }

    #[test]
    fn test_bhw_visits_all_agent_types() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));
        agents.insert(make_parent(1, 1));
        agents.insert(Agent::new_adult(
            AgentType::Teacher, 2, 1, 0, 40, 0.0, 0.0, true, true,
        ));

        let mut rng = SmallRng::seed_from_u64(42);
        let visited = apply_bhw_visits(&mut agents, 1.0, &mut rng);

        // All 3 agents should be visited (house-to-house visits all agent types)
        assert_eq!(visited, 3);
    }

    #[test]
    fn test_bhw_visits_clamps_coverage() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(make_infected_child(1, 0));

        let mut rng = SmallRng::seed_from_u64(42);
        // Should clamp to 1.0, not exceed
        let visited = apply_bhw_visits(&mut agents, 1.5, &mut rng);
        assert_eq!(visited, 1);
    }

    #[test]
    fn test_bhw_kap_capped_at_one() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let mut child = make_infected_child(1, 0);
        child.knowledge = 0.99;
        child.attitude = 0.99;
        let key = agents.insert(child);

        let mut rng = SmallRng::seed_from_u64(42);
        apply_bhw_visits(&mut agents, 1.0, &mut rng);

        let agent = agents.get(key).unwrap();
        assert!(agent.knowledge <= 1.0);
        assert!(agent.attitude <= 1.0);
    }

    // --- Cost tests ---

    #[test]
    fn test_mda_cost() {
        let intervention = InterventionType::MDA {
            target_school: None,
            drug: Drug::Albendazole,
            coverage_pct: 0.8,
        };
        let cost = intervention_cost(&intervention, 100, 50);
        assert!((cost - 500.0).abs() < f32::EPSILON); // 5.0 * 100
    }

    #[test]
    fn test_wash_latrine_cost() {
        let intervention = InterventionType::WashImprovement {
            improvement: WashType::Latrine,
            x: 0.0, y: 0.0,
        };
        assert!((intervention_cost(&intervention, 100, 50) - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_wash_water_pump_cost() {
        let intervention = InterventionType::WashImprovement {
            improvement: WashType::WaterPump,
            x: 0.0, y: 0.0,
        };
        assert!((intervention_cost(&intervention, 100, 50) - 800.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_wash_handwash_station_cost() {
        let intervention = InterventionType::WashImprovement {
            improvement: WashType::HandwashStation,
            x: 0.0, y: 0.0,
        };
        assert!((intervention_cost(&intervention, 100, 50) - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_education_cartoon_cost() {
        let intervention = InterventionType::Education {
            method: EducationMethod::Cartoon,
            target_school: None,
        };
        assert!((intervention_cost(&intervention, 100, 50) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_education_teacherled_cost() {
        let intervention = InterventionType::Education {
            method: EducationMethod::TeacherLed,
            target_school: None,
        };
        assert!((intervention_cost(&intervention, 100, 50) - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bhw_visits_cost() {
        let intervention = InterventionType::BhwVisits {
            coverage_pct: 0.5,
            duration_days: 7,
        };
        let cost = intervention_cost(&intervention, 100, 50);
        // 20.0 * 50 * 0.5 = 500.0
        assert!((cost - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_policy_budget_increase_cost() {
        let intervention = InterventionType::PolicyBudgetIncrease { multiplier: 1.5 };
        assert!((intervention_cost(&intervention, 100, 50) - 0.0).abs() < f32::EPSILON);
    }

    // --- Duration tests ---

    #[test]
    fn test_mda_single_school_duration() {
        let intervention = InterventionType::MDA {
            target_school: Some(1),
            drug: Drug::Albendazole,
            coverage_pct: 0.8,
        };
        assert_eq!(intervention_duration_ticks(&intervention), 24);
    }

    #[test]
    fn test_mda_all_schools_duration() {
        let intervention = InterventionType::MDA {
            target_school: None,
            drug: Drug::Albendazole,
            coverage_pct: 0.8,
        };
        assert_eq!(intervention_duration_ticks(&intervention), 72);
    }

    #[test]
    fn test_education_duration() {
        let intervention = InterventionType::Education {
            method: EducationMethod::Cartoon,
            target_school: None,
        };
        assert_eq!(intervention_duration_ticks(&intervention), 24);
    }

    #[test]
    fn test_bhw_visits_duration() {
        let intervention = InterventionType::BhwVisits {
            coverage_pct: 0.8,
            duration_days: 7,
        };
        assert_eq!(intervention_duration_ticks(&intervention), 168); // 7 * 24
    }

    #[test]
    fn test_wash_improvement_duration() {
        let intervention = InterventionType::WashImprovement {
            improvement: WashType::Latrine,
            x: 0.0, y: 0.0,
        };
        assert_eq!(intervention_duration_ticks(&intervention), 0);
    }

    #[test]
    fn test_policy_budget_increase_duration() {
        let intervention = InterventionType::PolicyBudgetIncrease { multiplier: 2.0 };
        assert_eq!(intervention_duration_ticks(&intervention), 0);
    }
}
