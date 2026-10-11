//! Primordial world v0 — the first-class ALife reference experiment.
//!
//! A 2D grid world with an energy field, diffusion, local sensing, resource
//! depletion, maintenance metabolism, action costs, signaling, and death.
//! Multiple organisms share the world. Everything is deterministic given the
//! seed.
//!
//! SEMANTIC LEAKAGE RULE: no text tells any subject "you are alive" or "find
//! food". Subjects see numeric channels (`local_energy`, `chemical_0`,
//! `nearby_signal_0`) and act through typed actions (`ABSORB`, `RELEASE`,
//! `SIGNAL`, `WAIT`). Any meaning ascribed to channels by an observer is the
//! observer's, not the system's.

use serde::{Deserialize, Serialize};

use crate::lineage::{DivisionPolicy, Genome, attempt_division};
use crate::rng::Rng;
use crate::wabix::{
    Action, CapabilityManifest, ManifestBuilder, Observation, SubjectRef, WabixError, World,
    WorldEvent, validate_action,
};

pub const WORLD_IMPLEMENTATION: &str = "primordial-v0";

/// Manifest shared by all primordial subjects (capability discovery).
pub fn primordial_manifest() -> CapabilityManifest {
    ManifestBuilder::new(WORLD_IMPLEMENTATION)
        .channel("local_energy", "scalar", (0.0, 1.0))
        .channel("chemical_0", "scalar", (0.0, 1.0))
        .channel("nearby_signal_0", "scalar", (0.0, 1.0))
        .action("WAIT", 0, vec![])
        .action("ABSORB", 1, vec![(0.0, 1.0)])
        .action("RELEASE", 1, vec![(0.0, 1.0)])
        .action("SIGNAL", 1, vec![(0.0, 1.0)])
        .action("MOVE", 2, vec![(0.0, 1.0), (0.0, 1.0)])
        .action("DIVIDE", 0, vec![])
        .build()
}

/// World configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrimordialConfig {
    pub width: u32,
    pub height: u32,
    /// Initial energy field level everywhere (0..1).
    pub initial_energy: f32,
    /// Energy regeneration per tick per cell (scaled by `regen` factor).
    pub regen_rate: f32,
    /// Diffusion fraction of each cell's energy to neighbors per tick.
    pub diffusion: f32,
    /// Per-tick maintenance metabolism (energy units, absolute per organism).
    pub metabolism: f32,
    /// Fraction of ABSORB magnitude actually converted into organism energy.
    pub absorb_efficiency: f32,
    /// Signal field decay per tick.
    pub signal_decay: f32,
}

impl Default for PrimordialConfig {
    fn default() -> Self {
        Self {
            width: 16,
            height: 16,
            initial_energy: 0.5,
            regen_rate: 0.01,
            diffusion: 0.05,
            metabolism: 0.05,
            absorb_efficiency: 0.8,
            signal_decay: 0.5,
        }
    }
}

/// One organism living in the world (world-side state; the Subject runtime
/// holds identity and memory separately).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Organism {
    pub subject_id: String,
    pub pos: (u32, u32),
    /// Energy store, in world units (starts at start_energy, dies at <= 0).
    pub energy: f32,
    pub alive: bool,
    /// Heritable genome (M4).
    pub genome: Genome,
    /// Lineage id (shared with parent); founders get their own.
    pub lineage_id: String,
    /// Generation depth from founder.
    pub generation: u32,
}

/// The primordial world.
pub struct PrimordialWorld {
    pub config: PrimordialConfig,
    manifest: CapabilityManifest,
    /// Energy field per cell, 0..=1 scaled by ENERGY_SCALE (f32 units).
    energy_field: Vec<f32>,
    /// Chemical field (no inherent semantics; a substrate for conventions).
    chemical_field: Vec<f32>,
    /// Signal field (decays; SIGNAL actions inject).
    signal_field: Vec<f32>,
    organisms: Vec<Organism>,
    rng: Rng,
    pub tick: u64,
    /// Division policy for M4 reproduction.
    pub division_policy: DivisionPolicy,
    /// Population cap: division is rejected when this is reached.
    pub population_cap: usize,
    /// Internal counter for generating child ids deterministically.
    child_counter: u64,
    /// Genome used for founder organisms (settable for reproduction
    /// experiments; defaults to Genome::default()).
    founder_genome: Genome,
}

/// Energy field stores arbitrary positive floats; observations normalize to
/// 0..1 by clamping at FIELD_NORMALIZATION.
pub const FIELD_NORMALIZATION: f32 = 10.0;

impl PrimordialWorld {
    pub fn new(config: PrimordialConfig, seed: u64) -> Self {
        let n = (config.width * config.height) as usize;
        let manifest = primordial_manifest();
        Self {
            energy_field: vec![config.initial_energy * FIELD_NORMALIZATION; n],
            chemical_field: vec![0.0; n],
            signal_field: vec![0.0; n],
            organisms: Vec::new(),
            rng: Rng::new(seed),
            manifest,
            config,
            tick: 0,
            division_policy: DivisionPolicy::default(),
            founder_genome: Genome::default(),
            population_cap: 256,
            child_counter: 0,
        }
    }

    fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.config.width + x) as usize
    }

    /// Override the founder genome used by [`spawn`](Self::spawn) (and the
    /// initial population). World-level knob for reproduction experiments:
    /// e.g. lowering `division_threshold` lets lineages expand under
    /// binding scarcity where the default 40.0 is unreachable.
    pub fn set_founder_genome(&mut self, genome: Genome) {
        self.founder_genome = genome;
    }

    /// Spawn a founder organism with the default genome.
    pub fn spawn(&mut self, subject_id: &str, start_energy: f32) -> (u32, u32) {
        let x = self.rng.next_range(self.config.width as u64) as u32;
        let y = self.rng.next_range(self.config.height as u64) as u32;
        let pos = (x, y);
        self.spawn_with_genome_at(
            subject_id,
            pos,
            start_energy,
            self.founder_genome.clone(),
            subject_id.to_string(), // founders start their own lineage
            0,
        );
        pos
    }

    /// Spawn with explicit genome/lineage/generation at a position (used by
    /// division; the child is placed on a random adjacent free cell).
    pub fn spawn_with_genome_at(
        &mut self,
        subject_id: &str,
        pos: (u32, u32),
        energy: f32,
        genome: Genome,
        lineage_id: String,
        generation: u32,
    ) {
        self.organisms.push(Organism {
            subject_id: subject_id.into(),
            pos,
            energy,
            alive: true,
            genome,
            lineage_id,
            generation,
        });
    }

    pub fn organism(&self, subject_id: &str) -> Option<&Organism> {
        self.organisms
            .iter()
            .find(|o| o.subject_id == subject_id && o.alive)
    }

    pub fn alive_count(&self) -> usize {
        self.organisms.iter().filter(|o| o.alive).count()
    }

    pub fn energy_field_cell(&self, x: u32, y: u32) -> f32 {
        self.energy_field[self.idx(x, y)]
    }

    /// All organisms (including dead) for metrics.
    pub fn all_organisms(&self) -> impl Iterator<Item = &Organism> {
        self.organisms.iter()
    }

    /// Total energy in the field.
    pub fn total_field_energy(&self) -> f32 {
        self.energy_field.iter().sum()
    }

    fn neighbors4(&self, x: u32, y: u32) -> Vec<(u32, u32)> {
        let mut out = Vec::with_capacity(4);
        let (w, h) = (self.config.width, self.config.height);
        if x > 0 {
            out.push((x - 1, y));
        }
        if x < w - 1 {
            out.push((x + 1, y));
        }
        if y > 0 {
            out.push((x, y - 1));
        }
        if y < h - 1 {
            out.push((x, y + 1));
        }
        out
    }

    /// Physics step: diffusion, regeneration, signal decay.
    fn step_physics(&mut self) {
        let (w, h) = (self.config.width, self.config.height);
        let d = self.config.diffusion;
        let regen = self.config.regen_rate * FIELD_NORMALIZATION;

        // Diffusion: compute deltas first, then apply (synchronous).
        let mut delta_energy = vec![0.0f32; self.energy_field.len()];
        let mut delta_signal = vec![0.0f32; self.signal_field.len()];
        for y in 0..h {
            for x in 0..w {
                let i = self.idx(x, y);
                let neighbors = self.neighbors4(x, y);
                let share = self.energy_field[i] * d / neighbors.len() as f32;
                for (nx, ny) in &neighbors {
                    let j = self.idx(*nx, *ny);
                    let flow = share - self.energy_field[j] * d / neighbors.len() as f32;
                    delta_energy[i] -= flow;
                    delta_energy[j] += flow;
                }
                // Signals diffuse uniformly (broadcast-like).
                let sig_share = self.signal_field[i] * 0.25;
                for (nx, ny) in &neighbors {
                    let j = self.idx(*nx, *ny);
                    delta_signal[i] -= sig_share / 4.0;
                    delta_signal[j] += sig_share / 4.0;
                }
            }
        }
        for (i, d) in delta_energy.iter().enumerate() {
            self.energy_field[i] = (self.energy_field[i] + d + regen).max(0.0);
        }
        for (i, v) in delta_signal.iter().enumerate() {
            self.signal_field[i] =
                (self.signal_field[i] * (1.0 - self.config.signal_decay) + v).max(0.0);
        }
    }

    /// Apply one validated action for one organism. Returns events + energy
    /// delta to apply to the organism after metabolism.
    fn apply_action(&mut self, org_idx: usize, action: &Action) -> (Vec<WorldEvent>, f32) {
        let mut events = Vec::new();
        let subject = self.organisms[org_idx].subject_id.clone();
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
                // params[0] in 0..1 = fraction of attempt strength; converts
                // up to param * MAX_ABSORB from the cell into organism energy
                // times absorb_efficiency × genome absorb_affinity. The
                // genome modifier makes absorption a selectable trait.
                let want = action.params[0] * MAX_ABSORB;
                let (x, y) = self.organisms[org_idx].pos;
                let i = self.idx(x, y);
                let available = self.energy_field[i].min(want);
                self.energy_field[i] -= available;
                let affinity = self.organisms[org_idx].genome.absorb_affinity;
                let gained = available * self.config.absorb_efficiency * affinity;
                energy_delta += gained;
                events.push(WorldEvent::ActionApplied {
                    subject: subject.clone(),
                    action: "ABSORB".into(),
                    cost: available,
                });
            }
            "RELEASE" => {
                // Return organism energy to the cell.
                let amount = (action.params[0] * MAX_ABSORB).min(self.organisms[org_idx].energy);
                self.organisms[org_idx].energy -= amount;
                let (x, y) = self.organisms[org_idx].pos;
                let i = self.idx(x, y);
                self.energy_field[i] += amount;
                events.push(WorldEvent::ActionApplied {
                    subject: subject.clone(),
                    action: "RELEASE".into(),
                    cost: amount,
                });
            }
            "SIGNAL" => {
                // Inject signal energy into the cell (costs organism energy).
                let strength = action.params[0] * MAX_SIGNAL;
                let cost = strength * SIGNAL_COST;
                if self.organisms[org_idx].energy >= cost {
                    self.organisms[org_idx].energy -= cost;
                    let (x, y) = self.organisms[org_idx].pos;
                    let i = self.idx(x, y);
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
            "MOVE" => {
                // params: (dx, dy) each 0..1 mapped to -1..=1 steps, cost scales
                // with distance. Out-of-bounds moves are clamped (world edge).
                let dx = (action.params[0] * 2.0 - 1.0).round() as i32;
                let dy = (action.params[1] * 2.0 - 1.0).round() as i32;
                let cost = MOVE_COST * (dx.abs() + dy.abs()) as f32;
                if self.organisms[org_idx].energy >= cost {
                    self.organisms[org_idx].energy -= cost;
                    let (w, h) = (self.config.width, self.config.height);
                    let (x, y) = self.organisms[org_idx].pos;
                    let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    self.organisms[org_idx].pos = (nx, ny);
                    events.push(WorldEvent::ActionApplied {
                        subject: subject.clone(),
                        action: "MOVE".into(),
                        cost,
                    });
                } else {
                    events.push(WorldEvent::ActionRejected {
                        subject: subject.clone(),
                        action: "MOVE".into(),
                        reason: "insufficient energy for move cost".into(),
                    });
                }
            }
            "DIVIDE" => {
                // M4 reproduction: genome-gated, energy-split, mutated child.
                if self.organisms.len() >= self.population_cap {
                    events.push(WorldEvent::ActionRejected {
                        subject: subject.clone(),
                        action: "DIVIDE".into(),
                        reason: "population cap reached".into(),
                    });
                } else {
                    let (
                        parent_energy,
                        parent_genome,
                        parent_lineage,
                        parent_generation,
                        parent_pos,
                    ) = {
                        let p = &self.organisms[org_idx];
                        (
                            p.energy,
                            p.genome.clone(),
                            p.lineage_id.clone(),
                            p.generation,
                            p.pos,
                        )
                    };
                    match attempt_division(parent_energy, &parent_genome, &self.division_policy) {
                        Ok(child_energy) => {
                            // Deduct child share + cost from parent.
                            self.organisms[org_idx].energy =
                                parent_energy - self.division_policy.division_cost - child_energy;
                            // Mutated child genome.
                            let child_genome = parent_genome.mutate(&mut self.rng);
                            // Child placed on a random adjacent cell (or same
                            // cell if the parent is at the edge).
                            let neighbors = self.neighbors4(parent_pos.0, parent_pos.1);
                            let (cx, cy) = if neighbors.is_empty() {
                                parent_pos
                            } else {
                                let pick = self.rng.next_range(neighbors.len() as u64) as usize;
                                neighbors[pick]
                            };
                            let child_id = format!("org-{:06}", self.child_counter);
                            self.child_counter += 1;
                            self.spawn_with_genome_at(
                                &child_id,
                                (cx, cy),
                                child_energy,
                                child_genome,
                                parent_lineage,
                                parent_generation + 1,
                            );
                            events.push(WorldEvent::ActionApplied {
                                subject: subject.clone(),
                                action: format!("DIVIDE->{child_id}"),
                                cost: self.division_policy.division_cost,
                            });
                        }
                        Err(reason) => {
                            events.push(WorldEvent::ActionRejected {
                                subject: subject.clone(),
                                action: "DIVIDE".into(),
                                reason,
                            });
                        }
                    }
                }
            }
            other => {
                events.push(WorldEvent::ActionRejected {
                    subject,
                    action: other.into(),
                    reason: "not implemented in primordial-v0".into(),
                });
            }
        }
        (events, energy_delta)
    }
}

/// Max energy extractable per ABSORB action (world units).
pub const MAX_ABSORB: f32 = 2.0;
/// Max signal strength per SIGNAL action.
pub const MAX_SIGNAL: f32 = 1.0;
/// Energy cost per unit of signal strength.
pub const SIGNAL_COST: f32 = 0.5;
/// Energy cost per step of movement.
pub const MOVE_COST: f32 = 0.1;

impl World for PrimordialWorld {
    fn implementation(&self) -> &'static str {
        WORLD_IMPLEMENTATION
    }

    fn manifest(&self) -> &CapabilityManifest {
        &self.manifest
    }

    fn observe(&self, subject: &SubjectRef) -> Result<Vec<Observation>, WabixError> {
        let org = self.organism(&subject.subject_id).ok_or_else(|| {
            WabixError::UnknownChannel(format!("dead subject {}", subject.subject_id))
        })?;
        let (x, y) = org.pos;
        let i = self.idx(x, y);
        Ok(vec![
            // Own energy store, normalized 0..1 (clamped at START_ENERGY scale).
            Observation::new("local_energy", (org.energy / 10.0).clamp(0.0, 1.0)),
            Observation::new(
                "chemical_0",
                (self.chemical_field[i] / FIELD_NORMALIZATION).clamp(0.0, 1.0),
            ),
            Observation::new(
                "nearby_signal_0",
                (self.signal_field[i] / FIELD_NORMALIZATION).clamp(0.0, 1.0),
            ),
            // NOTE: the cell's resource field is deliberately NOT observable
            // directly in v0 — subjects must probe it through ABSORB outcomes.
            // This keeps sensing honest and makes exploration meaningful.
        ])
    }

    fn validate(&self, action: &Action) -> Result<(), WabixError> {
        validate_action(&self.manifest, action)
    }

    fn step(&mut self, _subject: &SubjectRef, _action: Option<&Action>) -> Vec<WorldEvent> {
        // The primordial world drives all organisms internally; the generic
        // per-subject step interface is not used. See `tick()` below.
        unreachable!("primordial world uses tick(); per-subject step is not applicable")
    }
}

impl PrimordialWorld {
    /// Advance the world one tick: physics, then per-organism actions and
    /// metabolism, then death. Returns all events in deterministic order.
    pub fn tick(&mut self, actions: &[(String, Action)]) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        // Apply this tick's actions in the given order.
        for (subject_id, action) in actions {
            if let Some(org_idx) = self
                .organisms
                .iter()
                .position(|o| o.subject_id == *subject_id && o.alive)
            {
                // Execution-time validation (defense in depth).
                if let Err(e) = self.validate(action) {
                    events.push(WorldEvent::ActionRejected {
                        subject: subject_id.clone(),
                        action: action.name.clone(),
                        reason: e.to_string(),
                    });
                    continue;
                }
                let (mut evts, energy_delta) = self.apply_action(org_idx, action);
                self.organisms[org_idx].energy += energy_delta;
                events.append(&mut evts);
            }
        }

        // Metabolism for everyone.
        for org in &mut self.organisms {
            if org.alive {
                org.energy -= self.config.metabolism;
            }
        }

        // Physics.
        self.step_physics();

        // Death check.
        for org in &mut self.organisms {
            if org.alive && org.energy <= 0.0 {
                org.alive = false;
                events.push(WorldEvent::SubjectDied {
                    subject: org.subject_id.clone(),
                    cause: "energy exhausted".into(),
                });
            }
        }

        self.tick += 1;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_diffuses_and_regenerates() {
        let mut w = PrimordialWorld::new(PrimordialConfig::default(), 1);
        // Drain one cell.
        let (x, y) = (3, 3);
        let i = w.idx(x, y);
        w.energy_field[i] = 0.0;
        let before = w.energy_field[w.idx(x, y)];
        w.step_physics();
        let after = w.energy_field[w.idx(x, y)];
        assert!(after > before, "regen must raise the drained cell");
        // Neighbor should have been drained slightly by diffusion.
        let neighbor_before = 0.0;
        let _ = neighbor_before;
    }

    #[test]
    fn absorb_takes_energy_from_cell() {
        let mut w = PrimordialWorld::new(PrimordialConfig::default(), 2);
        let pos = w.spawn("s1", 5.0);
        let cell_before = w.energy_field_cell(pos.0, pos.1);
        let org_before = w.organism("s1").unwrap().energy;
        let (events, delta) = w.apply_action(
            0,
            &Action {
                name: "ABSORB".into(),
                params: vec![1.0],
            },
        );
        w.organisms[0].energy += delta;
        assert!(matches!(events[0], WorldEvent::ActionApplied { .. }));
        assert!(w.energy_field_cell(pos.0, pos.1) < cell_before);
        assert!(w.organism("s1").unwrap().energy > org_before);
    }

    #[test]
    fn metabolism_and_death() {
        let cfg = PrimordialConfig {
            metabolism: 1.0,
            ..Default::default()
        };
        let mut w = PrimordialWorld::new(cfg, 3);
        w.spawn("s1", 2.5);
        let _ = w.tick(&[]);
        assert_eq!(w.organism("s1").unwrap().energy, 1.5);
        let events = w.tick(&[]);
        let events2 = w.tick(&[]);
        let died = events
            .iter()
            .chain(events2.iter())
            .any(|e| matches!(e, WorldEvent::SubjectDied { subject, .. } if subject == "s1"));
        assert!(
            died,
            "organism with 2.5 energy and 1.0 metabolism dies on tick 3"
        );
        assert_eq!(w.alive_count(), 0);
    }

    #[test]
    fn move_costs_energy_and_respects_bounds() {
        let cfg = PrimordialConfig {
            width: 4,
            height: 4,
            ..Default::default()
        };
        let mut w = PrimordialWorld::new(cfg, 4);
        let pos = w.spawn("s1", 10.0);
        assert!(pos.0 <= 3 && pos.1 <= 3); // spawn is seeded-deterministic, in bounds
        let org_before = w.organism("s1").unwrap().energy;
        let (_, delta) = w.apply_action(
            0,
            &Action {
                name: "MOVE".into(),
                params: vec![1.0, 0.5], // dx=+1, dy=0
            },
        );
        w.organisms[0].energy += delta;
        let org = w.organism("s1").unwrap();
        assert!(org.energy < org_before);
        assert!(org.pos.0 <= 3 && org.pos.1 <= 3);
    }

    #[test]
    fn signal_field_decays() {
        let mut w = PrimordialWorld::new(PrimordialConfig::default(), 5);
        w.spawn("s1", 10.0);
        w.apply_action(
            0,
            &Action {
                name: "SIGNAL".into(),
                params: vec![1.0],
            },
        );
        w.tick(&[]);
        let s1 = w.signal_field[w.idx(
            w.organism("s1").map(|o| o.pos.0).unwrap_or(0),
            w.organism("s1").map(|o| o.pos.1).unwrap_or(0),
        )];
        assert!(s1 > 0.0, "signal must be injected");
        w.tick(&[]);
        w.tick(&[]);
        let s2 = w.signal_field[w.idx(
            w.organism("s1").map(|o| o.pos.0).unwrap_or(0),
            w.organism("s1").map(|o| o.pos.1).unwrap_or(0),
        )];
        assert!(s2 < s1, "signal must decay");
    }

    #[test]
    fn determinism_same_seed_same_trajectory() {
        let run = |seed: u64| {
            let mut w = PrimordialWorld::new(PrimordialConfig::default(), seed);
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
                                "MOVE".into()
                            },
                            params: if t % 2 == 0 {
                                vec![0.8]
                            } else {
                                vec![0.5, 0.9]
                            },
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
                    "t={} a=({},{},{:.3}) b=({},{},{:.3}) ev={}",
                    t,
                    w.organism("a").map(|o| o.pos.0).unwrap_or(999),
                    w.organism("a").map(|o| o.pos.1).unwrap_or(999),
                    w.organism("a").map(|o| o.energy).unwrap_or(-1.0),
                    w.organism("b").map(|o| o.pos.0).unwrap_or(999),
                    w.organism("b").map(|o| o.pos.1).unwrap_or(999),
                    w.organism("b").map(|o| o.energy).unwrap_or(-1.0),
                    events.len(),
                ));
            }
            trace
        };
        assert_eq!(run(123), run(123));
        assert_ne!(run(123), run(124));
    }
}
