//! Symmetry detection and exploitation.

use crate::constraint::{Constraint, ConstraintSystem, NumericValue};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymmetryGroup {
    pub variables: Vec<String>,
    pub orbits: Vec<Vec<String>>,
    pub reduction_factor: usize,
}

impl SymmetryGroup {
    fn factorial(n: usize) -> usize {
        (1..=n).try_fold(1usize, |acc, i| acc.checked_mul(i)).unwrap_or(usize::MAX)
    }

    pub fn compute_reduction_factor(&mut self) {
        self.reduction_factor = Self::factorial(self.variables.len()).max(1);
    }
}

pub struct SymmetryDetector;

impl SymmetryDetector {
    pub fn detect<V>(system: &ConstraintSystem<V>) -> Vec<SymmetryGroup> {
        let all_vars: Vec<String> = system.variables.keys().cloned().collect();
        let mut groups: Vec<SymmetryGroup> = Vec::new();
        let mut assigned: HashSet<String> = HashSet::new();

        for i in 0..all_vars.len() {
            if assigned.contains(&all_vars[i]) { continue; }
            let mut group_vars = vec![all_vars[i].clone()];
            assigned.insert(all_vars[i].clone());

            for j in (i+1)..all_vars.len() {
                if assigned.contains(&all_vars[j]) { continue; }
                if Self::are_symmetric(system, &all_vars[i], &all_vars[j]) {
                    group_vars.push(all_vars[j].clone());
                    assigned.insert(all_vars[j].clone());
                }
            }

            if group_vars.len() > 1 {
                let mut group = SymmetryGroup {
                    variables: group_vars.clone(),
                    orbits: vec![group_vars],
                    reduction_factor: 1,
                };
                group.compute_reduction_factor();
                groups.push(group);
            }
        }
        groups
    }

    fn are_symmetric<V>(system: &ConstraintSystem<V>, a: &str, b: &str) -> bool {
        let role_a = Self::variable_role(system, a);
        let role_b = Self::variable_role(system, b);
        if role_a != role_b { return false; }

        // For structural symmetry, we need that for each constraint containing a,
        // there is a structurally identical constraint containing b (with a↔b swap).
        // Simplified check: count constraints for each variable
        let count_a = system.constraints.iter().filter(|c| c.variables().contains(&a)).count();
        let count_b = system.constraints.iter().filter(|c| c.variables().contains(&b)).count();
        if count_a != count_b { return false; }

        true
    }

    fn variable_role<V>(system: &ConstraintSystem<V>, var: &str) -> Vec<String> {
        let mut role = Vec::new();
        for c in &system.constraints {
            if c.variables().contains(&var) {
                role.push(Self::constraint_role(c, var));
            }
        }
        role.sort();
        role
    }

    fn constraint_role<V>(c: &Constraint<V>, var: &str) -> String {
        match c {
            Constraint::Equality { a, b, .. } => {
                if a.as_str() == var { "eq_l".into() }
                else if b.as_str() == var { "eq_r".into() }
                else { "none".into() }
            }
            Constraint::Inequality { a, b } => {
                if a.as_str() == var || b.as_str() == var { "ineq".into() } else { "none".into() }
            }
            Constraint::Resource { usage_vars, .. } => {
                if usage_vars.iter().any(|v| v.as_str() == var) { "res".into() } else { "none".into() }
            }
            Constraint::Temporal { before, after, .. } => {
                if before.as_str() == var { "t_before".into() }
                else if after.as_str() == var { "t_after".into() }
                else { "none".into() }
            }
            Constraint::Dependency { requires, enables } => {
                if requires.as_str() == var { "dep_req".into() }
                else if enables.as_str() == var { "dep_en".into() }
                else { "none".into() }
            }
            Constraint::MutualExclusion { variables } => {
                if variables.iter().any(|v| v.as_str() == var) { "mutex".into() } else { "none".into() }
            }
        }
    }

    pub fn total_reduction(groups: &[SymmetryGroup]) -> usize {
        groups.iter().map(|g| g.reduction_factor).product::<usize>().max(1)
    }

    pub fn break_symmetries<V: NumericValue>(
        system: &mut ConstraintSystem<V>,
        groups: &[SymmetryGroup],
    ) {
        for group in groups {
            if group.variables.len() < 2 { continue; }
            for i in 0..group.variables.len() - 1 {
                system.add_constraint(Constraint::equality(&group.variables[i], &group.variables[i + 1]));
            }
        }
    }
}
