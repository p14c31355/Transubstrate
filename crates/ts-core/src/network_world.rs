//! Network world v0 — the second world behind WABIX (M9).
//!
//! A structurally different world from the grid-based primordial world:
//! nodes connected by edges. Subjects occupy nodes; energy and a signal
//! field live on nodes; movement follows edges. This proves the WABIX
//! abstraction generalizes beyond grid worlds and gives M9 world-migration
//! experiments a structurally different destination.
//!
//! SEMANTIC LEAKAGE RULE (same as primordial): channels are numeric
//! (`node_energy`, `nearby_signal_0`, `edge_count`), actions are typed
//! (`WAIT`, `ABSORB`, `SIGNAL`, `HOP`). No "you are alive" prompts.

use crate::rng::Rng;
use crate::wabix::{
    Action, CapabilityManifest, ManifestBuilder, Observation, SubjectRef, WabixError, World,
    WorldEvent, validate_action,
};
use serde::{Deserialize, Serialize};

pub const NETWORK_WORLD_IMPLEMENTATION: &str = "network-v0";

pub fn network_manifest() -> CapabilityManifest {
    ManifestBuilder::new(NETWORK_WORLD_IMPLEMENTATION)
        .channel("node_energy", "scalar", (0.0, 1.0))
        .channel("nearby_signal_0", "scalar", (0.0, 1.0))
        .channel("degree", "scalar", (0.0, 10.0))
        .action("WAIT", 0, vec![])
        .action("ABSORB", 1, vec![(0.0, 1.0)])
        .action("SIGNAL", 1, vec![(0.0, 1.0)])
        .action("HOP", 1, vec![(0.0, 1.0)])
        .build()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Ring topology node count (simple, deterministic connectivity).
    pub nodes: u32,
    /// Node energy capacity (units).
    pub node_capacity: f32,
    /// Energy regeneration per node per tick.
    pub regen_rate: f32,
    /// Per-tick metabolism per subject.
    pub metabolism: f32,
    /// Fraction of ABSORB magnitude actually converted.
    pub absorb_efficiency: f32,
    /// Signal field decay per tick.
    pub signal_decay: f32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            nodes: 16,
            node_capacity: 10.0,
            regen_rate: 0.02,
            metabolism: 0.05,
            absorb_efficiency: 0.8,
            signal_decay: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkSubject {
    pub subject_id: String,
    pub node: u32,
    pub energy: f32,
    pub alive: bool,
}

/// Ring-topology network world. Subjects occupy nodes; energy and signals
/// live on nodes; HOP moves ±1 along the ring. Energy is shared per node
/// (all subjects on a node draw from the same pool).
pub struct NetworkWorld {
    pub config: NetworkConfig,
    manifest: CapabilityManifest,
    node_energy: Vec<f32>,
    signal_field: Vec<f32>,
    subjects: Vec<NetworkSubject>,
    rng: Rng,
    pub tick: u64,
}

/// Max energy extractable per ABSORB (world units).
pub const NET_MAX_ABSORB: f32 = 2.0;
/// Max signal strength per SIGNAL.
pub const NET_MAX_SIGNAL: f32 = 1.0;
/// Energy cost per unit of signal strength.
pub const NET_SIGNAL_COST: f32 = 0.5;
/// Energy cost per hop.
pub const HOP_COST: f32 = 0.1;
/// Observation normalization ceiling for node energy.
pub const NET_FIELD_NORMALIZATION: f32 = 10.0;

impl NetworkWorld {
    pub fn new(config: NetworkConfig, seed: u64) -> Self {
        let manifest = crate::wabix::ManifestBuilder::new(NETWORK_WORLD_IMPLEMENTATION)
            .channel("node_energy", "scalar", (0.0, 1.0))
            .channel("nearby_signal_0", "scalar", (0.0, 1.0))
            .channel("degree", "scalar", (0.0, 10.0))
            .action("WAIT", 0, vec![])
            .action("ABSORB", 1, vec![(0.0, 1.0)])
            .action("SIGNAL", 1, vec![(0.0, 1.0)])
            .action("HOP", 1, vec![(0.0, 1.0)])
            .build();
        let n = config.nodes as usize;
        Self {
            node_energy: vec![config.node_capacity * NET_FIELD_NORMALIZATION / 2.0; n],
            signal_field: vec![0.0; n],
            subjects: Vec::new(),
            rng: Rng::new(seed),
            manifest,
            config,
            tick: 0,
        }
    }

    fn idx(&self, node: u32) -> usize {
        node as usize
    }

    /// Spawn a subject on a random node.
    pub fn spawn(&mut self, subject_id: &str, start_energy: f32) -> u32 {
        let node = self.rng.next_range(self.config.nodes as u64) as u32;
        self.subjects.push(NetworkSubject {
            subject_id: subject_id.into(),
            node,
            energy: start_energy,
            alive: true,
        });
        node
    }

    /// Spawn a subject on a specific node (used by world-migration tests).
    pub fn spawn_at(&mut self, subject_id: &str, node: u32, start_energy: f32) {
        self.subjects.push(NetworkSubject {
            subject_id: subject_id.into(),
            node,
            energy: start_energy,
            alive: true,
        });
    }

    pub fn subject(&self, subject_id: &str) -> Option<&NetworkSubject> {
        self.subjects
            .iter()
            .find(|s| s.subject_id == subject_id && s.alive)
    }

    pub fn all_subjects(&self) -> impl Iterator<Item = &NetworkSubject> {
        self.subjects.iter()
    }

    pub fn alive_count(&self) -> usize {
        self.subjects.iter().filter(|s| s.alive).count()
    }

    pub fn node_energy(&self, node: u32) -> f32 {
        self.node_energy[self.idx(node)]
    }

    pub fn total_node_energy(&self) -> f32 {
        self.node_energy.iter().sum()
    }

    /// Ring neighbors (±1 with wraparound).
    fn ring_neighbors(&self, node: u32) -> [u32; 2] {
        let n = self.config.nodes;
        [(node + n - 1) % n, (node + 1) % n]
    }

    /// Physics: node energy regen + signal decay.
    fn step_physics(&mut self) {
        let regen = self.config.regen_rate * NET_FIELD_NORMALIZATION;
        for e in self.node_energy.iter_mut() {
            *e = (*e + regen).min(self.config.node_capacity * NET_FIELD_NORMALIZATION);
        }
        for s in self.signal_field.iter_mut() {
            *s *= 1.0 - self.config.signal_decay;
        }
    }

    /// Migrate an existing subject into this world at a specific node with
    /// carried-over energy. This is the entry point for world-migration
    /// experiments: the SubjectRef identity is preserved.
    pub fn admit_subject(&mut self, subject_id: &str, node: u32, energy: f32) {
        self.spawn_at(subject_id, node, energy);
    }

    /// Advance the world one tick: actions (in given order), metabolism,
    /// physics, death. Returns events in deterministic order.
    pub fn tick(&mut self, actions: &[(String, Action)]) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        for (subject_id, action) in actions {
            let Some(idx) = self
                .subjects
                .iter()
                .position(|s| s.subject_id == *subject_id && s.alive)
            else {
                continue;
            };
            if let Err(e) = self.validate(action) {
                events.push(WorldEvent::ActionRejected {
                    subject: subject_id.clone(),
                    action: action.name.clone(),
                    reason: e.to_string(),
                });
                continue;
            }
            let (evts, energy_delta) = self.apply_action(idx, action);
            self.subjects[idx].energy += energy_delta;
            events.extend(evts);
        }

        for s in &mut self.subjects {
            if s.alive {
                s.energy -= self.config.metabolism;
            }
        }

        self.step_physics();

        for s in &mut self.subjects {
            if s.alive && s.energy <= 0.0 {
                s.alive = false;
                events.push(WorldEvent::SubjectDied {
                    subject: s.subject_id.clone(),
                    cause: "energy exhausted".into(),
                });
            }
        }

        self.tick += 1;
        events
    }

    fn apply_action(&mut self, idx: usize, action: &Action) -> (Vec<WorldEvent>, f32) {
        let mut events = Vec::new();
        let subject = self.subjects[idx].subject_id.clone();
        let mut energy_delta = 0.0f32;

        match action.name.as_str() {
            "WAIT" => {
                events.push(WorldEvent::ActionApplied {
                    subject: subject.clone(),
                    action: "WAIT".into(),
                    cost: 0.0,
                });
            }
            "ABSORB" => {
                let want = action.params[0] * NET_MAX_ABSORB;
                let node = self.subjects[idx].node;
                let i = self.idx(node);
                let available = self.node_energy[i].min(want);
                self.node_energy[i] -= available;
                let gained = available * self.config.absorb_efficiency;
                energy_delta += gained;
                events.push(WorldEvent::ActionApplied {
                    subject: subject.clone(),
                    action: "ABSORB".into(),
                    cost: available,
                });
            }
            "SIGNAL" => {
                let strength = action.params[0] * NET_MAX_SIGNAL;
                let cost = strength * NET_SIGNAL_COST;
                if self.subjects[idx].energy >= cost {
                    self.subjects[idx].energy -= cost;
                    let node = self.subjects[idx].node;
                    let i = self.idx(node);
                    self.signal_field[i] += strength;
                    events.push(WorldEvent::ActionApplied {
                        subject: subject.clone(),
                        action: "SIGNAL".into(),
                        cost,
                    });
                } else {
                    events.push(WorldEvent::ActionRejected {
                        subject: subject.clone(),
                        action: "SIGNAL".into(),
                        reason: "insufficient energy for signal cost".into(),
                    });
                }
            }
            "HOP" => {
                // params[0] in 0..1: <0.5 = hop backward (−1), >=0.5 = forward (+1).
                let dir = if action.params[0] < 0.5 { -1i64 } else { 1 };
                let cost = HOP_COST;
                if self.subjects[idx].energy >= cost {
                    self.subjects[idx].energy -= cost;
                    let cur = self.subjects[idx].node;
                    let neighbors = self.ring_neighbors(cur);
                    let next = if dir > 0 { neighbors[1] } else { neighbors[0] };
                    self.subjects[idx].node = next;
                    events.push(WorldEvent::ActionApplied {
                        subject: subject.clone(),
                        action: "HOP".into(),
                        cost,
                    });
                } else {
                    events.push(WorldEvent::ActionRejected {
                        subject: subject.clone(),
                        action: "HOP".into(),
                        reason: "insufficient energy for hop cost".into(),
                    });
                }
            }
            other => {
                events.push(WorldEvent::ActionRejected {
                    subject,
                    action: other.into(),
                    reason: "not implemented in network-v0".into(),
                });
            }
        }
        (events, energy_delta)
    }
}

/// Observation normalization ceiling for node energy.
impl NetworkWorld {
    fn idx_usize(&self, node: u32) -> usize {
        node as usize
    }
}

pub const NETWORK_FIELD_NORMALIZATION: f32 = 10.0;

impl World for NetworkWorld {
    fn implementation(&self) -> &'static str {
        NETWORK_WORLD_IMPLEMENTATION
    }

    fn manifest(&self) -> &CapabilityManifest {
        &self.manifest
    }

    fn observe(&self, subject: &SubjectRef) -> Result<Vec<Observation>, WabixError> {
        let s = self.subject(&subject.subject_id).ok_or_else(|| {
            WabixError::UnknownChannel(format!("dead subject {}", subject.subject_id))
        })?;
        let i = self.idx_usize(s.node);
        Ok(vec![
            Observation::new(
                "node_energy",
                (self.node_energy[i] / NET_FIELD_NORMALIZATION).clamp(0.0, 1.0),
            ),
            Observation::new(
                "nearby_signal_0",
                (self.signal_field[i] / NET_FIELD_NORMALIZATION).clamp(0.0, 1.0),
            ),
            Observation::new("degree", 2.0),
        ])
    }

    fn validate(&self, action: &Action) -> Result<(), WabixError> {
        validate_action(&self.manifest, action)
    }

    fn step(&mut self, _subject: &SubjectRef, _action: Option<&Action>) -> Vec<WorldEvent> {
        // NetworkWorld drives all subjects internally via tick(); the generic
        // per-subject step interface is not used (same as primordial).
        unreachable!("network world uses tick(); per-subject step is not applicable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_hop_moves_subject() {
        let mut w = NetworkWorld::new(NetworkConfig::default(), 1);
        let node = w.spawn("s1", 10.0);
        let before = w.subject("s1").unwrap().node;
        let (_, _) = w.apply_action(
            0,
            &Action {
                name: "HOP".into(),
                params: vec![0.9], // forward
            },
        );
        let after = w.subject("s1").unwrap().node;
        assert_ne!(before, after, "HOP must move the subject along the ring");
        let _ = node;
    }

    #[test]
    fn absorb_draws_node_energy() {
        let mut w = NetworkWorld::new(NetworkConfig::default(), 2);
        let node = w.spawn("s1", 5.0);
        let before = w.node_energy(node);
        let (events, delta) = w.apply_action(
            0,
            &Action {
                name: "ABSORB".into(),
                params: vec![1.0],
            },
        );
        w.subjects[0].energy += delta;
        assert!(matches!(events[0], WorldEvent::ActionApplied { .. }));
        assert!(w.node_energy(node) < before);
    }

    #[test]
    fn metabolism_and_death() {
        let cfg = NetworkConfig {
            metabolism: 1.0,
            ..Default::default()
        };
        let mut w = NetworkWorld::new(cfg, 3);
        w.spawn("s1", 2.5);
        let _ = w.tick(&[]);
        assert!((w.subject("s1").unwrap().energy - 1.5).abs() < 1e-5);
        w.tick(&[]);
        let events = w.tick(&[]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WorldEvent::SubjectDied { subject, .. } if subject == "s1"))
        );
        assert_eq!(w.alive_count(), 0);
    }

    #[test]
    fn same_seed_same_trajectory() {
        let run = |seed: u64| {
            let mut w = NetworkWorld::new(NetworkConfig::default(), seed);
            w.spawn("a", 10.0);
            w.spawn("b", 10.0);
            let mut trace = Vec::new();
            for t in 0..20 {
                let actions = vec![
                    (
                        "a".to_string(),
                        Action {
                            name: if t % 2 == 0 {
                                "ABSORB".into()
                            } else {
                                "HOP".into()
                            },
                            params: if t % 2 == 0 { vec![0.8] } else { vec![0.1] },
                        },
                    ),
                    (
                        "b".to_string(),
                        Action {
                            name: "SIGNAL".into(),
                            params: vec![0.5],
                        },
                    ),
                ];
                let events = w.tick(&actions);
                trace.push(format!(
                    "t={} a=({},{:.3}) b=({},{:.3}) ev={}",
                    t,
                    w.subject("a").map(|s| s.node).unwrap_or(999),
                    w.subject("a").map(|s| s.energy).unwrap_or(-1.0),
                    w.subject("b").map(|s| s.node).unwrap_or(999),
                    w.subject("b").map(|s| s.energy).unwrap_or(-1.0),
                    events.len(),
                ));
            }
            trace
        };
        assert_eq!(run(123), run(123));
        assert_ne!(run(123), run(124));
    }

    #[test]
    fn admit_subject_preserves_identity() {
        let mut w = NetworkWorld::new(NetworkConfig::default(), 5);
        w.admit_subject("traveler", 3, 7.5);
        let s = w.subject("traveler").unwrap();
        assert_eq!(s.node, 3);
        assert!((s.energy - 7.5).abs() < 1e-5);
    }
}
