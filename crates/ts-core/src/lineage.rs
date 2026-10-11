//! M4: Division, lineage, inheritance.
//!
//! Organisms in the primordial world can DIVIDE when they have enough
//! energy: the parent splits energy with the child, the child inherits a
//! (possibly mutated) genome, and both are recorded in a lineage tree.
//!
//! Explicit state taxonomy (research requirement):
//! - GENOME: heritable configuration (DivisionThreshold, AbsorbAffinity,
//!   SignalBias, MoveBias, MutationRate) — copied with mutation at division
//! - LIFETIME STATE: energy, position, memory (NOT inherited in v0)
//! - MODEL WEIGHTS: none (mock backends); when LLM backends arrive, weights
//!   are shared infrastructure, never per-organism genome

use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::wabix::WorldEvent;

/// Heritable genome. All fields are f32 in documented ranges; mutation is
/// clamped to keep genomes inside valid bounds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    /// Energy threshold above which the organism attempts division.
    pub division_threshold: f32,
    /// Multiplier on absorb efficiency (0.5..=1.5).
    pub absorb_affinity: f32,
    /// Bias toward signaling when energy is plentiful (0..=1).
    pub signal_bias: f32,
    /// Bias toward movement when energy is plentiful (0..=1).
    pub move_bias: f32,
    /// Per-trait mutation sigma at division (0..=0.5).
    pub mutation_rate: f32,
}

impl Default for Genome {
    fn default() -> Self {
        Self {
            division_threshold: 40.0,
            absorb_affinity: 1.0,
            signal_bias: 0.5,
            move_bias: 0.5,
            mutation_rate: 0.1,
        }
    }
}

impl Genome {
    /// Mutate: each trait moves by gaussian-ish noise scaled by mutation_rate,
    /// clamped to valid ranges. Deterministic given the RNG.
    pub fn mutate(&self, rng: &mut Rng) -> Genome {
        let mut jitter = |base: f32, sigma: f32, lo: f32, hi: f32| {
            // Symmetric uniform jitter as a cheap deterministic mutation;
            // gaussian can replace it later without changing the API.
            let u = rng.next_f32() * 2.0 - 1.0;
            (base + u * sigma).clamp(lo, hi)
        };
        let s = self.mutation_rate;
        // The mutation band is FIXED (3.0..=100.0), not parent-relative:
        // clamping the lower bound at the parent's value would make
        // downward mutation structurally impossible and turn any lineage
        // into a mechanical upward ratchet (observed: threshold 6.0 → 9.9
        // in 12 generations with zero selective advantage — pure clamp
        // artifact). With a symmetric band, the ecology does the
        // filtering: sub-6.0 thresholds fall into the over-reproduction
        // collapse regime (PR#65) and die there if they cannot sustain
        // themselves. Mutation proposes; selection disposes.
        Genome {
            division_threshold: jitter(self.division_threshold, s * 10.0, 3.0, 100.0),
            absorb_affinity: jitter(self.absorb_affinity, s * 0.2, 0.5, 1.5),
            signal_bias: jitter(self.signal_bias, s * 0.2, 0.0, 1.0),
            move_bias: jitter(self.move_bias, s * 0.2, 0.0, 1.0),
            mutation_rate: jitter(self.mutation_rate, s * 0.05, 0.0, 0.5),
        }
    }
}

/// Division policy: when and how an organism divides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DivisionPolicy {
    /// Fraction of parent energy transferred to the child (0.5 = even split).
    pub child_energy_fraction: f32,
    /// Fixed cost of division (energy lost to the process).
    pub division_cost: f32,
}

impl Default for DivisionPolicy {
    fn default() -> Self {
        Self {
            child_energy_fraction: 0.45,
            division_cost: 2.0,
        }
    }
}

/// Lineage record: one node per organism ever alive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineageNode {
    pub subject_id: String,
    /// Parent's subject_id; None for founders.
    pub parent: Option<String>,
    /// Lineage id: founders start a new one; children inherit.
    pub lineage_id: String,
    /// Genome at birth (post-mutation).
    pub genome_at_birth: Genome,
    /// Tick of birth (division).
    pub birth_tick: u64,
    /// Tick of death; None if still alive.
    pub death_tick: Option<u64>,
    /// Generation depth from founder (founder = 0).
    pub generation: u32,
}

/// Lineage tree with bounded memory; the full tree lives in the event log.
#[derive(Debug, Default)]
pub struct LineageTree {
    nodes: std::collections::VecDeque<LineageNode>,
    max_nodes: usize,
    dropped: usize,
}

impl LineageTree {
    pub fn new(max_nodes: usize) -> Self {
        Self {
            nodes: std::collections::VecDeque::new(),
            max_nodes,
            dropped: 0,
        }
    }

    pub fn record(&mut self, node: LineageNode) {
        self.nodes.push_back(node);
        while self.nodes.len() > self.max_nodes {
            self.nodes.pop_front();
            self.dropped += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &LineageNode> {
        self.nodes.iter()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Distinct lineage ids currently tracked.
    pub fn distinct_lineages(&self) -> usize {
        let mut seen: Vec<&str> = Vec::new();
        for n in &self.nodes {
            if !seen.contains(&n.lineage_id.as_str()) {
                seen.push(&n.lineage_id);
            }
        }
        seen.len()
    }

    /// Max generation depth among tracked nodes.
    pub fn max_generation(&self) -> u32 {
        self.nodes.iter().map(|n| n.generation).max().unwrap_or(0)
    }
}

/// Attempt division for an organism. Returns (child_pos, child_genome,
/// child_id) if division happened; reason string if rejected.
///
/// Division conditions (v0):
/// - organism alive
/// - energy >= genome.division_threshold
/// - division cost is affordable
pub fn attempt_division(
    parent_energy: f32,
    genome: &Genome,
    policy: &DivisionPolicy,
) -> Result<f32, String> {
    if parent_energy < genome.division_threshold {
        return Err(format!(
            "energy {:.2} below division threshold {:.2}",
            parent_energy, genome.division_threshold
        ));
    }
    if parent_energy - policy.division_cost <= 0.0 {
        return Err("division cost unaffordable".into());
    }
    let child_energy = (parent_energy - policy.division_cost) * policy.child_energy_fraction;
    Ok(child_energy)
}

/// Events specific to lineage (in addition to generic WorldEvent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineageEvent {
    Division {
        parent: String,
        child: String,
        lineage_id: String,
        generation: u32,
        tick: u64,
        child_genome: Genome,
        child_energy: f32,
    },
    DivisionRejected {
        subject: String,
        reason: String,
    },
    Death {
        subject: String,
        tick: u64,
        generation: u32,
    },
}

impl LineageEvent {
    /// Convert to generic world events for the unified log.
    pub fn to_world_events(&self) -> Vec<WorldEvent> {
        match self {
            LineageEvent::Division {
                parent,
                child,
                generation,
                ..
            } => vec![WorldEvent::ActionApplied {
                subject: parent.clone(),
                action: format!("DIVIDE->{child}(gen{generation})"),
                cost: 0.0,
            }],
            LineageEvent::DivisionRejected { subject, reason } => {
                vec![WorldEvent::ActionRejected {
                    subject: subject.clone(),
                    action: "DIVIDE".into(),
                    reason: reason.clone(),
                }]
            }
            LineageEvent::Death { .. } => vec![],
        }
    }
}

/// Spawn logic shared with the primordial world integration.
pub fn next_organism_id(counter: &mut u64) -> String {
    let id = format!("org-{:06}", counter);
    *counter += 1;
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_is_deterministic_and_bounded() {
        let g = Genome::default();
        let mut r1 = Rng::new(9);
        let mut r2 = Rng::new(9);
        let m1 = g.mutate(&mut r1);
        let m2 = g.mutate(&mut r2);
        assert_eq!(m1, m2, "same seed => same mutation");
        assert!((20.0..=100.0).contains(&m1.division_threshold));
        assert!((0.5..=1.5).contains(&m1.absorb_affinity));
        assert!((0.0..=1.0).contains(&m1.signal_bias));
    }

    #[test]
    fn division_requires_threshold() {
        let genome = Genome {
            division_threshold: 40.0,
            ..Default::default()
        };
        let policy = DivisionPolicy::default();
        assert!(attempt_division(39.9, &genome, &policy).is_err());
        let child = attempt_division(50.0, &genome, &policy).unwrap();
        // 50 - 2 cost = 48; child gets 45% = 21.6
        assert!((child - 21.6).abs() < 1e-5);
    }

    #[test]
    fn lineage_tree_tracks_generations() {
        let mut tree = LineageTree::new(1000);
        tree.record(LineageNode {
            subject_id: "root".into(),
            parent: None,
            lineage_id: "L1".into(),
            genome_at_birth: Genome::default(),
            birth_tick: 0,
            death_tick: None,
            generation: 0,
        });
        tree.record(LineageNode {
            subject_id: "child".into(),
            parent: Some("root".into()),
            lineage_id: "L1".into(),
            genome_at_birth: Genome::default(),
            birth_tick: 10,
            death_tick: Some(50),
            generation: 1,
        });
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.distinct_lineages(), 1);
        assert_eq!(tree.max_generation(), 1);
    }

    #[test]
    fn lineage_tree_bounds_memory() {
        let mut tree = LineageTree::new(4);
        for i in 0..10u32 {
            tree.record(LineageNode {
                subject_id: format!("o{i}"),
                parent: None,
                lineage_id: "L".into(),
                genome_at_birth: Genome::default(),
                birth_tick: 0,
                death_tick: None,
                generation: 0,
            });
        }
        assert_eq!(tree.len(), 4);
        assert_eq!(tree.dropped(), 6);
    }

    #[test]
    fn organism_ids_increment() {
        let mut counter = 0u64;
        assert_eq!(next_organism_id(&mut counter), "org-000000");
        assert_eq!(next_organism_id(&mut counter), "org-000001");
    }
}
