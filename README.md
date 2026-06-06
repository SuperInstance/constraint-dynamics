# constraint-dynamics

**The physics of constraints** — how constraints propagate, collide, and create emergent structure in agent systems.

This is the theoretical backbone of the [SuperInstance](https://github.com/SuperInstance) "constraint-aware AI" paradigm. Constraints aren't limitations — they're the structure that makes complex behavior possible.

## Philosophy

Think of constraints like particles in physics:

- **Propagation** is like a force — push on one constraint, and the effects ripple through the whole system.
- **Collision** is what happens when constraints conflict — they clash, and new structure emerges from the wreckage.
- **Energy** is the violation landscape — every state has an energy, and systems naturally flow toward low-energy configurations.
- **Emergence** is the magic — patterns that arise from constraint interactions but were never explicitly programmed.

## Architecture

```
constraint-dynamics/
├── constraint/     Core types: Equality, Inequality, Resource, Temporal, Dependency, MutualExclusion
├── propagation/    AC-3 arc consistency, forward checking, conflict-directed backjumping
├── dynamics/       Constraint evolution: velocity, momentum, relaxation, tightening
├── collision/      Conflict detection, unsatisfiable cores, resolution strategies
├── energy/         Energy landscapes, gradient descent, simulated annealing
├── emergence/      Clustering, periodicity, phase transitions, correlation detection
└── symmetry/       Group-theoretic symmetry detection and search-space reduction
```

## Quick Start

```rust
use constraint_dynamics::*;

// Build a constraint system
let mut system: ConstraintSystem<i64> = ConstraintSystem::new();
system.add_variable("x", Domain::Discrete(vec![1, 2, 3]));
system.add_variable("y", Domain::Discrete(vec![1, 2, 3]));
system.add_variable("z", Domain::Discrete(vec![1, 2, 3]));

// Add constraints (graph coloring K3 with 3 colors)
system.add_constraint(Constraint::inequality("x", "y"));
system.add_constraint(Constraint::inequality("y", "z"));
system.add_constraint(Constraint::inequality("x", "z"));

// Propagate to achieve arc consistency
let mut system = ArcConsistency3::run(system).unwrap();

// Assign and verify
system.assign("x", 1);
system.assign("y", 2);
system.assign("z", 3);
assert!((system.overall_satisfaction() - 1.0).abs() < 1e-9);
```

## Modules

### `constraint` — Core Types

Six fundamental constraint types, each with satisfaction evaluation:

| Type | Meaning | Example |
|------|---------|---------|
| `Equality` | Two variables must match | `x == y` |
| `Inequality` | Two variables must differ | `x ≠ y` |
| `Resource` | Budget shared among usages | `sum(usages) ≤ total` |
| `Temporal` | Ordering with minimum gap | `B starts ≥ 5s after A` |
| `Dependency` | One variable requires another | `flag must be set for action` |
| `MutualExclusion` | No two variables share a value | All-different |

### `propagation` — The Ripple Effect

When one constraint changes, the effects propagate:

- **AC-3**: Achieves arc consistency by iteratively removing unsupported domain values.
- **Forward Checking**: Prunes future domains when a variable is assigned.
- **Conflict-Directed Backjumping**: When dead-ended, jumps back to the actual cause instead of chronologically.

```rust
let ok = PropagationEngine::ac3(&mut system).unwrap(); // true = consistent
ForwardChecker::check(&mut system, "x").unwrap();       // prunes y's domain
```

### `dynamics` — Constraints Over Time

Constraints aren't static. They evolve:

- **Velocity**: Rate of change (tightening vs. relaxing).
- **Momentum**: Resistance to change, growing with age.
- **Convergence tracking**: Is the system settling down?

```rust
let mut dynamics = ConstraintDynamics::new(system);
dynamics.relax(0, 0.5)?;   // soften constraint 0
dynamics.tighten(1, 2.0)?;  // harden constraint 1
let energy = dynamics.step(); // advance one tick
```

### `collision` — When Constraints Clash

Like particle physics: constraints can collide, and the debris is interesting.

- Detect conflicting constraint groups.
- Find minimal unsatisfiable cores.
- Resolve via relaxation ordering.

```rust
let collisions = CollisionDetector::detect(&system);
for collision in &collisions {
    CollisionDetector::resolve(&mut system, collision)?;
}
```

### `energy` — The Violation Landscape

Every assignment has an energy = weighted sum of violations. Lower is better.

- **Energy Landscape**: Map of states, energies, and transitions.
- **Gradient Descent**: Follow the gradient to low-energy states.
- **Simulated Annealing**: Escape local minima via probabilistic acceptance of worse states.

```rust
let landscape = EnergyLandscape::new();
let (min_idx, min_energy) = landscape.global_minimum().unwrap();
let local_minima = landscape.local_minima();

let (energy, assignments) = SimulatedAnnealing::new(10.0, 0.99, 1000)
    .run(&mut system, &domains, 42);
```

### `emergence` — Patterns That Weren't Programmed

Structure arising from constraint interactions:

- **Clustering**: Variables with identical constraint neighborhoods.
- **Periodicity**: Cycling through values with fixed periods.
- **Phase Transitions**: Sudden shifts in satisfaction at critical parameter values.
- **Correlation / Anti-correlation**: Variables that move together or in opposition.

```rust
let clusters = EmergenceDetector::detect_clustering(&system);
let phase = EmergenceDetector::detect_phase_transition(&energy_series);
let periodic = EmergenceDetector::detect_periodicity(&values, "x".into());
let corr = EmergenceDetector::detect_correlation(&a, &b, "a".into(), "b".into());
```

### `symmetry` — Don't Search the Same Thing Twice

If swapping variables A and B gives an equivalent system, exploit it:

- Detect symmetry groups (variables with identical constraint roles).
- Compute search-space reduction factors (n! for a group of n).
- Break symmetries to eliminate redundant solutions.

```rust
let groups = SymmetryDetector::detect(&system);
let reduction = SymmetryDetector::total_reduction(&groups);
// reduction = product of factorials of group sizes
```

## Core Types

```rust
// Constraint variants
enum Constraint<V> {
    Equality { a, b },
    Inequality { a, b },
    Resource { name, total, usage_vars },
    Temporal { before, after, gap: Duration },
    Dependency { requires, enables },
    MutualExclusion { variables },
}

// The system: variables + constraints + assignments
struct ConstraintSystem<V> {
    constraints: Vec<Constraint<V>>,
    variables: HashMap<String, Domain<V>>,
    assignments: HashMap<String, V>,
    weights: HashMap<usize, f64>,
    propagation_queue: Vec<usize>,
}

// Energy landscape
struct EnergyLandscape {
    states: Vec<State>,
    energy: Vec<f64>,
    transitions: Vec<(usize, usize, f64)>,
}

// Emergent patterns
struct EmergentPattern {
    pattern_type: PatternType,
    variables: Vec<String>,
    confidence: f64,
}
```

## Testing

45 tests covering all modules:

```bash
cargo test
```

## License

MIT
