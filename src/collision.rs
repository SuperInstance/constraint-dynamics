//! Constraint collision detection and resolution.

use crate::constraint::{ConstraintSystem, NumericValue};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collision {
    pub constraints: Vec<usize>,
    pub variables: Vec<String>,
    pub severity: f64,
    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    RelaxLowest { relax_idx: usize, factor: f64 },
    RemoveNewest { remove_idx: usize },
    Emergent { description: String },
}

pub struct CollisionDetector;

impl CollisionDetector {
    pub fn detect<V: NumericValue>(system: &ConstraintSystem<V>) -> Vec<Collision> {
        let violated = system.violated_constraints();
        if violated.is_empty() { return Vec::new(); }

        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut assigned: HashSet<usize> = HashSet::new();

        for &vi in &violated {
            if assigned.contains(&vi) { continue; }
            let mut group = vec![vi];
            assigned.insert(vi);
            let vars_i: HashSet<String> = system.constraints[vi].variables()
                .into_iter().map(|s| s.to_string()).collect();

            for &vj in &violated {
                if assigned.contains(&vj) { continue; }
                let vars_j: HashSet<String> = system.constraints[vj].variables()
                    .into_iter().map(|s| s.to_string()).collect();
                if !vars_i.is_disjoint(&vars_j) {
                    group.push(vj);
                    assigned.insert(vj);
                }
            }
            groups.push(group);
        }

        groups.into_iter().map(|ci| {
            let all_vars: HashSet<String> = ci.iter()
                .flat_map(|&i| system.constraints[i].variables())
                .map(|s| s.to_string()).collect();

            let relax_idx = ci.iter()
                .min_by(|&&a, &&b| {
                    let wa = system.weights.get(&a).copied().unwrap_or(1.0);
                    let wb = system.weights.get(&b).copied().unwrap_or(1.0);
                    wa.partial_cmp(&wb).unwrap()
                })
                .copied().unwrap_or(ci[0]);

            let severity = ci.len() as f64;
            Collision {
                constraints: ci,
                variables: all_vars.into_iter().collect(),
                severity,
                resolution: ConflictResolution::RelaxLowest { relax_idx, factor: 0.5 },
            }
        }).collect()
    }

    pub fn unsatisfiable_core<V: NumericValue>(system: &ConstraintSystem<V>) -> Vec<usize> {
        let violated = system.violated_constraints();
        if violated.is_empty() { return Vec::new(); }

        let mut core = violated.clone();
        let mut i = 0;
        while i < core.len() {
            let candidate: Vec<usize> = core.iter().enumerate()
                .filter(|(j, _)| *j != i).map(|(_, &v)| v).collect();

            let still_violated: Vec<usize> = candidate.iter()
                .filter(|&&idx| system.constraints[idx].evaluate(&system.assignments).is_violated())
                .copied().collect();

            if !still_violated.is_empty() || candidate.is_empty() {
                i += 1;
            } else {
                core.remove(i);
            }
        }
        core
    }

    pub fn resolve<V: NumericValue>(system: &mut ConstraintSystem<V>, collision: &Collision) -> Result<()> {
        match &collision.resolution {
            ConflictResolution::RelaxLowest { relax_idx, factor } => {
                let current = system.weights.get(relax_idx).copied().unwrap_or(1.0);
                system.weights.insert(*relax_idx, current * factor);
                Ok(())
            }
            ConflictResolution::RemoveNewest { remove_idx } => {
                system.weights.insert(*remove_idx, 0.0);
                Ok(())
            }
            ConflictResolution::Emergent { .. } => Ok(()),
        }
    }

    pub fn is_consistent<V: NumericValue>(
        system: &ConstraintSystem<V>,
        constraint_indices: &[usize],
    ) -> bool {
        for i in 0..constraint_indices.len() {
            for j in (i+1)..constraint_indices.len() {
                let ci = &system.constraints[constraint_indices[i]];
                let cj = &system.constraints[constraint_indices[j]];
                let shared: Vec<&str> = ci.variables().into_iter()
                    .filter(|v| cj.variables().contains(v)).collect();
                if shared.is_empty() { continue; }
                if ci.evaluate(&system.assignments).is_violated()
                    && cj.evaluate(&system.assignments).is_violated()
                {
                    return false;
                }
            }
        }
        true
    }
}
