/// Daily cycle and agent movement scheduling for the STH simulation.
///
/// 1 tick = 1 hour of simulated time. 24 ticks = 1 day.
/// Agents follow location schedules based on their type, time of day,
/// and whether it is a school day.

use crate::agent::{Agent, AgentType, Location};

// ---------- Time of Day ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeOfDay {
    EarlyMorning, // 5-6: wake, hygiene
    Morning,      // 7-11: school / work
    Lunch,        // 12: eating (risk: unwashed food)
    Afternoon,    // 13-16: school or community play
    Evening,      // 17-19: home, eating
    Night,        // 20-4: sleep (no movement)
}

impl TimeOfDay {
    /// Convert a tick number to the corresponding time of day.
    pub fn from_tick(tick: u64) -> Self {
        let hour = (tick % 24) as u8;
        match hour {
            5..=6 => TimeOfDay::EarlyMorning,
            7..=11 => TimeOfDay::Morning,
            12 => TimeOfDay::Lunch,
            13..=16 => TimeOfDay::Afternoon,
            17..=19 => TimeOfDay::Evening,
            _ => TimeOfDay::Night, // 20-23 and 0-4
        }
    }

    /// Day number within the simulation (0-indexed).
    pub fn day_of_simulation(tick: u64) -> u32 {
        (tick / 24) as u32
    }

    /// Month number within the simulation (0-indexed, 30-day months).
    pub fn month_of_simulation(tick: u64) -> u32 {
        Self::day_of_simulation(tick) / 30
    }

    /// Whether the current tick falls on a school day (Mon-Fri).
    /// Day 0 is Monday.
    pub fn is_school_day(tick: u64) -> bool {
        let day_of_week = (tick / 24) % 7;
        day_of_week < 5
    }
}

// ---------- Location Routing ----------

/// Determine where an agent should be at the current time.
///
/// - **Children**: home at night/early-morning/evening; school on school days;
///   community on weekends.
/// - **Parents**: home at night/early-morning/evening; community (market, work)
///   during the day.
/// - **Teachers**: school on school days during work hours; home otherwise.
/// - **HealthWorkers**: health center during work hours; home otherwise.
pub fn target_location(agent: &Agent, time: TimeOfDay, is_school_day: bool) -> Location {
    match agent.agent_type {
        AgentType::Child => match time {
            TimeOfDay::Night | TimeOfDay::EarlyMorning | TimeOfDay::Evening => {
                Location::Home(agent.household_id)
            }
            TimeOfDay::Morning | TimeOfDay::Lunch | TimeOfDay::Afternoon => {
                if is_school_day {
                    Location::School(agent.school_id)
                } else {
                    Location::Community
                }
            }
        },
        AgentType::Parent => match time {
            TimeOfDay::Night | TimeOfDay::EarlyMorning | TimeOfDay::Evening => {
                Location::Home(agent.household_id)
            }
            _ => Location::Community,
        },
        AgentType::Teacher => match time {
            TimeOfDay::Morning | TimeOfDay::Lunch | TimeOfDay::Afternoon => {
                if is_school_day {
                    Location::School(agent.school_id)
                } else {
                    Location::Home(agent.household_id)
                }
            }
            _ => Location::Home(agent.household_id),
        },
        AgentType::HealthWorker => match time {
            TimeOfDay::Morning | TimeOfDay::Afternoon => Location::HealthCenter,
            _ => Location::Home(agent.household_id),
        },
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, AgentType, Location};

    /// Helper to create a minimal agent for testing.
    fn make_agent(agent_type: AgentType, household_id: u16, school_id: u8) -> Agent {
        if agent_type == AgentType::Child {
            Agent::new_child(
                household_id,
                school_id,
                0, // barangay_id
                8, // age_years
                0.0,
                0.0,
                0.5,
                0.5,
                0.5,
                true,
                true,
                true,
            )
        } else {
            Agent::new_adult(
                agent_type,
                household_id,
                school_id,
                0, // barangay_id
                30,
                0.0,
                0.0,
                true,
                true,
            )
        }
    }

    // ---- TimeOfDay::from_tick ----

    #[test]
    fn time_of_day_night() {
        // Hours 0-4 and 20-23
        for h in [0u64, 1, 2, 3, 4, 20, 21, 22, 23] {
            assert_eq!(TimeOfDay::from_tick(h), TimeOfDay::Night, "hour {h}");
        }
    }

    #[test]
    fn time_of_day_early_morning() {
        assert_eq!(TimeOfDay::from_tick(5), TimeOfDay::EarlyMorning);
        assert_eq!(TimeOfDay::from_tick(6), TimeOfDay::EarlyMorning);
    }

    #[test]
    fn time_of_day_morning() {
        for h in 7..=11 {
            assert_eq!(TimeOfDay::from_tick(h), TimeOfDay::Morning, "hour {h}");
        }
    }

    #[test]
    fn time_of_day_lunch() {
        assert_eq!(TimeOfDay::from_tick(12), TimeOfDay::Lunch);
    }

    #[test]
    fn time_of_day_afternoon() {
        for h in 13..=16 {
            assert_eq!(TimeOfDay::from_tick(h), TimeOfDay::Afternoon, "hour {h}");
        }
    }

    #[test]
    fn time_of_day_evening() {
        for h in 17..=19 {
            assert_eq!(TimeOfDay::from_tick(h), TimeOfDay::Evening, "hour {h}");
        }
    }

    #[test]
    fn time_of_day_wraps_across_days() {
        // tick 24 = hour 0 of day 1 = Night
        assert_eq!(TimeOfDay::from_tick(24), TimeOfDay::Night);
        // tick 31 = hour 7 of day 1 = Morning
        assert_eq!(TimeOfDay::from_tick(31), TimeOfDay::Morning);
    }

    // ---- day/month/school helpers ----

    #[test]
    fn day_of_simulation() {
        assert_eq!(TimeOfDay::day_of_simulation(0), 0);
        assert_eq!(TimeOfDay::day_of_simulation(23), 0);
        assert_eq!(TimeOfDay::day_of_simulation(24), 1);
        assert_eq!(TimeOfDay::day_of_simulation(47), 1);
        assert_eq!(TimeOfDay::day_of_simulation(48), 2);
    }

    #[test]
    fn month_of_simulation() {
        assert_eq!(TimeOfDay::month_of_simulation(0), 0);
        // Day 30 = tick 720
        assert_eq!(TimeOfDay::month_of_simulation(720), 1);
    }

    #[test]
    fn school_day_pattern() {
        // Day 0 (Mon) through Day 4 (Fri) are school days
        for day in 0..5 {
            let tick = day * 24 + 8; // 8am
            assert!(
                TimeOfDay::is_school_day(tick),
                "day {day} should be a school day"
            );
        }
        // Day 5 (Sat) and Day 6 (Sun) are not
        for day in 5..7 {
            let tick = day * 24 + 8;
            assert!(
                !TimeOfDay::is_school_day(tick),
                "day {day} should be a weekend"
            );
        }
        // Day 7 (Mon again) is a school day
        assert!(TimeOfDay::is_school_day(7 * 24 + 8));
    }

    // ---- target_location ----

    #[test]
    fn child_at_school_on_school_day() {
        let child = make_agent(AgentType::Child, 1, 2);
        let loc = target_location(&child, TimeOfDay::Morning, true);
        assert_eq!(loc, Location::School(2));
    }

    #[test]
    fn child_in_community_on_weekend() {
        let child = make_agent(AgentType::Child, 1, 2);
        let loc = target_location(&child, TimeOfDay::Morning, false);
        assert_eq!(loc, Location::Community);
    }

    #[test]
    fn child_at_home_at_night() {
        let child = make_agent(AgentType::Child, 1, 2);
        let loc = target_location(&child, TimeOfDay::Night, true);
        assert_eq!(loc, Location::Home(1));
    }

    #[test]
    fn parent_at_community_during_day() {
        let parent = make_agent(AgentType::Parent, 5, 0);
        let loc = target_location(&parent, TimeOfDay::Morning, true);
        assert_eq!(loc, Location::Community);
    }

    #[test]
    fn parent_at_home_at_evening() {
        let parent = make_agent(AgentType::Parent, 5, 0);
        let loc = target_location(&parent, TimeOfDay::Evening, true);
        assert_eq!(loc, Location::Home(5));
    }

    #[test]
    fn teacher_at_school_on_school_day() {
        let teacher = make_agent(AgentType::Teacher, 3, 1);
        let loc = target_location(&teacher, TimeOfDay::Morning, true);
        assert_eq!(loc, Location::School(1));
    }

    #[test]
    fn teacher_at_home_on_weekend() {
        let teacher = make_agent(AgentType::Teacher, 3, 1);
        let loc = target_location(&teacher, TimeOfDay::Morning, false);
        assert_eq!(loc, Location::Home(3));
    }

    #[test]
    fn health_worker_at_center_during_work() {
        let hw = make_agent(AgentType::HealthWorker, 7, 0);
        let loc = target_location(&hw, TimeOfDay::Morning, true);
        assert_eq!(loc, Location::HealthCenter);
    }

    #[test]
    fn health_worker_at_home_at_night() {
        let hw = make_agent(AgentType::HealthWorker, 7, 0);
        let loc = target_location(&hw, TimeOfDay::Night, true);
        assert_eq!(loc, Location::Home(7));
    }
}
