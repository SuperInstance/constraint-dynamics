//! Constraint propagation engine: AC-3, forward checking, backjumping.

use crate::constraint::{Constraint, ConstraintSystem, Domain, NumericValue};
use anyhow::{Result, bail};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Arc {
    pub variable: String,
    pub constraint_idx: usize,
}

pub struct PropagationEngine;

impl PropagationEngine {
    pub fn ac3<V: NumericValue>(system: &mut ConstraintSystem<V>) -> Result<bool> {
        let mut queue: VecDeque<Arc> = VecDeque::new();

        for (ci, c) in system.constraints.iter().enumerate() {
            for v in c.variables() {
                queue.push_back(Arc { variable: v.to_string(), constraint_idx: ci });
            }
        }

        while let Some(arc) = queue.pop_front() {
            if Self::revise(system, &arc)? {
                if let Some(Domain::Discrete(vals)) = system.variables.get(&arc.variable) {
                    if vals.is_empty() {
                        return Ok(false);
                    }
                }
                for (ci, c) in system.constraints.iter().enumerate() {
                    if ci == arc.constraint_idx { continue; }
                    if c.variables().contains(&arc.variable.as_str()) {
                        for v2 in c.variables() {
                            if v2 != arc.variable {
                                queue.push_back(Arc { variable: v2.to_string(), constraint_idx: ci });
                            }
                        }
                    }
                }
            }
        }

        system.propagation_queue.clear();
        Ok(true)
    }

    fn revise<V: NumericValue>(system: &mut ConstraintSystem<V>, arc: &Arc) -> Result<bool> {
        let constraint = system.constraints.get(arc.constraint_idx).cloned();
        let Some(constraint) = constraint else { return Ok(false) };

        let domain = system.variables.get(&arc.variable).cloned();
        let Some(domain) = domain else { return Ok(false) };

        match domain {
            Domain::Discrete(values) => {
                let surviving: Vec<V> = values.iter()
                    .filter(|val| Self::has_support(system, &constraint, &arc.variable, val).unwrap_or(false))
                    .cloned()
                    .collect();

                if surviving.len() < values.len() {
                    system.variables.insert(arc.variable.clone(), Domain::Discrete(surviving));
                    return Ok(true);
                }
                Ok(false)
            }
            Domain::Continuous { .. } | Domain::Unbounded => Ok(false),
        }
    }

    fn has_support<V: NumericValue>(
        system: &ConstraintSystem<V>,
        constraint: &Constraint<V>,
        variable: &str,
        value: &V,
    ) -> Result<bool> {
        let mut test_assign = system.assignments.clone();
        test_assign.insert(variable.to_string(), *value);

        // Check if all variables in the constraint are assigned
        let vars = constraint.variables();
        let all_assigned = vars.iter().all(|v| test_assign.contains_key(*v));

        if all_assigned {
            return Ok(constraint.evaluate(&test_assign).is_satisfied());
        }

        // Try to find support from other variables' domains
        let other_vars: Vec<&str> = vars.iter().copied()
            .filter(|v| *v != variable && !test_assign.contains_key(*v))
            .collect();

        for &other_var in &other_vars {
            if let Some(Domain::Discrete(vals)) = system.variables.get(other_var) {
                for v in vals {
                    test_assign.insert(other_var.to_string(), *v);
                    let all_now = vars.iter().all(|v2| test_assign.contains_key(*v2));
                    if all_now {
                        if constraint.evaluate(&test_assign).is_satisfied() {
                            return Ok(true);
                        }
                    } else {
                        // Recurse for remaining unassigned
                        // Simplified: just check if not violated
                        if !constraint.evaluate(&test_assign).is_violated() {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }
}

pub struct ArcConsistency3;

impl ArcConsistency3 {
    pub fn run<V: NumericValue>(mut system: ConstraintSystem<V>) -> Result<ConstraintSystem<V>> {
        if PropagationEngine::ac3(&mut system)? {
            Ok(system)
        } else {
            bail!("AC-3: unsatisfiable (domain wipe-out)")
        }
    }
}

pub struct ForwardChecker;

impl ForwardChecker {
    pub fn check<V: NumericValue>(system: &mut ConstraintSystem<V>, variable: &str) -> Result<bool> {
        let assigned_val = match system.assignments.get(variable) {
            Some(v) => *v,
            None => return Ok(true),
        };

        let mut consistent = true;

        for constraint in &system.constraints {
            let vars = constraint.variables();
            if !vars.contains(&variable) { continue; }

            for other_var in vars {
                if other_var == variable || system.assignments.contains_key(other_var) { continue; }

                if let Some(Domain::Discrete(vals)) = system.variables.get(other_var).cloned() {
                    let surviving: Vec<V> = vals.iter()
                        .filter(|v| {
                            let mut test = system.assignments.clone();
                            test.insert(variable.to_string(), assigned_val);
                            test.insert(other_var.to_string(), **v);
                            !constraint.evaluate(&test).is_violated()
                        })
                        .cloned()
                        .collect();

                    if surviving.is_empty() { consistent = false; }
                    system.variables.insert(other_var.to_string(), Domain::Discrete(surviving));
                }
            }
        }

        Ok(consistent)
    }
}

#[derive(Debug, Clone, Default)]
pub struct BackjumpState {
    pub conflict_sets: HashMap<String, HashSet<String>>,
    pub assignment_order: Vec<String>,
}

impl BackjumpState {
    pub fn new() -> Self { Self::default() }

    pub fn record_conflict(&mut self, variable: impl Into<String>, culprit: impl Into<String>) {
        self.conflict_sets.entry(variable.into()).or_default().insert(culprit.into());
    }

    pub fn backjump_target(&self, variable: &str) -> Option<usize> {
        let conflict = self.conflict_sets.get(variable)?;
        self.assignment_order.iter().enumerate()
            .filter(|(_, v)| conflict.contains(*v))
            .last()
            .map(|(i, _)| i)
    }
}
