//! # Constraint Dynamics
//!
//! The physics of constraints — how constraints propagate, collide, and create
//! emergent structure in agent systems.
//!
//! Constraints aren't limitations. They're the structure that makes complex
//! behavior possible. This library implements constraint propagation, conflict
//! resolution, energy landscapes, and emergent pattern detection.

pub mod collision;
pub mod constraint;
pub mod dynamics;
pub mod emergence;
pub mod energy;
pub mod propagation;
pub mod symmetry;

pub use collision::{Collision, ConflictResolution};
pub use constraint::{
    Constraint, ConstraintSystem, Domain,
    Satisfaction, SatisfactionLevel, NumericValue,
};
pub use dynamics::{ConstraintDynamics, ConstraintMomentum, ConstraintVelocity};
pub use emergence::{EmergentPattern, EmergenceDetector, PatternType};
pub use energy::{EnergyLandscape, GradientDescent, SimulatedAnnealing};
pub use propagation::{ArcConsistency3, ForwardChecker, PropagationEngine};
pub use symmetry::{SymmetryDetector, SymmetryGroup};

#[cfg(test)]
mod tests;
