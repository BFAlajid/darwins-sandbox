use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

use crate::infection::InfectionState;

new_key_type! {
    pub struct AgentKey;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// 6-12 years, primary simulation subject
    Child,
    /// Influences child KAP, household WASH
    Parent,
    /// School-based education delivery
    Teacher,
    /// BHW: house-to-house visits, MDA delivery
    HealthWorker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    /// Household identified by household_id
    Home(u16),
    /// School identified by school_id
    School(u8),
    /// Public area (market, playground, creek)
    Community,
    /// Health center
    HealthCenter,
    /// Water source identified by water_source_id
    WaterSource(u8),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    // --- Identity ---
    pub agent_type: AgentType,
    /// Age in years (6-12 for children)
    pub age_years: u8,
    pub household_id: u16,
    pub school_id: u8,
    pub barangay_id: u8,

    // --- Position (for rendering and spatial queries) ---
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub current_location: Location,

    // --- Infection state (per STH species) ---
    pub ascaris: InfectionState,
    pub trichuris: InfectionState,
    pub hookworm: InfectionState,

    // --- KAP scores (0.0-1.0) ---
    /// Knowledge about STH prevention
    pub knowledge: f32,
    /// Attitude toward preventive practices (0.0 = negative, 1.0 = positive)
    pub attitude: f32,
    /// Practice adherence (0.0 = high-risk, 1.0 = low-risk)
    pub practice: f32,

    // --- Behavioral state ---
    pub wears_shoes: bool,
    pub washes_hands_before_eating: bool,
    pub uses_latrine: bool,
    pub days_since_last_deworming: u16,

    // --- Structural access (SEM layer) ---
    pub household_has_latrine: bool,
    pub household_has_water: bool,
    pub school_has_wash_facility: bool,
}

impl Agent {
    /// Create a new child agent with the given parameters.
    pub fn new_child(
        household_id: u16,
        school_id: u8,
        barangay_id: u8,
        age_years: u8,
        x: f32,
        y: f32,
        knowledge: f32,
        attitude: f32,
        practice: f32,
        household_has_latrine: bool,
        household_has_water: bool,
        school_has_wash_facility: bool,
    ) -> Self {
        Self {
            agent_type: AgentType::Child,
            age_years: age_years.clamp(6, 12),
            household_id,
            school_id,
            barangay_id,
            x,
            y,
            target_x: x,
            target_y: y,
            current_location: Location::Home(household_id),
            ascaris: InfectionState::new(),
            trichuris: InfectionState::new(),
            hookworm: InfectionState::new(),
            knowledge: knowledge.clamp(0.0, 1.0),
            attitude: attitude.clamp(0.0, 1.0),
            practice: practice.clamp(0.0, 1.0),
            wears_shoes: false,
            washes_hands_before_eating: false,
            uses_latrine: false,
            days_since_last_deworming: 0,
            household_has_latrine,
            household_has_water,
            school_has_wash_facility,
        }
    }

    /// Create a new adult agent (parent, teacher, or health worker).
    pub fn new_adult(
        agent_type: AgentType,
        household_id: u16,
        school_id: u8,
        barangay_id: u8,
        age_years: u8,
        x: f32,
        y: f32,
        household_has_latrine: bool,
        household_has_water: bool,
    ) -> Self {
        debug_assert!(
            agent_type != AgentType::Child,
            "Use new_child() for child agents"
        );

        Self {
            agent_type,
            age_years,
            household_id,
            school_id,
            barangay_id,
            x,
            y,
            target_x: x,
            target_y: y,
            current_location: Location::Home(household_id),
            ascaris: InfectionState::new(),
            trichuris: InfectionState::new(),
            hookworm: InfectionState::new(),
            // Adults start with moderate KAP
            knowledge: 0.5,
            attitude: 0.5,
            practice: 0.5,
            wears_shoes: true,
            washes_hands_before_eating: false,
            uses_latrine: household_has_latrine,
            days_since_last_deworming: 0,
            household_has_latrine,
            household_has_water,
            school_has_wash_facility: false,
        }
    }

    /// Whether the agent is currently infected with any STH species.
    #[inline]
    pub fn is_infected(&self) -> bool {
        self.ascaris.is_infected()
            || self.trichuris.is_infected()
            || self.hookworm.is_infected()
    }

    /// Total EPG across all species.
    #[inline]
    pub fn total_epg(&self) -> f32 {
        self.ascaris.epg + self.trichuris.epg + self.hookworm.epg
    }

    /// Returns true if any float field is NaN (safety guard).
    #[inline]
    pub fn has_nan(&self) -> bool {
        self.x.is_nan()
            || self.y.is_nan()
            || self.target_x.is_nan()
            || self.target_y.is_nan()
            || self.knowledge.is_nan()
            || self.attitude.is_nan()
            || self.practice.is_nan()
            || self.ascaris.epg.is_nan()
            || self.trichuris.epg.is_nan()
            || self.hookworm.epg.is_nan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_child_clamps_age() {
        let agent = Agent::new_child(0, 0, 0, 4, 0.0, 0.0, 0.5, 0.5, 0.5, true, true, true);
        assert_eq!(agent.age_years, 6);

        let agent = Agent::new_child(0, 0, 0, 15, 0.0, 0.0, 0.5, 0.5, 0.5, true, true, true);
        assert_eq!(agent.age_years, 12);
    }

    #[test]
    fn test_new_child_clamps_kap() {
        let agent = Agent::new_child(0, 0, 0, 8, 0.0, 0.0, -0.5, 1.5, 2.0, true, true, true);
        assert_eq!(agent.knowledge, 0.0);
        assert_eq!(agent.attitude, 1.0);
        assert_eq!(agent.practice, 1.0);
    }

    #[test]
    fn test_new_child_starts_uninfected() {
        let agent = Agent::new_child(1, 2, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true);
        assert!(!agent.is_infected());
        assert_eq!(agent.total_epg(), 0.0);
    }

    #[test]
    fn test_agent_type_equality() {
        assert_eq!(AgentType::Child, AgentType::Child);
        assert_ne!(AgentType::Child, AgentType::Parent);
    }

    #[test]
    fn test_location_equality() {
        assert_eq!(Location::Home(5), Location::Home(5));
        assert_ne!(Location::Home(5), Location::Home(6));
        assert_ne!(Location::Home(5), Location::Community);
        assert_eq!(Location::School(1), Location::School(1));
    }

    #[test]
    fn test_new_adult() {
        let agent = Agent::new_adult(
            AgentType::Teacher,
            10, 1, 0, 35,
            100.0, 200.0,
            true, true,
        );
        assert_eq!(agent.agent_type, AgentType::Teacher);
        assert_eq!(agent.household_id, 10);
        assert_eq!(agent.school_id, 1);
        assert!(agent.uses_latrine);
    }

    #[test]
    fn test_has_nan_detects_nan_position() {
        let mut agent = Agent::new_child(0, 0, 0, 8, 0.0, 0.0, 0.5, 0.5, 0.5, true, true, true);
        assert!(!agent.has_nan());
        agent.x = f32::NAN;
        assert!(agent.has_nan());
    }

    #[test]
    fn test_is_infected_with_one_species() {
        let mut agent = Agent::new_child(0, 0, 0, 8, 0.0, 0.0, 0.5, 0.5, 0.5, true, true, true);
        assert!(!agent.is_infected());

        agent.ascaris.epg = 100.0;
        agent.ascaris.worm_burden = 1.0;
        assert!(agent.is_infected());
        assert!(agent.total_epg() > 0.0);
    }
}
