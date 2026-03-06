use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::config::SimConfig;

pub const NUM_INPUTS: usize = 12;
pub const NUM_HIDDEN: usize = 8;
pub const NUM_OUTPUTS: usize = 3;

// Weight counts
pub const IH_WEIGHTS: usize = NUM_INPUTS * NUM_HIDDEN; // 96
pub const HO_WEIGHTS: usize = NUM_HIDDEN * NUM_OUTPUTS; // 24
pub const TOTAL_PARAMS: usize = IH_WEIGHTS + NUM_HIDDEN + HO_WEIGHTS + NUM_OUTPUTS; // 131

/// Polynomial tanh approximation: x*(27+x²)/(27+9x²)
/// Avoids transcendental functions for cross-browser determinism.
#[inline(always)]
fn poly_tanh(x: f32) -> f32 {
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// Feedforward neural network with fixed topology: 12→8→3
/// Uses Vec<f32> for serde compatibility (arrays >32 not supported).
/// Forward pass still uses const loop bounds for compiler optimization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brain {
    pub ih_weights: Vec<f32>,  // length IH_WEIGHTS (96)
    pub h_biases: Vec<f32>,    // length NUM_HIDDEN (8)
    pub ho_weights: Vec<f32>,  // length HO_WEIGHTS (24)
    pub o_biases: Vec<f32>,    // length NUM_OUTPUTS (3)
}

/// Output of forward pass
pub struct BrainOutput {
    pub turn: f32,   // [-1, 1] → scaled to [-max_turn_rate, max_turn_rate]
    pub thrust: f32,  // [0, 1]
    pub reproduce: f32, // > 0.5 = wants to reproduce
}

impl Brain {
    /// Random initialization with small weights
    pub fn new_random(rng: &mut impl Rng, config: &SimConfig) -> Self {
        let clamp = config.weight_clamp;
        let init_range = 1.0 / (NUM_INPUTS as f32).sqrt();

        let ih_weights: Vec<f32> = (0..IH_WEIGHTS)
            .map(|_| rng.gen_range(-init_range..init_range).clamp(-clamp, clamp))
            .collect();

        let h_biases: Vec<f32> = (0..NUM_HIDDEN)
            .map(|_| rng.gen_range(-0.1..0.1))
            .collect();

        let init_range_ho = 1.0 / (NUM_HIDDEN as f32).sqrt();
        let ho_weights: Vec<f32> = (0..HO_WEIGHTS)
            .map(|_| rng.gen_range(-init_range_ho..init_range_ho).clamp(-clamp, clamp))
            .collect();

        let mut o_biases: Vec<f32> = vec![0.0; NUM_OUTPUTS];
        // Slight negative bias on reproduce output to prevent over-reproduction at start
        o_biases[2] = -0.5;

        Self {
            ih_weights,
            h_biases,
            ho_weights,
            o_biases,
        }
    }

    /// Create offspring brain with mutation
    pub fn mutate(&self, rng: &mut impl Rng, config: &SimConfig) -> Self {
        let rate = config.base_mutation_rate;
        let strength = config.base_mutation_strength;
        let clamp = config.weight_clamp;
        let cauchy_prob = config.cauchy_probability;

        let mut child = self.clone();

        // Mutate all weight arrays
        mutate_weights(&mut child.ih_weights, rng, rate, strength, clamp, cauchy_prob);
        mutate_weights(&mut child.h_biases, rng, rate, strength, clamp, cauchy_prob);
        mutate_weights(&mut child.ho_weights, rng, rate, strength, clamp, cauchy_prob);
        mutate_weights(&mut child.o_biases, rng, rate, strength, clamp, cauchy_prob);

        child
    }

    /// Forward pass: inputs → hidden (poly_tanh) → outputs (poly_tanh)
    #[inline]
    pub fn forward(&self, inputs: &[f32; NUM_INPUTS]) -> BrainOutput {
        // Hidden layer
        let mut hidden = [0.0f32; NUM_HIDDEN];
        for h in 0..NUM_HIDDEN {
            let mut sum = self.h_biases[h];
            let base = h * NUM_INPUTS;
            for i in 0..NUM_INPUTS {
                sum += inputs[i] * self.ih_weights[base + i];
            }
            hidden[h] = poly_tanh(sum);
        }

        // Output layer
        let mut outputs = [0.0f32; NUM_OUTPUTS];
        for o in 0..NUM_OUTPUTS {
            let mut sum = self.o_biases[o];
            let base = o * NUM_HIDDEN;
            for h in 0..NUM_HIDDEN {
                sum += hidden[h] * self.ho_weights[base + h];
            }
            outputs[o] = poly_tanh(sum);
        }

        BrainOutput {
            turn: outputs[0],                        // [-1, 1]
            thrust: (outputs[1] + 1.0) * 0.5,        // map [-1,1] → [0,1]
            reproduce: (outputs[2] + 1.0) * 0.5,     // map [-1,1] → [0,1]
        }
    }

    /// Check for NaN in any weight
    pub fn has_nan(&self) -> bool {
        self.ih_weights.iter().any(|w| w.is_nan())
            || self.h_biases.iter().any(|b| b.is_nan())
            || self.ho_weights.iter().any(|w| w.is_nan())
            || self.o_biases.iter().any(|b| b.is_nan())
    }
}

/// Mutate a weight array in-place
fn mutate_weights(
    weights: &mut [f32],
    rng: &mut impl Rng,
    rate: f32,
    strength: f32,
    clamp: f32,
    cauchy_prob: f32,
) {
    for w in weights.iter_mut() {
        if rng.gen::<f32>() < rate {
            let perturbation = if rng.gen::<f32>() < cauchy_prob {
                // Cauchy mutation for occasional large jumps
                cauchy_sample(rng) * strength * 3.0
            } else {
                // Gaussian-like mutation (using uniform approximation)
                rng.gen_range(-1.0..1.0) * strength
            };
            *w = (*w + perturbation).clamp(-clamp, clamp);
        }
    }
}

/// Simple Cauchy sample via inverse CDF: tan(π * (u - 0.5))
fn cauchy_sample(rng: &mut impl Rng) -> f32 {
    let u: f32 = rng.gen_range(0.01..0.99);
    (std::f32::consts::PI * (u - 0.5)).tan()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_poly_tanh_bounds() {
        // poly_tanh should be bounded roughly in [-1, 1]
        for x in [-10.0, -5.0, -1.0, 0.0, 1.0, 5.0, 10.0] {
            let y = poly_tanh(x);
            assert!(y >= -1.5 && y <= 1.5, "poly_tanh({}) = {} out of bounds", x, y);
        }
        assert!((poly_tanh(0.0)).abs() < 1e-6);
    }

    #[test]
    fn test_forward_deterministic() {
        let config = SimConfig::default();
        let mut rng = SmallRng::seed_from_u64(42);
        let brain = Brain::new_random(&mut rng, &config);

        let inputs = [0.5f32; NUM_INPUTS];
        let out1 = brain.forward(&inputs);
        let out2 = brain.forward(&inputs);

        assert_eq!(out1.turn, out2.turn);
        assert_eq!(out1.thrust, out2.thrust);
        assert_eq!(out1.reproduce, out2.reproduce);
    }

    #[test]
    fn test_mutation_preserves_clamp() {
        let config = SimConfig::default();
        let mut rng = SmallRng::seed_from_u64(99);
        let brain = Brain::new_random(&mut rng, &config);

        let child = brain.mutate(&mut rng, &config);
        let clamp = config.weight_clamp;

        for w in child.ih_weights.iter().chain(child.ho_weights.iter())
            .chain(child.h_biases.iter()).chain(child.o_biases.iter()) {
            assert!(*w >= -clamp && *w <= clamp, "weight {} out of clamp range", w);
        }
    }

    #[test]
    fn test_output_ranges() {
        let config = SimConfig::default();
        let mut rng = SmallRng::seed_from_u64(123);
        let brain = Brain::new_random(&mut rng, &config);

        // Test with various inputs
        for seed in 0..100 {
            let mut inputs = [0.0f32; NUM_INPUTS];
            let mut r = SmallRng::seed_from_u64(seed);
            for inp in inputs.iter_mut() {
                *inp = r.gen_range(-1.0..1.0);
            }
            let out = brain.forward(&inputs);
            assert!(out.turn >= -1.0 && out.turn <= 1.0);
            assert!(out.thrust >= 0.0 && out.thrust <= 1.0);
            assert!(out.reproduce >= 0.0 && out.reproduce <= 1.0);
        }
    }
}
