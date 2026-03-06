use crate::creature::{Creature, CreatureKey};
use crate::speciation::SpeciesTracker;
use slotmap::DenseSlotMap;

/// 12 floats per creature: [x, y, rotation, size, energy_norm, r, g, b, vx, vy, age_norm, species_id]
pub const FLOATS_PER_CREATURE: usize = 12;

pub struct RenderBuffer {
    data: Vec<f32>,
    creature_count: usize,
}

impl RenderBuffer {
    pub fn new(max_creatures: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_creatures * FLOATS_PER_CREATURE),
            creature_count: 0,
        }
    }

    /// Pack all creature data into the flat buffer, sorted by species_id for
    /// pre-grouped path batching on the JS side.
    pub fn pack(
        &mut self,
        creatures: &DenseSlotMap<CreatureKey, Creature>,
        species_tracker: &SpeciesTracker,
        max_energy: f32,
        max_lifespan: u32,
    ) {
        self.creature_count = creatures.len();
        let needed = self.creature_count * FLOATS_PER_CREATURE;
        self.data.clear();
        if self.data.capacity() < needed {
            self.data.reserve(needed - self.data.capacity());
        }

        // Collect and sort by species_id for pre-grouped rendering
        let mut sorted: Vec<&Creature> = creatures.values().collect();
        sorted.sort_unstable_by_key(|c| c.species_id);

        for creature in sorted {
            let (r, g, b) = species_tracker.get_color(creature.species_id);
            let energy_norm = (creature.energy / max_energy).clamp(0.0, 1.0);
            let age_norm =
                (creature.age as f32 / max_lifespan as f32).clamp(0.0, 1.0);

            self.data.push(creature.x);
            self.data.push(creature.y);
            self.data.push(creature.rotation);
            self.data.push(creature.size_trait);
            self.data.push(energy_norm);
            self.data.push(r);
            self.data.push(g);
            self.data.push(b);
            self.data.push(creature.vx);
            self.data.push(creature.vy);
            self.data.push(age_norm);
            self.data.push(creature.species_id as f32);
        }
    }

    pub fn as_ptr(&self) -> *const f32 {
        self.data.as_ptr()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn creature_count(&self) -> usize {
        self.creature_count
    }
}
