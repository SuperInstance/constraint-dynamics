//! Constraint energy landscape: gradient descent and simulated annealing.

use crate::constraint::{ConstraintSystem, NumericValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type State = HashMap<String, i64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyLandscape {
    pub states: Vec<State>,
    pub energy: Vec<f64>,
    pub transitions: Vec<(usize, usize, f64)>,
}

impl EnergyLandscape {
    pub fn new() -> Self { Self { states: Vec::new(), energy: Vec::new(), transitions: Vec::new() } }

    pub fn add_state(&mut self, state: State, energy: f64) -> usize {
        let idx = self.states.len();
        self.states.push(state);
        self.energy.push(energy);
        idx
    }

    pub fn add_transition(&mut self, from: usize, to: usize, barrier: f64) {
        self.transitions.push((from, to, barrier));
    }

    pub fn global_minimum(&self) -> Option<(usize, f64)> {
        self.energy.iter().enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, &e)| (i, e))
    }

    pub fn local_minima(&self) -> Vec<usize> {
        let neighbors: HashMap<usize, Vec<usize>> = {
            let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
            for &(from, to, _) in &self.transitions {
                map.entry(from).or_default().push(to);
                map.entry(to).or_default().push(from);
            }
            map
        };

        self.states.iter().enumerate()
            .filter(|(i, _)| {
                let my_energy = self.energy[*i];
                let nbrs = neighbors.get(i).cloned().unwrap_or_default();
                nbrs.is_empty() || nbrs.iter().all(|&j| self.energy[j] >= my_energy)
            })
            .map(|(i, _)| i)
            .collect()
    }
}

impl Default for EnergyLandscape { fn default() -> Self { Self::new() } }

// ---------------------------------------------------------------------------
// Gradient Descent
// ---------------------------------------------------------------------------

pub struct GradientDescent {
    pub learning_rate: f64,
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl GradientDescent {
    pub fn new(learning_rate: f64, max_iterations: usize) -> Self {
        Self { learning_rate, max_iterations, tolerance: 1e-8 }
    }

    pub fn run<V: NumericValue>(&self, system: &mut ConstraintSystem<V>) -> (HashMap<String, i64>, f64) {
        let mut best_energy = system.total_violation();
        let mut best_assignments: HashMap<String, i64> = system.assignments.iter()
            .map(|(k, v)| (k.clone(), v.to_f64() as i64)).collect();

        for _ in 0..self.max_iterations {
            let current_energy = system.total_violation();
            if current_energy < best_energy {
                best_energy = current_energy;
                best_assignments = system.assignments.iter()
                    .map(|(k, v)| (k.clone(), v.to_f64() as i64)).collect();
            }
            if current_energy < self.tolerance { break; }

            // Simple hill-climbing: try flipping each variable to a better value
            let var_names: Vec<String> = system.assignments.keys().cloned().collect();
            for var in &var_names {
                let original = match system.assignments.get(var) {
                    Some(v) => *v,
                    None => continue,
                };

                // Try incrementing and decrementing
                for delta in [-1.0, 1.0] {
                    let mut test = system.assignments.clone();
                    test.insert(var.clone(), V::from_f64(original.to_f64() + delta));
                    let test_energy = system.constraints.iter().enumerate()
                        .map(|(i, c)| c.evaluate(&test).violation * system.weights.get(&i).copied().unwrap_or(c.weight()))
                        .sum::<f64>();

                    if test_energy < current_energy {
                        system.assignments.insert(var.clone(), V::from_f64(original.to_f64() + delta));
                        break;
                    }
                }
            }
        }

        (best_assignments, best_energy)
    }
}

// ---------------------------------------------------------------------------
// Simulated Annealing
// ---------------------------------------------------------------------------

mod rng {
    pub struct Rng { state: u64 }
    impl Rng {
        pub fn new(seed: u64) -> Self { Self { state: if seed == 0 { 1 } else { seed } } }
        pub fn next_u64(&mut self) -> u64 {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 7;
            self.state ^= self.state << 17;
            self.state
        }
        pub fn next_f64(&mut self) -> f64 {
            (self.next_u64() as f64) / (u64::MAX as f64)
        }
    }
}

pub struct SimulatedAnnealing {
    pub initial_temperature: f64,
    pub cooling_rate: f64,
    pub max_iterations: usize,
    pub min_temperature: f64,
}

impl SimulatedAnnealing {
    pub fn new(initial_temp: f64, cooling_rate: f64, max_iterations: usize) -> Self {
        Self { initial_temperature: initial_temp, cooling_rate, max_iterations, min_temperature: 1e-6 }
    }

    pub fn run<V: NumericValue>(
        &self,
        system: &mut ConstraintSystem<V>,
        domains: &HashMap<String, Vec<i64>>,
        seed: u64,
    ) -> (f64, HashMap<String, i64>) {
        let mut rng = rng::Rng::new(seed);
        let mut temperature = self.initial_temperature;

        // Initialize random
        for (var, vals) in domains {
            if !vals.is_empty() {
                let idx = (rng.next_u64() as usize) % vals.len();
                system.assign(var, V::from_f64(vals[idx] as f64));
            }
        }

        let mut current_energy = system.total_violation();
        let mut best_energy = current_energy;
        let mut best_assignments: HashMap<String, i64> = system.assignments.iter()
            .map(|(k, v)| (k.clone(), v.to_f64() as i64)).collect();

        let var_names: Vec<String> = domains.keys().cloned().collect();
        if var_names.is_empty() {
            return (best_energy, best_assignments);
        }

        for _ in 0..self.max_iterations {
            if temperature < self.min_temperature { break; }

            let vi = (rng.next_u64() as usize) % var_names.len();
            let var = &var_names[vi];
            let vals = match domains.get(var) {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };

            let new_val = vals[(rng.next_u64() as usize) % vals.len()];
            let old_val = system.assignments.get(var).copied();

            system.assign(var, V::from_f64(new_val as f64));
            let new_energy = system.total_violation();
            let delta = new_energy - current_energy;

            if delta < 0.0 || rng.next_f64() < (-delta / temperature).exp().min(1.0) {
                current_energy = new_energy;
                if current_energy < best_energy {
                    best_energy = current_energy;
                    best_assignments = system.assignments.iter()
                        .map(|(k, v)| (k.clone(), v.to_f64() as i64)).collect();
                }
            } else {
                if let Some(old) = old_val {
                    system.assign(var, old);
                }
            }

            temperature *= self.cooling_rate;
        }

        (best_energy, best_assignments)
    }
}
