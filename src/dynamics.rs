//! Constraint dynamics: velocity, momentum, and evolution over time.

use crate::constraint::{Constraint, ConstraintSystem, NumericValue};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintVelocity {
    pub constraint_idx: usize,
    pub rate: f64,
}

impl ConstraintVelocity {
    pub fn tightening(idx: usize, rate: f64) -> Self { Self { constraint_idx: idx, rate } }
    pub fn relaxing(idx: usize, rate: f64) -> Self { Self { constraint_idx: idx, rate: -rate } }
    pub fn is_static(&self) -> bool { self.rate.abs() < f64::EPSILON }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintMomentum {
    pub constraint_idx: usize,
    pub age: u64,
    pub inertia: f64,
}

impl ConstraintMomentum {
    pub fn new(idx: usize) -> Self { Self { constraint_idx: idx, age: 0, inertia: 1.0 } }

    pub fn effective_inertia(&self) -> f64 {
        self.inertia * (1.0 + (self.age as f64).ln().max(0.0))
    }

    pub fn tick(&mut self) { self.age += 1; }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintDynamics<V> {
    pub system: ConstraintSystem<V>,
    pub velocities: HashMap<usize, ConstraintVelocity>,
    pub momenta: HashMap<usize, ConstraintMomentum>,
    pub tick: u64,
    pub energy_history: Vec<f64>,
}

impl<V: NumericValue> ConstraintDynamics<V> {
    pub fn new(system: ConstraintSystem<V>) -> Self {
        let momenta = (0..system.constraints.len())
            .map(|i| (i, ConstraintMomentum::new(i)))
            .collect();
        Self { system, velocities: HashMap::new(), momenta, tick: 0, energy_history: Vec::new() }
    }

    pub fn add_constraint_dynamic(&mut self, c: Constraint<V>, velocity: Option<ConstraintVelocity>) -> usize {
        let idx = self.system.add_constraint(c);
        self.momenta.insert(idx, ConstraintMomentum::new(idx));
        if let Some(v) = velocity { self.velocities.insert(idx, v); }
        idx
    }

    pub fn remove_constraint(&mut self, idx: usize) -> Result<()> {
        if idx >= self.system.constraints.len() { anyhow::bail!("index {idx} out of range"); }
        self.velocities.remove(&idx);
        self.momenta.remove(&idx);
        Ok(())
    }

    pub fn step(&mut self) -> f64 {
        for m in self.momenta.values_mut() { m.tick(); }
        let energy = self.system.total_violation();
        self.energy_history.push(energy);
        self.tick += 1;
        energy
    }

    pub fn relax(&mut self, idx: usize, factor: f64) -> Result<()> {
        if idx >= self.system.constraints.len() { anyhow::bail!("index {idx} out of range"); }
        let current = self.system.weights.get(&idx).copied().unwrap_or(1.0);
        self.system.weights.insert(idx, current * factor);
        Ok(())
    }

    pub fn tighten(&mut self, idx: usize, factor: f64) -> Result<()> {
        if idx >= self.system.constraints.len() { anyhow::bail!("index {idx} out of range"); }
        let current = self.system.weights.get(&idx).copied().unwrap_or(1.0);
        self.system.weights.insert(idx, current * factor);
        Ok(())
    }

    pub fn is_converging(&self, window: usize) -> bool {
        if self.energy_history.len() < window { return false; }
        let recent: Vec<f64> = self.energy_history.iter().rev().take(window).copied().collect();
        recent[0] < *recent.last().unwrap()
    }

    pub fn energy_velocity(&self, window: usize) -> f64 {
        if self.energy_history.len() < 2 { return 0.0; }
        let n = window.min(self.energy_history.len());
        let recent: Vec<f64> = self.energy_history.iter().rev().take(n).copied().collect();
        if recent.len() < 2 { return 0.0; }
        (recent[0] - *recent.last().unwrap()) / (recent.len() - 1) as f64
    }
}
