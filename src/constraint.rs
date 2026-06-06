//! Core constraint types and the constraint system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Trait for values that can be used numerically in constraint evaluation.
pub trait NumericValue: Clone + PartialEq + fmt::Debug + Send + Sync + 'static + Copy {
    fn to_f64(self) -> f64;
    fn from_f64(v: f64) -> Self;
}

impl NumericValue for i64 {
    fn to_f64(self) -> f64 { self as f64 }
    fn from_f64(v: f64) -> Self { v.round() as i64 }
}

impl NumericValue for f64 {
    fn to_f64(self) -> f64 { self }
    fn from_f64(v: f64) -> Self { v }
}

impl NumericValue for i32 {
    fn to_f64(self) -> f64 { self as f64 }
    fn from_f64(v: f64) -> Self { v.round() as i32 }
}

// ---------------------------------------------------------------------------
// Domain
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Domain<V> {
    Discrete(Vec<V>),
    Continuous { lo: f64, hi: f64 },
    Unbounded,
}

impl<V: Clone> Domain<V> {
    pub fn size(&self) -> Option<usize> {
        match self {
            Domain::Discrete(v) => Some(v.len()),
            Domain::Continuous { .. } | Domain::Unbounded => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Domain::Discrete(v) => v.is_empty(),
            _ => false,
        }
    }

    pub fn restrict_to(&mut self, allowed: &[V])
    where V: PartialEq {
        if let Domain::Discrete(vs) = self {
            vs.retain(|v| allowed.contains(v));
        }
    }

    pub fn remove(&mut self, value: &V)
    where V: PartialEq {
        if let Domain::Discrete(vs) = self {
            vs.retain(|v| v != value);
        }
    }
}

// ---------------------------------------------------------------------------
// Satisfaction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SatisfactionLevel {
    Satisfied,
    Partial(f64),
    Violated,
}

impl SatisfactionLevel {
    pub fn score(&self) -> f64 {
        match self {
            SatisfactionLevel::Satisfied => 1.0,
            SatisfactionLevel::Partial(v) => v.clamp(0.0, 1.0),
            SatisfactionLevel::Violated => 0.0,
        }
    }

    pub fn is_satisfied(&self) -> bool { matches!(self, SatisfactionLevel::Satisfied) }
    pub fn is_violated(&self) -> bool { matches!(self, SatisfactionLevel::Violated) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Satisfaction {
    pub level: SatisfactionLevel,
    pub violation: f64,
    pub reason: String,
}

impl Satisfaction {
    pub fn satisfied() -> Self {
        Satisfaction { level: SatisfactionLevel::Satisfied, violation: 0.0, reason: "satisfied".into() }
    }

    pub fn violated(reason: impl Into<String>) -> Self {
        Satisfaction { level: SatisfactionLevel::Violated, violation: 1.0, reason: reason.into() }
    }

    pub fn partial(score: f64, violation: f64, reason: impl Into<String>) -> Self {
        Satisfaction { level: SatisfactionLevel::Partial(score), violation, reason: reason.into() }
    }

    pub fn is_violated(&self) -> bool { self.level.is_violated() }
    pub fn is_satisfied(&self) -> bool { self.level.is_satisfied() }
}

// ---------------------------------------------------------------------------
// Constraint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint<V> {
    Equality { a: String, b: String, #[serde(skip)] phantom: std::marker::PhantomData<V> },
    Inequality { a: String, b: String },
    Resource { name: String, total: f64, usage_vars: Vec<String> },
    Temporal { before: String, after: String, gap: Duration },
    Dependency { requires: String, enables: String },
    MutualExclusion { variables: Vec<String> },
}

impl<V> Constraint<V> {
    pub fn equality(a: impl Into<String>, b: impl Into<String>) -> Self {
        Constraint::Equality { a: a.into(), b: b.into(), phantom: std::marker::PhantomData }
    }
    pub fn inequality(a: impl Into<String>, b: impl Into<String>) -> Self {
        Constraint::Inequality { a: a.into(), b: b.into() }
    }
    pub fn resource(name: impl Into<String>, total: f64, usage_vars: Vec<String>) -> Self {
        Constraint::Resource { name: name.into(), total, usage_vars }
    }
    pub fn temporal(before: impl Into<String>, after: impl Into<String>, gap: Duration) -> Self {
        Constraint::Temporal { before: before.into(), after: after.into(), gap }
    }
    pub fn dependency(requires: impl Into<String>, enables: impl Into<String>) -> Self {
        Constraint::Dependency { requires: requires.into(), enables: enables.into() }
    }
    pub fn mutual_exclusion(variables: Vec<String>) -> Self {
        Constraint::MutualExclusion { variables }
    }

    pub fn variables(&self) -> Vec<&str> {
        match self {
            Constraint::Equality { a, b, .. } => vec![a.as_str(), b.as_str()],
            Constraint::Inequality { a, b } => vec![a.as_str(), b.as_str()],
            Constraint::Resource { usage_vars, .. } => usage_vars.iter().map(|s| s.as_str()).collect(),
            Constraint::Temporal { before, after, .. } => vec![before.as_str(), after.as_str()],
            Constraint::Dependency { requires, enables } => vec![requires.as_str(), enables.as_str()],
            Constraint::MutualExclusion { variables } => variables.iter().map(|s| s.as_str()).collect(),
        }
    }

    pub fn evaluate(&self, assignments: &HashMap<String, V>) -> Satisfaction
    where V: NumericValue {
        match self {
            Constraint::Equality { a, b, .. } => {
                let va = assignments.get(a.as_str());
                let vb = assignments.get(b.as_str());
                match (va, vb) {
                    (Some(va), Some(vb)) if va == vb => Satisfaction::satisfied(),
                    (Some(_), Some(_)) => Satisfaction::violated(format!("{a} ≠ {b}")),
                    _ => Satisfaction::partial(0.5, 0.5, "unassigned"),
                }
            }
            Constraint::Inequality { a, b } => {
                let va = assignments.get(a.as_str());
                let vb = assignments.get(b.as_str());
                match (va, vb) {
                    (Some(va), Some(vb)) if va != vb => Satisfaction::satisfied(),
                    (Some(_), Some(_)) => Satisfaction::violated(format!("{a} = {b}")),
                    _ => Satisfaction::partial(0.5, 0.5, "unassigned"),
                }
            }
            Constraint::Resource { total, usage_vars, .. } => {
                let assigned: Vec<&V> = usage_vars.iter().filter_map(|v| assignments.get(v.as_str())).collect();
                if assigned.is_empty() {
                    return Satisfaction::partial(0.5, 0.0, "no usage assigned");
                }
                let usage: f64 = assigned.iter().map(|v| (*v).to_f64()).sum();
                if usage <= *total {
                    Satisfaction::satisfied()
                } else {
                    Satisfaction::violated(format!("resource over by {:.2}", usage - total))
                }
            }
            Constraint::Temporal { before, after, gap, .. } => {
                match (assignments.get(before.as_str()), assignments.get(after.as_str())) {
                    (Some(vb), Some(va)) => {
                        let tb = vb.to_f64();
                        let ta = va.to_f64();
                        if ta >= tb + gap.as_secs_f64() {
                            Satisfaction::satisfied()
                        } else {
                            Satisfaction::violated("temporal gap deficit")
                        }
                    }
                    _ => Satisfaction::partial(0.5, 0.5, "unassigned"),
                }
            }
            Constraint::Dependency { requires, enables } => {
                match (assignments.get(requires.as_str()), assignments.get(enables.as_str())) {
                    (Some(_), Some(_)) => Satisfaction::satisfied(),
                    (None, Some(_)) => Satisfaction::violated(format!("{enables} without {requires}")),
                    _ => Satisfaction::partial(0.5, 0.0, "unassigned"),
                }
            }
            Constraint::MutualExclusion { variables } => {
                let values: Vec<(&str, &V)> = variables
                    .iter().filter_map(|v| assignments.get(v.as_str()).map(|val| (v.as_str(), val))).collect();
                for i in 0..values.len() {
                    for j in (i+1)..values.len() {
                        if values[i].1 == values[j].1 {
                            return Satisfaction::violated("mutual exclusion violated");
                        }
                    }
                }
                Satisfaction::satisfied()
            }
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            Constraint::Resource { .. } => 1.5,
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// ConstraintSystem
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSystem<V> {
    pub constraints: Vec<Constraint<V>>,
    pub variables: HashMap<String, Domain<V>>,
    pub assignments: HashMap<String, V>,
    pub weights: HashMap<usize, f64>,
    pub propagation_queue: Vec<usize>,
}

impl<V: NumericValue> ConstraintSystem<V> {
    pub fn new() -> Self {
        ConstraintSystem {
            constraints: Vec::new(),
            variables: HashMap::new(),
            assignments: HashMap::new(),
            weights: HashMap::new(),
            propagation_queue: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, name: impl Into<String>, domain: Domain<V>) {
        self.variables.insert(name.into(), domain);
    }

    pub fn add_constraint(&mut self, c: Constraint<V>) -> usize {
        let idx = self.constraints.len();
        self.propagation_queue.push(idx);
        self.constraints.push(c);
        idx
    }

    pub fn assign(&mut self, var: impl Into<String>, value: V) {
        let name = var.into();
        self.assignments.insert(name.clone(), value);
        for (i, c) in self.constraints.iter().enumerate() {
            if c.variables().contains(&name.as_str()) && !self.propagation_queue.contains(&i) {
                self.propagation_queue.push(i);
            }
        }
    }

    pub fn total_violation(&self) -> f64 {
        self.constraints.iter().enumerate()
            .map(|(i, c)| c.evaluate(&self.assignments).violation * self.weights.get(&i).copied().unwrap_or(c.weight()))
            .sum()
    }

    pub fn overall_satisfaction(&self) -> f64 {
        if self.constraints.is_empty() { return 1.0; }
        let total: f64 = self.constraints.iter().enumerate()
            .map(|(i, c)| c.evaluate(&self.assignments).level.score() * self.weights.get(&i).copied().unwrap_or(c.weight()))
            .sum();
        let w_sum: f64 = self.constraints.iter().enumerate()
            .map(|(i, c)| self.weights.get(&i).copied().unwrap_or(c.weight()))
            .sum();
        total / w_sum
    }

    pub fn violated_constraints(&self) -> Vec<usize> {
        self.constraints.iter().enumerate()
            .filter(|(_, c)| c.evaluate(&self.assignments).is_violated())
            .map(|(i, _)| i)
            .collect()
    }

    pub fn variable_count(&self) -> usize { self.variables.len() }
    pub fn constraint_count(&self) -> usize { self.constraints.len() }
}

impl<V: NumericValue> Default for ConstraintSystem<V> {
    fn default() -> Self { Self::new() }
}
