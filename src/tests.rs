//! Unit tests co-located with the library.

#[cfg(test)]
mod tests {
    use crate::constraint::*;
    use std::collections::HashMap;

    #[test]
    fn constraint_system_default() {
        let sys: ConstraintSystem<i64> = ConstraintSystem::default();
        assert_eq!(sys.constraint_count(), 0);
        assert_eq!(sys.variable_count(), 0);
    }

    #[test]
    fn constraint_system_counts() {
        let mut sys: ConstraintSystem<i64> = ConstraintSystem::new();
        sys.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
        sys.add_variable("y", Domain::Discrete(vec![1, 2]));
        sys.add_constraint(Constraint::equality("x", "y"));
        assert_eq!(sys.variable_count(), 2);
        assert_eq!(sys.constraint_count(), 1);
    }

    #[test]
    fn domain_size_and_empty() {
        let d = Domain::Discrete(vec![1, 2, 3]);
        assert_eq!(d.size(), Some(3));
        assert!(!d.is_empty());

        let empty = Domain::Discrete(Vec::<i64>::new());
        assert!(empty.is_empty());

        let c: Domain<i64> = Domain::Continuous { lo: 0.0, hi: 1.0 };
        assert!(c.size().is_none());
    }

    #[test]
    fn emergent_pattern_display() {
        use crate::emergence::PatternType;
        let pt = PatternType::Periodicity { period: 5 };
        assert!(pt.to_string().contains("5"));
    }
}
