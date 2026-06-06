//! Emergent structure detection: clustering, periodicity, phase transitions.

use crate::constraint::ConstraintSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    Clustering,
    Periodicity { period: usize },
    PhaseTransition { threshold: f64 },
    Correlation,
    AntiCorrelation,
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::Clustering => write!(f, "clustering"),
            PatternType::Periodicity { period } => write!(f, "periodicity(p={period})"),
            PatternType::PhaseTransition { threshold } => write!(f, "phase_transition(t={threshold:.2})"),
            PatternType::Correlation => write!(f, "correlation"),
            PatternType::AntiCorrelation => write!(f, "anti_correlation"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentPattern {
    pub pattern_type: PatternType,
    pub variables: Vec<String>,
    pub confidence: f64,
    pub description: String,
}

impl EmergentPattern {
    pub fn new(pattern_type: PatternType, variables: Vec<String>, confidence: f64, description: impl Into<String>) -> Self {
        Self { pattern_type, variables, confidence, description: description.into() }
    }
}

pub struct EmergenceDetector;

impl EmergenceDetector {
    pub fn detect_clustering<V>(system: &ConstraintSystem<V>) -> Vec<EmergentPattern> {
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for c in &system.constraints {
            let vars = c.variables();
            for v in &vars {
                let neighbors: Vec<String> = vars.iter()
                    .filter(|v2| **v2 != *v).map(|s| s.to_string()).collect();
                adjacency.entry(v.to_string()).or_default().extend(neighbors);
            }
        }

        let mut sorted_adj: HashMap<String, Vec<String>> = HashMap::new();
        for (k, mut v) in adjacency { v.sort(); v.dedup(); sorted_adj.insert(k, v); }

        let mut clusters: Vec<Vec<String>> = Vec::new();
        let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();

        for v1 in sorted_adj.keys() {
            if assigned.contains(v1) { continue; }
            let n1 = &sorted_adj[v1];
            let mut cluster = vec![v1.clone()];
            assigned.insert(v1.clone());

            for v2 in sorted_adj.keys() {
                if assigned.contains(v2) { continue; }
                if sorted_adj.get(v2).map(|n| n == n1).unwrap_or(false) {
                    cluster.push(v2.clone());
                    assigned.insert(v2.clone());
                }
            }

            if cluster.len() > 1 { clusters.push(cluster); }
        }

        clusters.into_iter().map(|vars| {
            EmergentPattern::new(
                PatternType::Clustering, vars.clone(),
                1.0 / (1.0 + vars.len() as f64).ln(),
                format!("cluster of {} variables", vars.len()),
            )
        }).collect()
    }

    pub fn detect_phase_transition(energies: &[(f64, f64)]) -> Option<EmergentPattern> {
        if energies.len() < 3 { return None; }

        let mut max_jump = 0.0_f64;
        let mut threshold = 0.0_f64;

        for i in 1..energies.len() {
            let jump = (energies[i].1 - energies[i - 1].1).abs();
            if jump > max_jump {
                max_jump = jump;
                threshold = (energies[i].0 + energies[i - 1].0) / 2.0;
            }
        }

        let min_e = energies.iter().map(|(_, e)| *e).fold(f64::MAX, f64::min);
        let max_e = energies.iter().map(|(_, e)| *e).fold(f64::MIN, f64::max);
        let range = max_e - min_e;

        if range > 0.0 && max_jump / range > 0.3 {
            Some(EmergentPattern::new(
                PatternType::PhaseTransition { threshold },
                vec![],
                (max_jump / range).min(1.0),
                format!("phase transition at {threshold:.2}"),
            ))
        } else {
            None
        }
    }

    pub fn detect_periodicity(values: &[f64], variable: String) -> Vec<EmergentPattern> {
        let n = values.len();
        if n < 4 { return Vec::new(); }

        (2..=(n / 2)).filter_map(|period| {
            let total = n - period;
            let matches = (period..n)
                .filter(|i| (values[*i] - values[i - period]).abs() < 1e-6)
                .count();
            let confidence = matches as f64 / total as f64;
            if confidence > 0.8 {
                Some(EmergentPattern::new(
                    PatternType::Periodicity { period },
                    vec![variable.clone()],
                    confidence,
                    format!("periodic, period={period}"),
                ))
            } else {
                None
            }
        }).collect()
    }

    pub fn detect_correlation(
        values_a: &[f64], values_b: &[f64],
        name_a: String, name_b: String,
    ) -> Option<EmergentPattern> {
        let n = values_a.len().min(values_b.len());
        if n < 3 { return None; }

        let mean_a: f64 = values_a[..n].iter().sum::<f64>() / n as f64;
        let mean_b: f64 = values_b[..n].iter().sum::<f64>() / n as f64;

        let (cov, va, vb) = (0..n).fold((0.0, 0.0, 0.0), |(cov, va, vb), i| {
            let da = values_a[i] - mean_a;
            let db = values_b[i] - mean_b;
            (cov + da * db, va + da * da, vb + db * db)
        });

        if va < 1e-10 || vb < 1e-10 { return None; }

        let correlation = cov / (va.sqrt() * vb.sqrt());
        if correlation.abs() > 0.7 {
            Some(EmergentPattern::new(
                if correlation > 0.0 { PatternType::Correlation } else { PatternType::AntiCorrelation },
                vec![name_a, name_b],
                correlation.abs(),
                format!("correlation = {correlation:.3}"),
            ))
        } else {
            None
        }
    }
}
