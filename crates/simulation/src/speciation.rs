use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::brain::{IH_WEIGHTS, HO_WEIGHTS};
use crate::config::SimConfig;
use crate::creature::Creature;

/// Colorblind-safe palette (Wong/IBM style, 8 high-contrast colors)
const SPECIES_PALETTE: [(f32, f32, f32); 8] = [
    (0.90, 0.62, 0.00), // orange
    (0.34, 0.71, 0.91), // sky blue
    (0.00, 0.62, 0.45), // bluish green
    (0.94, 0.89, 0.26), // yellow
    (0.00, 0.45, 0.70), // blue
    (0.84, 0.37, 0.00), // vermilion
    (0.80, 0.47, 0.65), // reddish purple
    (0.60, 0.60, 0.60), // gray (fallback)
];

/// Latin-ish phoneme tables for name generation
const ONSETS: &[&str] = &[
    "b", "c", "d", "f", "g", "l", "m", "n", "p", "r", "s", "t", "v", "z",
    "br", "cr", "dr", "fl", "gr", "pl", "pr", "sc", "sp", "st", "tr",
];
const VOWELS: &[&str] = &["a", "e", "i", "o", "u", "ae", "ia", "io", "us"];
const CODAS: &[&str] = &["", "", "", "n", "s", "x", "r", "l", "m"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Species {
    pub id: u32,
    pub name: String,
    pub color: (f32, f32, f32),
    pub member_count: u32,
    /// Representative brain weights for distance comparison
    rep_brain_ih: Vec<f32>,
    rep_brain_ho: Vec<f32>,
    /// Representative trait values
    rep_speed: f32,
    rep_size: f32,
    rep_vision: f32,
}

pub struct SpeciesTracker {
    pub species: Vec<Species>,
    pub threshold: f32,
    next_id: u32,
    ticks_since_adjust: u32,
}

impl SpeciesTracker {
    pub fn new(config: &SimConfig) -> Self {
        // Create the initial species (id=0)
        let initial = Species {
            id: 0,
            name: "Primordius vitas".to_string(),
            color: SPECIES_PALETTE[0],
            member_count: config.initial_population,
            rep_brain_ih: vec![0.0; IH_WEIGHTS],
            rep_brain_ho: vec![0.0; HO_WEIGHTS],
            rep_speed: (config.min_speed + config.max_speed) * 0.5,
            rep_size: (config.min_size + config.max_size) * 0.5,
            rep_vision: (config.min_vision_range + config.max_vision_range) * 0.5,
        };

        Self {
            species: vec![initial],
            threshold: config.compatibility_threshold,
            next_id: 1,
            ticks_since_adjust: 0,
        }
    }

    /// Assign species to an offspring. Returns the species_id.
    /// If close to parent's species, joins it. Otherwise, forks a new species.
    pub fn assign_species(
        &mut self,
        offspring: &Creature,
        parent_species_id: u32,
        rng: &mut impl Rng,
    ) -> u32 {
        // First check parent's species
        if let Some(species) = self.species.iter().find(|s| s.id == parent_species_id) {
            let dist = genome_distance(offspring, species);
            if dist < self.threshold {
                return parent_species_id;
            }
        }

        // Check all other species
        let mut best_id = None;
        let mut best_dist = f32::MAX;
        for species in &self.species {
            let dist = genome_distance(offspring, species);
            if dist < self.threshold && dist < best_dist {
                best_dist = dist;
                best_id = Some(species.id);
            }
        }

        if let Some(id) = best_id {
            return id;
        }

        // Fork new species
        let id = self.next_id;
        self.next_id += 1;
        let color_idx = (id as usize) % SPECIES_PALETTE.len();
        let name = generate_species_name(id, rng);

        self.species.push(Species {
            id,
            name,
            color: SPECIES_PALETTE[color_idx],
            member_count: 0,
            rep_brain_ih: offspring.brain.ih_weights.clone(),
            rep_brain_ho: offspring.brain.ho_weights.clone(),
            rep_speed: offspring.speed_trait,
            rep_size: offspring.size_trait,
            rep_vision: offspring.vision_range,
        });

        id
    }

    /// Recount members and update representatives. Call after step.
    pub fn update_counts(&mut self, creatures: impl Iterator<Item = (u32, f32, f32, f32, Vec<f32>, Vec<f32>)>) {
        // Reset counts
        for s in &mut self.species {
            s.member_count = 0;
        }

        // Count and pick a representative (first creature per species)
        for (species_id, speed, size, vision, ih, ho) in creatures {
            if let Some(s) = self.species.iter_mut().find(|s| s.id == species_id) {
                if s.member_count == 0 {
                    // Use first member as representative
                    s.rep_speed = speed;
                    s.rep_size = size;
                    s.rep_vision = vision;
                    s.rep_brain_ih = ih;
                    s.rep_brain_ho = ho;
                }
                s.member_count += 1;
            }
        }

        // Remove extinct species
        self.species.retain(|s| s.member_count > 0);
    }

    /// Adjust compatibility threshold to maintain target species count.
    /// Call every 100 ticks.
    pub fn adjust_threshold(&mut self, config: &SimConfig) {
        self.ticks_since_adjust += 1;
        if self.ticks_since_adjust < 100 {
            return;
        }
        self.ticks_since_adjust = 0;

        let num_species = self.species.len() as u32;
        if num_species > config.target_species_max {
            self.threshold *= 1.05; // relax — merge similar species
        } else if num_species < config.target_species_min {
            self.threshold *= 0.95; // tighten — encourage splitting
        }

        // Keep threshold in reasonable bounds
        self.threshold = self.threshold.clamp(0.1, 50.0);
    }

    /// Get color for a species_id
    pub fn get_color(&self, species_id: u32) -> (f32, f32, f32) {
        self.species.iter()
            .find(|s| s.id == species_id)
            .map(|s| s.color)
            .unwrap_or(SPECIES_PALETTE[SPECIES_PALETTE.len() - 1])
    }

    /// Get species count
    pub fn species_count(&self) -> usize {
        self.species.len()
    }
}

/// Genome distance: 0.3 * trait_distance + 0.7 * brain_distance
/// Both components are normalized L1 distances.
fn genome_distance(creature: &Creature, species: &Species) -> f32 {
    // Trait distance (normalized by typical ranges)
    let d_speed = (creature.speed_trait - species.rep_speed).abs() / 7.0; // max_speed - min_speed ~ 7
    let d_size = (creature.size_trait - species.rep_size).abs() / 8.0;    // max_size - min_size ~ 8
    let d_vision = (creature.vision_range - species.rep_vision).abs() / 90.0; // max - min ~ 90
    let d_traits = (d_speed + d_size + d_vision) / 3.0;

    // Brain distance (normalized L1 over weights)
    let brain = &creature.brain;
    let ih_dist: f32 = brain.ih_weights.iter()
        .zip(species.rep_brain_ih.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / IH_WEIGHTS as f32;

    let ho_dist: f32 = brain.ho_weights.iter()
        .zip(species.rep_brain_ho.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / HO_WEIGHTS as f32;

    let d_brain = (ih_dist + ho_dist) / 2.0;

    0.3 * d_traits + 0.7 * d_brain
}

/// Generate a Latin-ish species name from id + rng
fn generate_species_name(_id: u32, rng: &mut impl Rng) -> String {
    let genus = generate_word(rng, 2, 3);
    let species = generate_word(rng, 2, 2);

    // Capitalize genus
    let mut genus_chars = genus.chars();
    let capitalized: String = match genus_chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + genus_chars.as_str(),
    };

    format!("{} {}", capitalized, species)
}

fn generate_word(rng: &mut impl Rng, min_syllables: usize, max_syllables: usize) -> String {
    let syllable_count = rng.gen_range(min_syllables..=max_syllables);
    let mut word = String::with_capacity(syllable_count * 4);

    for _ in 0..syllable_count {
        let onset = ONSETS[rng.gen_range(0..ONSETS.len())];
        let vowel = VOWELS[rng.gen_range(0..VOWELS.len())];
        let coda = CODAS[rng.gen_range(0..CODAS.len())];
        word.push_str(onset);
        word.push_str(vowel);
        word.push_str(coda);
    }

    word
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_name_generation() {
        let mut rng = SmallRng::seed_from_u64(42);
        for i in 0..10 {
            let name = generate_species_name(i, &mut rng);
            assert!(name.contains(' '), "Name should have genus + species: {}", name);
            assert!(name.len() > 4, "Name too short: {}", name);
            // First char should be uppercase
            assert!(name.chars().next().unwrap().is_uppercase());
        }
    }

    #[test]
    fn test_species_palette_valid() {
        for (r, g, b) in &SPECIES_PALETTE {
            assert!(*r >= 0.0 && *r <= 1.0);
            assert!(*g >= 0.0 && *g <= 1.0);
            assert!(*b >= 0.0 && *b <= 1.0);
        }
    }

    #[test]
    fn test_threshold_adjustment() {
        let config = SimConfig::default();
        let mut tracker = SpeciesTracker::new(&config);
        let initial_threshold = tracker.threshold;

        // Simulate 100 ticks with too few species
        tracker.ticks_since_adjust = 99;
        tracker.adjust_threshold(&config);
        // Should have tightened (threshold decreased)
        assert!(tracker.threshold < initial_threshold);
    }
}
