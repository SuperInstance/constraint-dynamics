//! Tutorial: Constraint dynamics for fleet resource management
//!
//! Shows constraint systems, collision detection, arc consistency, energy landscapes.

use std::collections::HashMap;
use constraint_dynamics::constraint::{Constraint, ConstraintSystem};
use constraint_dynamics::propagation::ArcConsistency3;
use constraint_dynamics::energy::EnergyLandscape;

fn main() {
    println!("=== Constraint Dynamics Tutorial ===\n");

    // Part 1: Basic constraint system (use i64 — the NumericValue impl)
    println!("Part 1: Building a constraint system");
    let mut system: ConstraintSystem<i64> = ConstraintSystem::new();
    
    system.add_variable("gpu_a", constraint_dynamics::constraint::Domain::Discrete(vec![0i64, 1, 2, 3]));
    system.add_variable("gpu_b", constraint_dynamics::constraint::Domain::Discrete(vec![0i64, 1, 2, 3]));
    system.add_constraint(Constraint::inequality("gpu_a", "gpu_b"));
    println!("  2 variables, 1 inequality constraint\n");

    // Part 2: Arc consistency (AC-3)
    println!("Part 2: Arc consistency (AC-3)");
    match ArcConsistency3::run(system) {
        Ok(_refined) => println!("  AC-3 succeeded! Domains refined"),
        Err(e) => println!("  AC-3 failed: {}", e),
    }
    println!();

    // Part 3: Energy landscape (states are HashMap<String, i64>)
    println!("Part 4: Energy landscape");
    let mut landscape = EnergyLandscape::new();
    
    let s0: HashMap<String, i64> = [("gpu_a".into(), 0), ("gpu_b".into(), 0)].into();
    let s1: HashMap<String, i64> = [("gpu_a".into(), 1), ("gpu_b".into(), 2)].into();
    let s2: HashMap<String, i64> = [("gpu_a".into(), 3), ("gpu_b".into(), 1)].into();
    
    landscape.add_state(s0, 10.0);  // collision state
    landscape.add_state(s1, 2.0);   // partial
    landscape.add_state(s2, 0.0);   // solved
    
    landscape.add_transition(0, 1, 3.0);
    landscape.add_transition(1, 2, 1.0);
    
    if let Some((idx, energy)) = landscape.global_minimum() {
        println!("  Global minimum: state {} (energy={:.1})", idx, energy);
    }
    println!("  Local minima: {:?}", landscape.local_minima());
}
