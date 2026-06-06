//! Comprehensive integration tests.

#[cfg(test)]
mod tests {
    use constraint_dynamics::collision::{CollisionDetector, ConflictResolution};
    use constraint_dynamics::constraint::*;
    use constraint_dynamics::dynamics::*;
    use constraint_dynamics::emergence::*;
    use constraint_dynamics::energy::*;
    use constraint_dynamics::propagation::*;
    use constraint_dynamics::symmetry::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn simple_system() -> ConstraintSystem<i64> {
        let mut sys = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("z", Domain::Discrete(vec![1, 2, 3]));
        sys
    }

    // ── Constraint basics ─────────────────────────────────────────────

    #[test]
    fn test_equality_satisfied() {
        let c: Constraint<i64> = Constraint::equality("x", "y");
        let mut assign = HashMap::new();
        assign.insert("x".into(), 5);
        assign.insert("y".into(), 5);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_equality_violated() {
        let c: Constraint<i64> = Constraint::equality("x", "y");
        let mut assign = HashMap::new();
        assign.insert("x".into(), 5);
        assign.insert("y".into(), 3);
        assert!(c.evaluate(&assign).is_violated());
    }

    #[test]
    fn test_inequality_satisfied() {
        let c: Constraint<i64> = Constraint::inequality("x", "y");
        let mut assign = HashMap::new();
        assign.insert("x".into(), 5);
        assign.insert("y".into(), 3);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_inequality_violated() {
        let c: Constraint<i64> = Constraint::inequality("x", "y");
        let mut assign = HashMap::new();
        assign.insert("x".into(), 5);
        assign.insert("y".into(), 5);
        assert!(c.evaluate(&assign).is_violated());
    }

    #[test]
    fn test_resource_constraint_ok() {
        let c: Constraint<i64> = Constraint::resource("budget", 10.0, vec!["a".into(), "b".into()]);
        let mut assign = HashMap::new();
        assign.insert("a".into(), 4);
        assign.insert("b".into(), 5);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_resource_constraint_over() {
        let c: Constraint<i64> = Constraint::resource("budget", 10.0, vec!["a".into(), "b".into()]);
        let mut assign = HashMap::new();
        assign.insert("a".into(), 7);
        assign.insert("b".into(), 5);
        assert!(c.evaluate(&assign).is_violated());
    }

    #[test]
    fn test_temporal_constraint_ok() {
        let c: Constraint<i64> = Constraint::temporal("start", "end", Duration::from_secs(5));
        let mut assign = HashMap::new();
        assign.insert("start".into(), 0);
        assign.insert("end".into(), 10);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_temporal_constraint_violated() {
        let c: Constraint<i64> = Constraint::temporal("start", "end", Duration::from_secs(5));
        let mut assign = HashMap::new();
        assign.insert("start".into(), 8);
        assign.insert("end".into(), 10);
        assert!(c.evaluate(&assign).is_violated());
    }

    #[test]
    fn test_dependency_satisfied() {
        let c: Constraint<i64> = Constraint::dependency("flag", "action");
        let mut assign = HashMap::new();
        assign.insert("flag".into(), 1);
        assign.insert("action".into(), 1);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_dependency_violated() {
        let c: Constraint<i64> = Constraint::dependency("flag", "action");
        let mut assign = HashMap::new();
        assign.insert("action".into(), 1);
        assert!(c.evaluate(&assign).is_violated());
    }

    #[test]
    fn test_mutual_exclusion_ok() {
        let c: Constraint<i64> = Constraint::mutual_exclusion(vec!["x".into(), "y".into(), "z".into()]);
        let mut assign = HashMap::new();
        assign.insert("x".into(), 1);
        assign.insert("y".into(), 2);
        assign.insert("z".into(), 3);
        assert!(c.evaluate(&assign).is_satisfied());
    }

    #[test]
    fn test_mutual_exclusion_violated() {
        let c: Constraint<i64> = Constraint::mutual_exclusion(vec!["x".into(), "y".into()]);
        let mut assign = HashMap::new();
        assign.insert("x".into(), 5);
        assign.insert("y".into(), 5);
        assert!(c.evaluate(&assign).is_violated());
    }

    // ── System ────────────────────────────────────────────────────────

    #[test]
    fn test_system_overall_satisfaction() {
        let mut sys = simple_system();
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.add_constraint(Constraint::inequality("y", "z"));
        sys.assign("x", 1);
        sys.assign("y", 1);
        sys.assign("z", 2);
        assert!((sys.overall_satisfaction() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_system_total_violation() {
        let mut sys = simple_system();
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.assign("x", 1);
        sys.assign("y", 2);
        assert!(sys.total_violation() > 0.0);
    }

    #[test]
    fn test_violated_constraints_tracking() {
        let mut sys = simple_system();
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.add_constraint(Constraint::inequality("x", "y"));
        sys.assign("x", 1);
        sys.assign("y", 2);
        let violated = sys.violated_constraints();
        assert_eq!(violated.len(), 1);
    }

    // ── AC-3 ──────────────────────────────────────────────────────────

    #[test]
    fn test_ac3_arc_consistency() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2]));
        sys.add_constraint(Constraint::equality("x", "y"));

        let ok = PropagationEngine::ac3(&mut sys).unwrap();
        assert!(ok);

        if let Domain::Discrete(ref vals) = sys.variables["x"] {
            assert!(vals.contains(&1));
            assert!(vals.contains(&2));
            assert!(!vals.contains(&3));
        } else { panic!("discrete expected"); }
    }

    #[test]
    fn test_ac3_detects_unsatisfiable() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1]));
        sys.add_variable("y", Domain::Discrete(vec![2]));
        sys.add_constraint(Constraint::equality("x", "y"));
        assert!(!PropagationEngine::ac3(&mut sys).unwrap());
    }

    #[test]
    fn test_ac3_propagation_terminates() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        for i in 0..10 {
            sys.add_variable(format!("v{i}"), Domain::Discrete(vec![1, 2, 3]));
        }
        for i in 0..9 {
            sys.add_constraint(Constraint::inequality(format!("v{i}"), format!("v{}", i + 1)));
        }
        assert!(PropagationEngine::ac3(&mut sys).unwrap());
    }

    // ── Forward checking ──────────────────────────────────────────────

    #[test]
    fn test_forward_checking_prunes() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.assign("x", 2);
        assert!(ForwardChecker::check(&mut sys, "x").unwrap());

        if let Domain::Discrete(ref vals) = sys.variables["y"] {
            assert_eq!(vals.len(), 1);
            assert!(vals.contains(&2));
        }
    }

    // ── Dynamics ──────────────────────────────────────────────────────

    #[test]
    fn test_constraint_dynamics_step() {
        let mut sys = simple_system();
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.assign("x", 1);
        sys.assign("y", 2);
        let mut dynamics = ConstraintDynamics::new(sys);
        let energy = dynamics.step();
        assert!(energy > 0.0);
        assert_eq!(dynamics.tick, 1);
    }

    #[test]
    fn test_constraint_momentum_grows() {
        let m = ConstraintMomentum::new(0);
        assert!((m.effective_inertia() - 1.0).abs() < 1e-9);
        let aged = ConstraintMomentum { age: 100, ..ConstraintMomentum::new(0) };
        assert!(aged.effective_inertia() > 1.0);
    }

    #[test]
    fn test_relax_reduces_weight() {
        let mut sys = simple_system();
        let idx = sys.add_constraint(Constraint::equality("x", "y"));
        let mut dynamics = ConstraintDynamics::new(sys);
        dynamics.relax(idx, 0.5).unwrap();
        assert_eq!(dynamics.system.weights[&idx], 0.5);
    }

    // ── Collision ─────────────────────────────────────────────────────

    #[test]
    fn test_collision_detection() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2]));
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.add_constraint(Constraint::inequality("x", "y"));
        sys.assign("x", 1);
        sys.assign("y", 2);
        assert!(!CollisionDetector::detect(&sys).is_empty());
    }

    #[test]
    fn test_collision_resolution() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1]));
        sys.add_variable("y", Domain::Discrete(vec![2]));
        sys.add_constraint(Constraint::equality("x", "y"));
        sys.assign("x", 1);
        sys.assign("y", 2);
        let collisions = CollisionDetector::detect(&sys);
        if let Some(collision) = collisions.first() {
            CollisionDetector::resolve(&mut sys, collision).unwrap();
            if let ConflictResolution::RelaxLowest { relax_idx, factor } = &collision.resolution {
                assert_eq!(sys.weights[relax_idx], *factor);
            }
        }
    }

    // ── Energy landscape ──────────────────────────────────────────────

    #[test]
    fn test_energy_landscape_minima() {
        let mut landscape = EnergyLandscape::new();
        let s1: State = HashMap::from([("x".into(), 1)]);
        let s2: State = HashMap::from([("x".into(), 2)]);
        let s3: State = HashMap::from([("x".into(), 3)]);
        let i1 = landscape.add_state(s1, 2.0);
        let i2 = landscape.add_state(s2, 0.5);
        let i3 = landscape.add_state(s3, 1.0);
        landscape.add_transition(i1, i2, 0.5);
        landscape.add_transition(i2, i3, 0.3);
        let (min_idx, min_energy) = landscape.global_minimum().unwrap();
        assert_eq!(min_idx, i2);
        assert!((min_energy - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_local_minima_detection() {
        let mut landscape = EnergyLandscape::new();
        let i1 = landscape.add_state(HashMap::from([("x".into(), 1)]), 3.0);
        let i2 = landscape.add_state(HashMap::from([("x".into(), 2)]), 1.0);
        let i3 = landscape.add_state(HashMap::from([("x".into(), 3)]), 2.0);
        landscape.add_transition(i1, i2, 1.0);
        landscape.add_transition(i2, i3, 0.5);
        let minima = landscape.local_minima();
        assert!(minima.contains(&i2));
    }

    // ── Simulated annealing ───────────────────────────────────────────

    #[test]
    fn test_simulated_annealing_finds_low_energy() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_constraint(Constraint::equality("x", "y"));
        let domains = HashMap::from([("x".into(), vec![1, 2, 3]), ("y".into(), vec![1, 2, 3])]);
        let sa = SimulatedAnnealing::new(10.0, 0.99, 1000);
        let (energy, assignments) = sa.run(&mut sys, &domains, 42);
        assert!(energy < 1.0);
        assert_eq!(assignments.get("x"), assignments.get("y"));
    }

    // ── Emergence ─────────────────────────────────────────────────────

    #[test]
    fn test_clustering_detection() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("a", Domain::Discrete(vec![1, 2]));
        sys.add_variable("x", Domain::Discrete(vec![1, 2]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2]));
        sys.add_variable("z", Domain::Discrete(vec![1, 2]));
        sys.add_constraint(Constraint::inequality("a", "x"));
        sys.add_constraint(Constraint::inequality("a", "y"));
        sys.add_constraint(Constraint::inequality("a", "z"));
        let clusters = EmergenceDetector::detect_clustering(&sys);
        assert!(!clusters.is_empty());
        assert!(clusters.iter().any(|c|
            c.variables.contains(&"x".to_string())
            && c.variables.contains(&"y".to_string())
            && c.variables.contains(&"z".to_string())
        ));
    }

    #[test]
    fn test_phase_transition_detection() {
        let energies: Vec<(f64, f64)> = (0..10).map(|i| {
            (i as f64, if i < 5 { 0.1 } else { 5.0 })
        }).collect();
        let result = EmergenceDetector::detect_phase_transition(&energies);
        assert!(result.is_some());
        if let Some(p) = result {
            if let PatternType::PhaseTransition { threshold } = p.pattern_type {
                assert!((threshold - 4.5).abs() < 1.0);
            }
        }
    }

    #[test]
    fn test_periodicity_detection() {
        let values: Vec<f64> = vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0];
        let patterns = EmergenceDetector::detect_periodicity(&values, "x".to_string());
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| matches!(
            &p.pattern_type, PatternType::Periodicity { period } if *period == 3
        )));
    }

    #[test]
    fn test_correlation_detection() {
        let a: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b: Vec<f64> = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let result = EmergenceDetector::detect_correlation(&a, &b, "a".to_string(), "b".to_string());
        assert!(result.is_some());
        assert!(matches!(result.unwrap().pattern_type, PatternType::Correlation));
    }

    #[test]
    fn test_anti_correlation_detection() {
        let a: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b: Vec<f64> = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let result = EmergenceDetector::detect_correlation(&a, &b, "a".to_string(), "b".to_string());
        assert!(result.is_some());
        assert!(matches!(result.unwrap().pattern_type, PatternType::AntiCorrelation));
    }

    // ── Symmetry ──────────────────────────────────────────────────────

    #[test]
    fn test_symmetry_detection() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("a", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_constraint(Constraint::inequality("a", "x"));
        sys.add_constraint(Constraint::inequality("a", "y"));
        let groups = SymmetryDetector::detect(&sys);
        assert!(!groups.is_empty());
        assert!(groups.iter().any(|g|
            g.variables.contains(&"x".to_string()) && g.variables.contains(&"y".to_string())
        ));
    }

    #[test]
    fn test_symmetry_reduction_factor() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("a", Domain::Discrete(vec![1, 2, 3]));
        for c in ['x', 'y', 'z'] {
            sys.add_variable(c.to_string(), Domain::Discrete(vec![1, 2, 3]));
            sys.add_constraint(Constraint::inequality("a", c.to_string()));
        }
        let groups = SymmetryDetector::detect(&sys);
        let total = SymmetryDetector::total_reduction(&groups);
        assert!(total >= 6);
    }

    #[test]
    fn test_no_false_symmetry() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_constraint(Constraint::temporal("x", "y", Duration::from_secs(1)));
        let groups = SymmetryDetector::detect(&sys);
        assert!(groups.is_empty());
    }

    // ── Backjumping ───────────────────────────────────────────────────

    #[test]
    fn test_backjump_state() {
        let mut bj = BackjumpState::new();
        bj.assignment_order = vec!["a".into(), "b".into(), "c".into()];
        bj.record_conflict("c", "a");
        bj.record_conflict("c", "b");
        let target = bj.backjump_target("c").unwrap();
        assert_eq!(target, 1);
    }

    // ── Integration ───────────────────────────────────────────────────

    #[test]
    fn test_full_pipeline() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("z", Domain::Discrete(vec![1, 2, 3]));
        sys.add_constraint(Constraint::inequality("x", "y"));
        sys.add_constraint(Constraint::inequality("y", "z"));
        sys.add_constraint(Constraint::inequality("x", "z"));

        let result = ArcConsistency3::run(sys);
        assert!(result.is_ok());

        let mut sys = result.unwrap();
        sys.assign("x", 1);
        sys.assign("y", 2);
        sys.assign("z", 3);
        assert!((sys.overall_satisfaction() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_temporal_scheduling() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("task_a", Domain::Discrete(vec![0, 1, 2, 3, 4, 5]));
        sys.add_variable("task_b", Domain::Discrete(vec![0, 1, 2, 3, 4, 5]));
        sys.add_variable("task_c", Domain::Discrete(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
        sys.add_constraint(Constraint::temporal("task_a", "task_b", Duration::from_secs(2)));
        sys.add_constraint(Constraint::temporal("task_b", "task_c", Duration::from_secs(3)));
        sys.assign("task_a", 0);
        sys.assign("task_b", 2);
        sys.assign("task_c", 5);
        assert!(sys.overall_satisfaction() > 0.99);
    }

    #[test]
    fn test_domain_restrict() {
        let mut d = Domain::Discrete(vec![1, 2, 3, 4, 5]);
        d.restrict_to(&[2, 4]);
        if let Domain::Discrete(ref vals) = d { assert_eq!(vals.len(), 2); }
    }

    #[test]
    fn test_domain_remove() {
        let mut d = Domain::Discrete(vec![1, 2, 3]);
        d.remove(&2);
        if let Domain::Discrete(ref vals) = d { assert_eq!(vals, &vec![1, 3]); }
    }

    #[test]
    fn test_satisfaction_levels() {
        assert!(SatisfactionLevel::Satisfied.score() > 0.99);
        assert!((SatisfactionLevel::Partial(0.6).score() - 0.6).abs() < 1e-9);
        assert!(SatisfactionLevel::Violated.score() < 0.01);
    }
}
