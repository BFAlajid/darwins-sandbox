//! Domain event tracking for the STH simulation.
//!
//! Captures meaningful simulation events (MDA campaigns, outbreaks,
//! facility construction, etc.) for display in the UI event log.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SthEventType {
    MdaCompleted {
        school_id: Option<u8>,
        treated: u32,
    },
    OutbreakDetected {
        prevalence: f32,
    },
    PrevalenceDropped {
        from: f32,
        to: f32,
    },
    FacilityBuilt {
        facility_type: String,
        x: f32,
        y: f32,
    },
    EducationCompleted {
        method: String,
        affected: u32,
    },
    BhwVisitsCompleted {
        visited: u32,
    },
    RainEvent,
    BudgetDepleted,
    MedicineStockout,
    CovidClosureStart,
    CovidClosureEnd,
}

// ---------------------------------------------------------------------------
// Event struct
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SthEvent {
    pub tick: u64,
    pub event_type: SthEventType,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Event tracker
// ---------------------------------------------------------------------------

pub struct SthEventTracker {
    events: Vec<SthEvent>,
}

impl SthEventTracker {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    /// Push a new event onto the tracker.
    pub fn push(&mut self, tick: u64, event_type: SthEventType, message: String) {
        self.events.push(SthEvent {
            tick,
            event_type,
            message,
        });
    }

    /// Serialize all events to JSON and clear the buffer.
    /// Used for incremental event delivery to the JS frontend.
    pub fn drain_json(&mut self) -> String {
        let json = serde_json::to_string(&self.events).unwrap_or_else(|_| "[]".to_string());
        self.events.clear();
        json
    }

    /// Serialize all events to JSON without clearing.
    pub fn get_json(&self) -> String {
        serde_json::to_string(&self.events).unwrap_or_else(|_| "[]".to_string())
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether there are no pending events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for SthEventTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_is_empty() {
        let tracker = SthEventTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_push_and_len() {
        let mut tracker = SthEventTracker::new();
        tracker.push(0, SthEventType::RainEvent, "It rained".to_string());
        assert_eq!(tracker.len(), 1);
        assert!(!tracker.is_empty());
    }

    #[test]
    fn test_drain_json_clears_events() {
        let mut tracker = SthEventTracker::new();
        tracker.push(10, SthEventType::BudgetDepleted, "Budget gone".to_string());
        tracker.push(
            20,
            SthEventType::MdaCompleted {
                school_id: Some(1),
                treated: 50,
            },
            "MDA done".to_string(),
        );

        let json = tracker.drain_json();
        assert!(!json.is_empty());
        assert!(json.contains("BudgetDepleted"));
        assert!(json.contains("MdaCompleted"));

        // After drain, tracker should be empty
        assert!(tracker.is_empty());
        assert_eq!(tracker.drain_json(), "[]");
    }

    #[test]
    fn test_get_json_preserves_events() {
        let mut tracker = SthEventTracker::new();
        tracker.push(5, SthEventType::RainEvent, "Rain".to_string());

        let json1 = tracker.get_json();
        let json2 = tracker.get_json();
        assert_eq!(json1, json2);
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = SthEvent {
            tick: 100,
            event_type: SthEventType::PrevalenceDropped {
                from: 0.3,
                to: 0.1,
            },
            message: "Prevalence dropped".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: SthEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tick, 100);
        assert_eq!(deserialized.message, "Prevalence dropped");
    }

    #[test]
    fn test_facility_built_event() {
        let mut tracker = SthEventTracker::new();
        tracker.push(
            50,
            SthEventType::FacilityBuilt {
                facility_type: "Latrine".to_string(),
                x: 100.0,
                y: 200.0,
            },
            "Latrine built at (100, 200)".to_string(),
        );
        let json = tracker.get_json();
        assert!(json.contains("Latrine"));
        assert!(json.contains("100"));
    }
}
