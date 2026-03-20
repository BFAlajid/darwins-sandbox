//! Render buffer for WASM-to-JS data transfer.
//!
//! Packs agent, environment, and facility data into flat f32 arrays
//! for efficient transfer to the JavaScript rendering layer.

use slotmap::DenseSlotMap;

use crate::agent::{Agent, AgentKey, AgentType};
use crate::environment::EnvironmentGrid;
use crate::infection::SthSpecies;
use crate::infection::Intensity;

/// Number of f32 values per agent in the render buffer.
///
/// Layout: [x, y, target_x, target_y, agent_type, infection_status,
///          epg_norm, knowledge, attitude, practice, household_id,
///          school_id, r, g, b, barangay_id]
pub const FLOATS_PER_AGENT: usize = 16;

/// Number of f32 values per facility: [x, y, type, 1.0]
pub const FLOATS_PER_FACILITY: usize = 4;

// ---------------------------------------------------------------------------
// Render buffer
// ---------------------------------------------------------------------------

pub struct SthRenderBuffer {
    agent_data: Vec<f32>,
    env_data: Vec<f32>,
    facility_data: Vec<f32>,
}

impl SthRenderBuffer {
    pub fn new() -> Self {
        Self {
            agent_data: Vec::new(),
            env_data: Vec::new(),
            facility_data: Vec::new(),
        }
    }

    /// Pack all render data from the current simulation state.
    pub fn pack(
        &mut self,
        agents: &DenseSlotMap<AgentKey, Agent>,
        environment: &EnvironmentGrid,
    ) {
        // --- Agent data ---
        self.agent_data.clear();
        let needed = agents.len() * FLOATS_PER_AGENT;
        if self.agent_data.capacity() < needed {
            self.agent_data.reserve(needed - self.agent_data.capacity());
        }

        for agent in agents.values() {
            self.agent_data.push(agent.x);
            self.agent_data.push(agent.y);
            self.agent_data.push(agent.target_x);
            self.agent_data.push(agent.target_y);
            self.agent_data.push(agent.agent_type as u8 as f32);

            // Infection status: worst across all species
            // 0=negative, 1=light, 2=moderate, 3=heavy
            let worst = [
                agent.ascaris.intensity(SthSpecies::Ascaris),
                agent.trichuris.intensity(SthSpecies::Trichuris),
                agent.hookworm.intensity(SthSpecies::Hookworm),
            ]
            .iter()
            .map(|i| match i {
                Intensity::Negative => 0u8,
                Intensity::Light => 1,
                Intensity::Moderate => 2,
                Intensity::Heavy => 3,
            })
            .max()
            .unwrap_or(0);
            self.agent_data.push(worst as f32);

            // Normalized EPG (log scale): log10(epg) / 5.0, clamped to [0, 1]
            let total_epg = agent.ascaris.epg + agent.trichuris.epg + agent.hookworm.epg;
            let epg_norm = if total_epg > 0.0 {
                (total_epg.max(1.0).log10() / 5.0).clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.agent_data.push(epg_norm);

            self.agent_data.push(agent.knowledge);
            self.agent_data.push(agent.attitude);
            self.agent_data.push(agent.practice);
            self.agent_data.push(agent.household_id as f32);
            self.agent_data.push(agent.school_id as f32);

            // Color derived from agent type, tinted by infection for children
            let (r, g, b) = agent_color(agent);
            self.agent_data.push(r);
            self.agent_data.push(g);
            self.agent_data.push(b);

            self.agent_data.push(agent.barangay_id as f32);
        }

        // --- Environment contamination grid ---
        self.env_data = environment.contamination_grid_flat();

        // --- Facility positions ---
        self.facility_data = environment.facility_positions_flat();
    }

    /// Get agent render data as a slice.
    pub fn agent_data(&self) -> &[f32] {
        &self.agent_data
    }

    /// Get environment contamination data as a slice.
    pub fn env_data(&self) -> &[f32] {
        &self.env_data
    }

    /// Get facility position data as a slice.
    pub fn facility_data(&self) -> &[f32] {
        &self.facility_data
    }

    /// Number of agents in the buffer.
    pub fn agent_count(&self) -> usize {
        self.agent_data.len() / FLOATS_PER_AGENT
    }
}

impl Default for SthRenderBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Agent color computation
// ---------------------------------------------------------------------------

/// Compute RGB color for an agent based on type and infection status.
///
/// - Children: blue base (0.3, 0.7, 0.9), tinted red by infection severity
/// - Parents: green (0.5, 0.8, 0.5)
/// - Teachers: gold (0.9, 0.7, 0.2)
/// - Health workers: red (0.9, 0.3, 0.3)
fn agent_color(agent: &Agent) -> (f32, f32, f32) {
    let base = match agent.agent_type {
        AgentType::Child => (0.3, 0.7, 0.9),
        AgentType::Parent => (0.5, 0.8, 0.5),
        AgentType::Teacher => (0.9, 0.7, 0.2),
        AgentType::HealthWorker => (0.9, 0.3, 0.3),
    };

    // Tint children toward red based on infection severity
    if agent.agent_type == AgentType::Child {
        let infection_max = agent
            .ascaris
            .epg
            .max(agent.trichuris.epg)
            .max(agent.hookworm.epg);
        if infection_max > 0.0 {
            let severity = (infection_max.max(1.0).log10() / 5.0).clamp(0.0, 1.0);
            (
                base.0 + (0.9 - base.0) * severity,
                base.1 * (1.0 - severity * 0.6),
                base.2 * (1.0 - severity * 0.6),
            )
        } else {
            base
        }
    } else {
        base
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::environment::EnvironmentGrid;
    use crate::infection::InfectionState;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = SthRenderBuffer::new();
        assert_eq!(buf.agent_count(), 0);
        assert!(buf.agent_data().is_empty());
        assert!(buf.env_data().is_empty());
        assert!(buf.facility_data().is_empty());
    }

    #[test]
    fn test_pack_single_agent() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        ));

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();
        buf.pack(&agents, &env);

        assert_eq!(buf.agent_count(), 1);
        assert_eq!(buf.agent_data().len(), FLOATS_PER_AGENT);

        // Check x, y
        assert!((buf.agent_data()[0] - 10.0).abs() < 1e-6);
        assert!((buf.agent_data()[1] - 20.0).abs() < 1e-6);

        // Check agent_type == Child (0)
        assert!((buf.agent_data()[4] - 0.0).abs() < 1e-6);

        // Check infection_status == 0 (uninfected)
        assert!((buf.agent_data()[5] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_pack_multiple_agents() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        ));
        agents.insert(Agent::new_adult(
            AgentType::Parent, 0, 0, 0, 35, 30.0, 40.0, true, true,
        ));
        agents.insert(Agent::new_adult(
            AgentType::Teacher, 1, 1, 0, 40, 50.0, 60.0, true, true,
        ));

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();
        buf.pack(&agents, &env);

        assert_eq!(buf.agent_count(), 3);
        assert_eq!(buf.agent_data().len(), 3 * FLOATS_PER_AGENT);
    }

    #[test]
    fn test_pack_infected_child_color_tinted() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        let mut child = Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        );
        // Heavy ascaris infection
        child.ascaris = InfectionState {
            epg: 50000.0,
            worm_burden: 50.0,
            days_infected: 60,
        };
        agents.insert(child);

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();
        buf.pack(&agents, &env);

        // R should be tinted toward 0.9 (more red)
        let r = buf.agent_data()[12];
        assert!(r > 0.3, "Infected child should have higher red: {r}");

        // Infection status should be Heavy (3)
        assert!((buf.agent_data()[5] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_env_data_matches_grid() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        ));

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();
        buf.pack(&agents, &env);

        // 100/10 * 100/10 = 100 cells
        assert_eq!(buf.env_data().len(), 100);
    }

    #[test]
    fn test_epg_norm_zero_for_uninfected() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        ));

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();
        buf.pack(&agents, &env);

        // EPG norm at index 6 should be 0.0
        assert!((buf.agent_data()[6] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_pack_clears_previous_data() {
        let mut agents: DenseSlotMap<AgentKey, Agent> = DenseSlotMap::with_key();
        agents.insert(Agent::new_child(
            0, 0, 0, 8, 10.0, 20.0, 0.5, 0.5, 0.5, true, true, true,
        ));

        let env = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let mut buf = SthRenderBuffer::new();

        // Pack once with 1 agent
        buf.pack(&agents, &env);
        assert_eq!(buf.agent_count(), 1);

        // Add another agent and re-pack
        agents.insert(Agent::new_child(
            1, 0, 0, 9, 30.0, 40.0, 0.5, 0.5, 0.5, true, true, true,
        ));
        buf.pack(&agents, &env);
        assert_eq!(buf.agent_count(), 2);

        // Remove all and re-pack
        let keys: Vec<AgentKey> = agents.keys().collect();
        for key in keys {
            agents.remove(key);
        }
        buf.pack(&agents, &env);
        assert_eq!(buf.agent_count(), 0);
    }
}
