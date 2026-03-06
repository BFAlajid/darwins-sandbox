use crate::creature::{Creature, CreatureKey};
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
            // Default white color for M1 (species colors come in M4)
            let (r, g, b) = species_color_placeholder(creature.species_id);
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

/// Simple deterministic color from species_id (placeholder until proper speciation).
/// Uses golden angle hue rotation for distinct colors.
fn species_color_placeholder(species_id: u32) -> (f32, f32, f32) {
    let hue = (species_id as f32 * 137.508) % 360.0;
    let s = 0.7_f32;
    let l = 0.6_f32;
    hsl_to_rgb_normalized(hue, s, l)
}

fn hsl_to_rgb_normalized(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h2 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    (r1 + m, g1 + m, b1 + m)
}
