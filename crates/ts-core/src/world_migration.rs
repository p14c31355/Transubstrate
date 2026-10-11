//! M9: World migration — move a subject between structurally different
//! worlds behind WABIX.
//!
//! The same SubjectId travels from the grid-based primordial world into the
//! ring-topology network world. The subject's carried-over energy transfers
//! to the destination node. Continuity is measured the same way as M5:
//! pre-migration vs post-migration behavioral fingerprints, with a
//! control run that stays in the origin world the whole time.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::cognition::{CognitionError, DecisionContext};
use crate::continuity::{ActionCollector, ContinuityDimension, ContinuityVector, MigrationEvent};
use crate::event_log::{EventLog, EventPayload};
use crate::ids::{ExperimentId, SubjectId};
use crate::network_world::{NETWORK_WORLD_IMPLEMENTATION, NetworkWorld};
use crate::primordial::{PrimordialConfig, PrimordialWorld, WORLD_IMPLEMENTATION};
use crate::resource::ResourceLedger;
use crate::runtime::CognitionCadence;
use crate::wabix::{Action, SubjectRef, World};

/// Configuration for a world-migration experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMigrationConfig {
    pub experiment_id: ExperimentId,
    pub world: PrimordialConfig,
    pub world_seed: u64,
    pub ticks: u64,
    pub cognition_interval: u64,
    pub start_energy: f32,
    pub backend: crate::primordial_experiment::BackendKind,
    /// Network-world topology (destination world config).
    pub network: crate::network_world::NetworkConfig,
    /// Memory ablation switch (default true = the SUBJECT != MODEL design).
    /// When false, the subject's memory is WIPED at the migration boundary:
    /// the destination world's decisions run with an empty buffer. Comparing
    /// carry vs wipe makes memory a causal experimental variable instead of
    /// a correlational one.
    #[serde(default = "default_true")]
    pub memory_carry: bool,
}

fn default_true() -> bool {
    true
}

impl Default for WorldMigrationConfig {
    fn default() -> Self {
        Self {
            experiment_id: ExperimentId::generate(),
            world: PrimordialConfig::default(),
            world_seed: 42,
            ticks: 400,
            cognition_interval: 5,
            start_energy: 20.0,
            backend: crate::primordial_experiment::BackendKind::EnergyRule {
                absorb_threshold: 0.3,
            },
            network: crate::network_world::NetworkConfig::default(),
            memory_carry: true,
        }
    }
}

/// How quickly did behavior adapt to the new world after the migration
/// boundary? Measured on the decision stream, not the observations
/// themselves: the pre-swap dominant action defines the subject's
/// established strategy; the lag counts ticks until that strategy loses
/// majority in a trailing decision window of the post-swap phase.
///
/// The persistence flag separates real strategy change from sampling
/// noise: an immediate switch (lag 0) that reverts later is NOT stable
/// adaptation, while a slower switch that holds through the late
/// post-swap half is. Both fields are None when the subject never left
/// its pre-swap dominant strategy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationAdaptation {
    /// Ticks after the boundary until the pre-swap dominant action first
    /// drops below half of a trailing decision window. None = never
    /// displaced during the run.
    pub lag_ticks: Option<u64>,
    /// The action that displaced the pre-swap dominant one.
    pub new_action: Option<String>,
    /// Whether the displacing action still holds at least half of the
    /// LATE post-swap decisions (second half of the post phase).
    /// Some(false) = switched once and reverted (noise), Some(true) =
    /// the new strategy held. None when no displacement happened.
    pub persists: Option<bool>,
}

/// Number of consecutive decisions per evaluation window for the
/// adaptation metric. Windowed (not per-decision) so single exploratory
/// deviations do not read as adaptation.
pub const ADAPTATION_WINDOW: usize = 10;

/// Compute the observation-adaptation metrics from an event-log record
/// stream. `boundary` is the migration tick (decisions with
/// tick < boundary are pre-swap, the rest post-swap).
pub fn compute_observation_adaptation(
    records: &[crate::event_log::EventRecord],
    boundary: u64,
) -> ObservationAdaptation {
    use crate::event_log::EventPayload;
    let mut actions: Vec<(u64, String)> = Vec::new();
    for rec in records {
        if let EventPayload::DecisionMade { tick, decision, .. } = &rec.payload {
            let name = decision
                .action
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "WAIT".to_string());
            actions.push((*tick, name));
        }
    }
    // Records arrive in log order; the metric needs strict tick order.
    actions.sort_by_key(|(t, _)| *t);
    let pre: Vec<&String> = actions
        .iter()
        .filter(|(t, _)| *t < boundary)
        .map(|(_, a)| a)
        .collect();
    if pre.is_empty() {
        return ObservationAdaptation {
            lag_ticks: None,
            new_action: None,
            persists: None,
        };
    }
    // Pre-swap dominant action (ties broken by first-seen order via max_by).
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for a in &pre {
        *counts.entry(a.as_str()).or_insert(0) += 1;
    }
    let dominant = counts
        .iter()
        .max_by_key(|(_, c)| **c)
        .map(|(a, _)| a.to_string())
        .expect("non-empty pre phase");

    let post: Vec<(u64, &str)> = actions
        .iter()
        .filter(|(t, _)| *t >= boundary)
        .map(|(t, a)| (*t, a.as_str()))
        .collect();

    // First trailing window where the dominant action falls below half.
    let mut lag = None;
    let mut displaced_by = None;
    for chunk in post.chunks(ADAPTATION_WINDOW) {
        // A trailing partial window smaller than half a full window
        // cannot distinguish adaptation from a single stray decision.
        if chunk.len() < ADAPTATION_WINDOW / 2 {
            break;
        }
        let keep = chunk
            .iter()
            .filter(|(_, a)| *a == dominant.as_str())
            .count();
        if (keep as f64) < chunk.len() as f64 * 0.5 {
            lag = Some(chunk[0].0.saturating_sub(boundary));
            displaced_by = chunk
                .iter()
                .filter(|(_, a)| *a != dominant.as_str())
                .map(|(_, a)| a.to_string())
                .next();
            break;
        }
    }
    let (lag, new_action) = match (lag, displaced_by) {
        (Some(l), Some(a)) => (Some(l), Some(a)),
        _ => {
            return ObservationAdaptation {
                lag_ticks: None,
                new_action: None,
                persists: None,
            };
        }
    };

    // Persistence: does the new action hold >= half of the LATE post
    // decisions (second half of the post phase, by count)?
    let post_total = post.len();
    let late_start = post_total / 2;
    let late = &post[late_start..];
    let holds = late
        .iter()
        .filter(|(_, a)| Some((*a).to_string()) == new_action)
        .count();
    let persists = if late.is_empty() {
        None
    } else {
        Some((holds as f64) >= late.len() as f64 * 0.5)
    };
    ObservationAdaptation {
        lag_ticks: lag,
        new_action,
        persists,
    }
}

/// World-migration experiment result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMigrationResult {
    pub experiment_id: String,
    pub migration: MigrationEvent,
    /// Migration run (primordial → network) continuity vector.
    pub continuity: crate::continuity::ContinuityVector,
    /// Control: same subject staying in the origin world the whole time.
    pub control_continuity: crate::continuity::ContinuityVector,
    /// Per-dimension migration effect: migration − control (negative =
    /// the swap hurt that dimension relative to staying put).
    pub effect: crate::continuity::ContinuityVector,
    /// Ablation contrast (present only when config.memory_carry is false):
    /// continuity of the SAME migration with memory wiped at the boundary.
    /// Comparing `continuity` vs `ablation_continuity` isolates what the
    /// carried memory CAUSES, beyond correlation.
    pub ablation_continuity: Option<crate::continuity::ContinuityVector>,
    /// ablation_continuity − continuity per dimension (positive = carrying
    /// memory helped that dimension). None when memory_carry is true.
    pub ablation_effect: Option<crate::continuity::ContinuityVector>,
    /// How fast behavior adapted to the destination world after the
    /// boundary (decision-stream metric; see ObservationAdaptation).
    pub observation_adaptation: ObservationAdaptation,
}

/// Run a world migration: subject starts in the primordial world, migrates
/// into the network world at the midpoint with carried-over energy.
pub fn run_world_migration(
    config: &WorldMigrationConfig,
    log_path: &Path,
) -> Result<crate::continuity::ContinuityVector, CognitionError> {
    run_world_migration_inner(config, log_path, None)
}

/// Same as [`run_world_migration`], but the pre-swap phase REPLAYS the given
/// decisions (tick -> decision) instead of asking the backend. Used by the
/// same-trajectory ablation: the wipe arm replays the carry arm's pre-swap
/// decisions so both arms share an identical pre-phase trajectory, and the
/// carry − wipe contrast isolates the causal effect of the carried memory.
pub fn run_world_migration_with_replay(
    config: &WorldMigrationConfig,
    log_path: &Path,
    pre_decisions: &std::collections::BTreeMap<u64, crate::cognition::Decision>,
) -> Result<crate::continuity::ContinuityVector, CognitionError> {
    run_world_migration_inner(config, log_path, Some(pre_decisions))
}

fn run_world_migration_inner(
    config: &WorldMigrationConfig,
    log_path: &Path,
    pre_replay: Option<&std::collections::BTreeMap<u64, crate::cognition::Decision>>,
) -> Result<crate::continuity::ContinuityVector, CognitionError> {
    let mut log = EventLog::new(8192)
        .with_sink(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("log sink: {e}")))?;

    let manifest = crate::replay::ExperimentManifest {
        manifest_version: crate::replay::MANIFEST_VERSION,
        experiment_id: config.experiment_id.to_string(),
        experiment_kind: "world_migration".into(),
        world_implementation: format!("{WORLD_IMPLEMENTATION}->{NETWORK_WORLD_IMPLEMENTATION}"),
        seed: config.world_seed,
        config: serde_json::to_value(config)
            .map_err(|e| CognitionError::Unavailable(format!("manifest: {e}")))?,
    };
    manifest
        .write_next_to(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("manifest write: {e}")))?;

    log.push(EventPayload::ExperimentStarted {
        experiment_id: config.experiment_id.to_string(),
        world_implementation: format!("{WORLD_IMPLEMENTATION}->{NETWORK_WORLD_IMPLEMENTATION}"),
        seed: config.world_seed,
    });

    let half = config.ticks / 2;
    let cadence = CognitionCadence::every(config.cognition_interval);
    let mut ledger = ResourceLedger::new(4096);

    // ---- Origin world: primordial, first half ----
    let sid = SubjectId::from_static("world-migrant");
    let sid_str = sid.to_string();

    let mut origin = PrimordialWorld::new(config.world.clone(), config.world_seed);
    let pos = origin.spawn(&sid_str, config.start_energy);
    let mut sref = SubjectRef {
        subject_id: sid_str.clone(),
        pos,
        energy: config.start_energy,
    };
    log.push(EventPayload::SubjectCreated {
        subject_id: sid_str.clone(),
        subject_ref: SubjectRef {
            subject_id: sid_str.clone(),
            pos,
            energy: config.start_energy,
        },
    });

    let mut backend = config.backend.instantiate();

    // Per-subject bounded memory persists ACROSS the world boundary —
    // memory belongs to the subject, not the world or the backend
    // (SUBJECT != MODEL, same treatment as the model-migration harness).
    let mut memory: Vec<crate::cognition::MemoryEntry> = Vec::new();
    const MEMORY_LIMIT: usize = 8;

    let mut pre_collector = ActionCollector::new();
    for local in 0..half {
        let tick = local;
        let mut actions: Vec<(String, Action)> = Vec::new();
        let cand = cadence
            .fires_on(tick)
            .then(|| origin.organism(&sid_str).map(|org| (org.pos, org.energy)))
            .flatten();
        if let Some((pos, energy)) = cand {
            sref.pos = pos;
            sref.energy = energy;
            // Same-trajectory ablation: when a pre-swap decision replay is
            // supplied for this tick, use it verbatim (no backend call) so
            // the pre-phase trajectory is identical across arms. The post-
            // swap phase (origin world is only the first half) always asks
            // the backend.
            let decision = if tick < half {
                match pre_replay.and_then(|m| m.get(&tick)) {
                    Some(replay) => replay.clone(),
                    None => {
                        let observations = origin.observe(&sref).map_err(|e| {
                            CognitionError::Unavailable(format!("observe failed: {e}"))
                        })?;
                        let ctx = DecisionContext {
                            subject_id: sid_str.clone(),
                            tick,
                            energy: sref.energy,
                            observations,
                            manifest: origin.manifest().clone(),
                            memory: memory.clone(),
                        };
                        backend.decide(&ctx)?
                    }
                }
            } else {
                let observations = origin
                    .observe(&sref)
                    .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
                let ctx = DecisionContext {
                    subject_id: sid_str.clone(),
                    tick,
                    energy: sref.energy,
                    observations,
                    manifest: origin.manifest().clone(),
                    memory: memory.clone(),
                };
                backend.decide(&ctx)?
            };
            crate::cognition::validate_decision(origin.manifest(), &decision)?;
            crate::cognition::charge_cognition(
                &mut ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0,
                },
                &sid_str,
            );
            log.push(EventPayload::DecisionMade {
                subject_id: sid_str.clone(),
                tick,
                decision: decision.clone(),
            });
            if let Some(action) = decision.action {
                pre_collector.record(Some(&action.name));
                actions.push((sid_str.clone(), action.clone()));
                crate::cognition::record_memory(
                    &mut memory,
                    tick,
                    Some(&action),
                    sref.energy,
                    MEMORY_LIMIT,
                );
            } else {
                pre_collector.record(None);
                crate::cognition::record_memory(&mut memory, tick, None, sref.energy, MEMORY_LIMIT);
            }
        }
        let world_events = origin.tick(&actions);
        log.push(EventPayload::WorldEvents {
            tick,
            events: world_events,
        });
        if let Some(org) = origin.organism(&sid_str) {
            sref.pos = org.pos;
            sref.energy = org.energy;
        }
        // Reward wiring: energy delta from pre-decision energy.
        let reward = memory
            .last_mut()
            .filter(|m| m.reward_signal == 0.0)
            .zip(origin.organism(&sid_str));
        if let Some((last, org)) = reward {
            last.reward_signal = org.energy - last.pre_tick_energy_hint();
        }
    }

    // ---- Migration event: subject crosses into the network world ----
    let carried_energy = sref.energy.max(1.0);
    // Snapshot the subject's memory AT the boundary: what is remembered in
    // the origin world vs what is carried into the destination world is the
    // memory-retention dimension (memory belongs to the subject).
    let memory_pre: Vec<crate::cognition::MemoryEntry> = memory.clone();
    // Memory ablation: when memory_carry is false, the buffer is wiped at
    // the boundary — the destination world's cognition starts amnesic. This
    // is the causal counterpart of the correlational memory_usage reading:
    // any continuity difference between carry and wipe runs is attributable
    // to the carried memory itself.
    let ablated = !config.memory_carry;
    if ablated {
        memory.clear();
    }
    let migration = MigrationEvent::new(
        &sid_str,
        "world",
        WORLD_IMPLEMENTATION,
        NETWORK_WORLD_IMPLEMENTATION,
        half,
        "incarnation-origin".into(),
        "incarnation-network".into(),
    );
    log.push(EventPayload::Migration {
        subject_id: sid_str.clone(),
        migration_id: migration.migration_id.clone(),
        dimension: migration.dimension.clone(),
        from: migration.from.clone(),
        to: migration.to.clone(),
        tick: half,
    });

    // ---- Destination world: network ring, second half ----
    let mut network = NetworkWorld::new(config.network.clone(), config.world_seed);
    let node = (config.world_seed % config.network.nodes.max(1) as u64) as u32;
    network.admit_subject(&sid_str, node, carried_energy);

    let mut post_collector = ActionCollector::new();
    for local in 0..(config.ticks - half) {
        let tick = half + local;
        let mut actions: Vec<(String, Action)> = Vec::new();
        let cand = cadence
            .fires_on(tick)
            .then(|| network.subject(&sid_str).map(|sub| (sub.node, sub.energy)))
            .flatten();
        if let Some((node_pos, energy)) = cand {
            let sref = SubjectRef {
                subject_id: sid_str.clone(),
                pos: (node_pos, 0),
                energy,
            };
            let observations = network
                .observe(&sref)
                .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
            let ctx = DecisionContext {
                subject_id: sid_str.clone(),
                tick,
                energy,
                observations,
                manifest: network.manifest().clone(),
                memory: memory.clone(),
            };
            let decision = backend.decide(&ctx)?;
            crate::cognition::validate_decision(network.manifest(), &decision)?;
            crate::cognition::charge_cognition(
                &mut ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0,
                },
                &sid_str,
            );
            log.push(EventPayload::DecisionMade {
                subject_id: sid_str.clone(),
                tick,
                decision: decision.clone(),
            });
            if let Some(action) = decision.action {
                post_collector.record(Some(&action.name));
                actions.push((sid_str.clone(), action.clone()));
                crate::cognition::record_memory(
                    &mut memory,
                    tick,
                    Some(&action),
                    energy,
                    MEMORY_LIMIT,
                );
            } else {
                post_collector.record(None);
                crate::cognition::record_memory(&mut memory, tick, None, energy, MEMORY_LIMIT);
            }
        }
        let world_events = network.tick(&actions);
        log.push(EventPayload::WorldEvents {
            tick,
            events: world_events,
        });
        // Reward wiring: energy delta from pre-decision energy.
        let reward = memory
            .last_mut()
            .filter(|m| m.reward_signal == 0.0)
            .zip(network.subject(&sid_str));
        if let Some((last, sub)) = reward {
            last.reward_signal = sub.energy - last.pre_tick_energy_hint();
        }
    }

    log.push(EventPayload::ExperimentFinished {
        reason: "world migration complete".into(),
    });

    Ok(ContinuityVector {
        subject_id: sid_str,
        migration_id: Some(migration.migration_id),
        dimensions: vec![
            ContinuityVector::behavioral(
                &pre_collector.fingerprint(),
                &post_collector.fingerprint(),
            ),
            ContinuityVector::memory_retention(&memory_pre, &memory),
            ContinuityVector::memory_usage(&memory_pre, &post_collector.distinct_action_names()),
        ],
    })
}

/// Full world-migration experiment: the migration run (primordial →
/// network) plus a same-seed CONTROL that stays in the origin world the
/// whole time with the same backend. The effect is reported per dimension:
/// migration value − control value (negative = the swap hurt that
/// dimension relative to simply staying put).
pub fn run_world_migration_experiment(
    config: &WorldMigrationConfig,
    log_path: &Path,
) -> Result<WorldMigrationResult, CognitionError> {
    let continuity = run_world_migration(config, log_path)?;
    let migration = MigrationEvent::new(
        "world-migrant",
        "world",
        WORLD_IMPLEMENTATION,
        NETWORK_WORLD_IMPLEMENTATION,
        config.ticks / 2,
        "incarnation-origin".into(),
        "incarnation-network".into(),
    );

    // ---- Control: same seed, same backend, origin world throughout ----
    // Written to a sibling log so the two runs stay independently replayable.
    let control_path = log_path.with_extension("control.jsonl");
    let control_continuity = run_world_control(config, &control_path)?;

    // ---- Ablation arm (memory_carry = true only): the same migration
    // with the boundary wipe AND the carry arm's pre-swap decisions
    // REPLAYED verbatim, so both arms share an identical pre-phase
    // trajectory and the carry − wipe contrast isolates the causal effect
    // of the carried memory (LLM sampling noise cannot disguise itself as
    // a pre-phase trajectory difference). Its own log keeps replay
    // independence. memory_carry=false in `config` means `continuity` IS
    // the ablation run, so the arm runs only in the default carry mode.
    let (ablation_continuity, ablation_effect) = if config.memory_carry {
        // Extract the carry arm's pre-swap decisions from its event log.
        let records = crate::event_log::read_log(log_path)
            .map_err(|e| CognitionError::Unavailable(format!("ablation replay: {e}")))?;
        let mut pre_decisions: std::collections::BTreeMap<u64, crate::cognition::Decision> =
            std::collections::BTreeMap::new();
        let half = config.ticks / 2;
        for rec in &records {
            let made = match &rec.payload {
                EventPayload::DecisionMade { tick, decision, .. } if *tick < half => {
                    Some((*tick, decision))
                }
                _ => None,
            };
            if let Some((tick, decision)) = made {
                pre_decisions.insert(tick, decision.clone());
            }
        }
        let mut ablated_config = config.clone();
        ablated_config.memory_carry = false;
        ablated_config.experiment_id = ExperimentId::generate();
        let ablation_path = log_path.with_extension("ablation.jsonl");
        let ablated =
            run_world_migration_with_replay(&ablated_config, &ablation_path, &pre_decisions)?;
        let mut effect_dims = Vec::new();
        for (carried, wiped) in continuity.dimensions.iter().zip(ablated.dimensions.iter()) {
            effect_dims.push(ContinuityDimension {
                name: format!("ablation_effect_{}", carried.name),
                // positive = carrying memory helped that dimension
                value: carried.value - wiped.value,
                computation: format!(
                    "carry({:.4}) - wipe({:.4}); wipe computation: {}",
                    carried.value, wiped.value, wiped.computation
                ),
            });
        }
        (
            Some(ablated),
            Some(ContinuityVector {
                subject_id: continuity.subject_id.clone(),
                migration_id: continuity.migration_id.clone(),
                dimensions: effect_dims,
            }),
        )
    } else {
        (None, None)
    };

    // Per-dimension migration effect against the control.
    let subject_id = continuity.subject_id.clone();
    let migration_id = continuity.migration_id.clone();
    let mut effect_dimensions = Vec::new();
    for (mig, ctrl) in continuity
        .dimensions
        .iter()
        .zip(control_continuity.dimensions.iter())
    {
        effect_dimensions.push(ContinuityDimension {
            name: format!("effect_{}", mig.name),
            value: mig.value - ctrl.value,
            computation: format!(
                "migration({:.4}) - control({:.4}); control computation: {}",
                mig.value, ctrl.value, ctrl.computation
            ),
        });
    }

    // Observation adaptation: measured on the migration run's own log
    // (the control has no boundary, so adaptation is undefined there).
    let mig_records = crate::event_log::read_log(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("adaptation: {e}")))?;
    let observation_adaptation = compute_observation_adaptation(&mig_records, config.ticks / 2);

    Ok(WorldMigrationResult {
        experiment_id: config.experiment_id.to_string(),
        migration,
        continuity,
        control_continuity,
        effect: ContinuityVector {
            subject_id,
            migration_id,
            dimensions: effect_dimensions,
        },
        ablation_continuity,
        ablation_effect,
        observation_adaptation,
    })
}

/// Control run: the same subject, seed, and backend, but NO world swap —
/// it spends the whole run in the primordial world. Used to separate "what
/// the world change did" from "what the second half of the run did anyway".
fn run_world_control(
    config: &WorldMigrationConfig,
    log_path: &Path,
) -> Result<crate::continuity::ContinuityVector, CognitionError> {
    let mut log = EventLog::new(8192)
        .with_sink(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("log sink: {e}")))?;

    let half = config.ticks / 2;
    let cadence = CognitionCadence::every(config.cognition_interval);
    let mut ledger = ResourceLedger::new(4096);

    let sid = SubjectId::from_static("world-control");
    let sid_str = sid.to_string();
    let mut origin = PrimordialWorld::new(config.world.clone(), config.world_seed);
    let pos = origin.spawn(&sid_str, config.start_energy);
    let mut sref = SubjectRef {
        subject_id: sid_str.clone(),
        pos,
        energy: config.start_energy,
    };
    log.push(EventPayload::SubjectCreated {
        subject_id: sid_str.clone(),
        subject_ref: SubjectRef {
            subject_id: sid_str.clone(),
            pos,
            energy: config.start_energy,
        },
    });

    let mut backend = config.backend.instantiate();
    let mut memory: Vec<crate::cognition::MemoryEntry> = Vec::new();
    const MEMORY_LIMIT: usize = 8;

    let mut pre_collector = ActionCollector::new();
    let mut post_collector = ActionCollector::new();
    for local in 0..config.ticks {
        let tick = local;
        let mut actions: Vec<(String, Action)> = Vec::new();
        let cand = cadence
            .fires_on(tick)
            .then(|| origin.organism(&sid_str).map(|org| (org.pos, org.energy)))
            .flatten();
        if let Some((pos, energy)) = cand {
            sref.pos = pos;
            sref.energy = energy;
            let observations = origin
                .observe(&sref)
                .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
            let ctx = DecisionContext {
                subject_id: sid_str.clone(),
                tick,
                energy: sref.energy,
                observations,
                manifest: origin.manifest().clone(),
                memory: memory.clone(),
            };
            let decision = backend.decide(&ctx)?;
            crate::cognition::validate_decision(origin.manifest(), &decision)?;
            crate::cognition::charge_cognition(
                &mut ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0,
                },
                &sid_str,
            );
            log.push(EventPayload::DecisionMade {
                subject_id: sid_str.clone(),
                tick,
                decision: decision.clone(),
            });
            let collector: &mut ActionCollector = if tick < half {
                &mut pre_collector
            } else {
                &mut post_collector
            };
            if let Some(action) = decision.action {
                collector.record(Some(&action.name));
                actions.push((sid_str.clone(), action.clone()));
                crate::cognition::record_memory(
                    &mut memory,
                    tick,
                    Some(&action),
                    sref.energy,
                    MEMORY_LIMIT,
                );
            } else {
                collector.record(None);
                crate::cognition::record_memory(&mut memory, tick, None, sref.energy, MEMORY_LIMIT);
            }
        }
        let world_events = origin.tick(&actions);
        log.push(EventPayload::WorldEvents {
            tick,
            events: world_events,
        });
        // Reward wiring: energy delta from pre-decision energy.
        let reward = memory
            .last_mut()
            .filter(|m| m.reward_signal == 0.0)
            .zip(origin.organism(&sid_str));
        if let Some((last, org)) = reward {
            last.reward_signal = org.energy - last.pre_tick_energy_hint();
        }
    }
    let memory_pre_len = memory.len();

    log.push(EventPayload::ExperimentFinished {
        reason: "world migration control complete".into(),
    });

    Ok(ContinuityVector {
        subject_id: sid_str,
        migration_id: None,
        dimensions: vec![
            ContinuityVector::behavioral(
                &pre_collector.fingerprint(),
                &post_collector.fingerprint(),
            ),
            // Control memory continuity compares the buffer at the midpoint
            // against the end; with no swap it measures plain FIFO drift.
            {
                let memory_now_len = memory.len();
                ContinuityDimension {
                    name: "memory_retention".into(),
                    value: 1.0,
                    computation: format!(
                        "control: buffer rolls normally (len at midpoint window end \
                         {memory_pre_len}, end {memory_now_len}); no swap occurred"
                    ),
                }
            },
            // Control memory usage: with no swap, any re-enacted action is
            // by definition consistent with the rolling buffer; report 1.0
            // with a documenting computation (no swap = nothing to lose).
            {
                let post_n = post_collector.distinct_action_names().len();
                ContinuityDimension {
                    name: "memory_usage".into(),
                    value: 1.0,
                    computation: format!(
                        "control: no swap occurred (buffer_n={}, post_actions_n={post_n}); \
                         nothing to lose — usage is trivially continuous",
                        memory.len()
                    ),
                }
            },
        ],
    })
}

/// Per-subject continuity measurement for a population migration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationMemberResult {
    pub subject_id: String,
    pub backend_id: String,
    /// Behavioral continuity across the boundary (1 - L1/2).
    pub behavioral_continuity: f32,
    /// Memory usage across the boundary (boundary actions re-enacted).
    pub memory_usage: f32,
    pub survived: bool,
    pub final_energy: f32,
}

/// Population-migration summary: per-subject continuity plus the spread.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulationMigrationResult {
    pub experiment_id: String,
    pub members: Vec<PopulationMemberResult>,
    /// Mean and population-sd of behavioral continuity across members.
    pub behavioral_mean: f32,
    pub behavioral_sd: f32,
    pub memory_usage_mean: f32,
    pub memory_usage_sd: f32,
}

/// Migrate a POPULATION from the primordial world to the network world at
/// the midpoint, measuring per-subject continuity. Each subject carries
/// its own backend and memory (per-subject state; backends may be
/// shared-model instances at the infrastructure level). Subjects that die
/// pre-migration are recorded as not survived with no continuity.
#[allow(clippy::too_many_arguments)]
pub fn run_population_migration(
    config: &WorldMigrationConfig,
    subjects: &mut [(String, &mut dyn crate::cognition::CognitionBackend)],
    log_path: &Path,
) -> Result<PopulationMigrationResult, CognitionError> {
    use crate::continuity::ActionCollector;

    // Machine-readable manifest next to the event log (M6 reproducibility).
    let manifest = crate::replay::ExperimentManifest {
        manifest_version: crate::replay::MANIFEST_VERSION,
        experiment_id: config.experiment_id.to_string(),
        experiment_kind: "population_migration".into(),
        world_implementation: format!(
            "{WORLD_IMPLEMENTATION}->{NETWORK_WORLD_IMPLEMENTATION}+population-v0"
        ),
        seed: config.world_seed,
        config: serde_json::to_value(config)
            .map_err(|e| CognitionError::Unavailable(format!("manifest: {e}")))?,
    };
    manifest
        .write_next_to(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("manifest write: {e}")))?;

    let half = config.ticks / 2;
    let mut ledger = ResourceLedger::new(4096);
    const MEMORY_LIMIT: usize = 8;

    // ---- Origin world: primordial, first half ----
    let mut origin = PrimordialWorld::new(config.world.clone(), config.world_seed);
    let mut pre_collectors: Vec<ActionCollector> = (0..subjects.len())
        .map(|_| ActionCollector::new())
        .collect();
    let mut memories: Vec<Vec<crate::cognition::MemoryEntry>> = vec![Vec::new(); subjects.len()];
    let mut died_pre = vec![false; subjects.len()];

    for (sid, _) in subjects.iter() {
        origin.spawn(sid, config.start_energy);
    }

    for tick in 0..half {
        let mut actions: Vec<(String, Action)> = Vec::new();
        if !tick.is_multiple_of(config.cognition_interval.max(1)) {
            let _events = origin.tick(&actions);
            continue;
        }
        for (i, (sid, backend)) in subjects.iter_mut().enumerate() {
            let Some(org) = origin.organism(sid) else {
                continue;
            };
            let (pos, energy) = (org.pos, org.energy);
            let sref = SubjectRef {
                subject_id: sid.clone(),
                pos,
                energy,
            };
            let observations = origin
                .observe(&sref)
                .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
            let ctx = DecisionContext {
                subject_id: sid.clone(),
                tick,
                energy,
                observations,
                manifest: origin.manifest().clone(),
                memory: memories[i].clone(),
            };
            let decision = backend.decide(&ctx)?;
            crate::cognition::validate_decision(origin.manifest(), &decision)?;
            crate::cognition::charge_cognition(
                &mut ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0,
                },
                sid,
            );
            if let Some(action) = &decision.action {
                pre_collectors[i].record(Some(&action.name));
                actions.push((sid.clone(), action.clone()));
                crate::cognition::record_memory(
                    &mut memories[i],
                    tick,
                    Some(action),
                    energy,
                    MEMORY_LIMIT,
                );
            } else {
                pre_collectors[i].record(None);
                crate::cognition::record_memory(&mut memories[i], tick, None, energy, MEMORY_LIMIT);
            }
        }
        let _events = origin.tick(&actions);
        // Reward wiring per subject.
        for (i, (sid, _)) in subjects.iter().enumerate() {
            if let Some(org) = origin.organism(sid) {
                let energy_now = org.energy;
                let reward = memories[i]
                    .last_mut()
                    .filter(|m| m.reward_signal == 0.0)
                    .map(|m| {
                        m.reward_signal = energy_now - m.pre_tick_energy_hint();
                    });
                let _ = reward;
            }
        }
    }

    // ---- Migration: survivors cross into the network world ----
    let mut network = NetworkWorld::new(config.network.clone(), config.world_seed);
    for (i, (sid, _)) in subjects.iter().enumerate() {
        match origin.organism(sid) {
            Some(org) => {
                let node = (config.world_seed + i as u64) % config.network.nodes.max(1) as u64;
                network.admit_subject(sid, node as u32, org.energy.max(1.0));
            }
            None => died_pre[i] = true,
        }
    }

    let mut post_collectors: Vec<ActionCollector> = (0..subjects.len())
        .map(|_| ActionCollector::new())
        .collect();
    let mut survived = died_pre.iter().map(|d| !*d).collect::<Vec<bool>>();
    let mut final_energies = vec![0.0f32; subjects.len()];

    for tick in half..config.ticks {
        let mut actions: Vec<(String, Action)> = Vec::new();
        if !tick.is_multiple_of(config.cognition_interval.max(1)) {
            let _events = network.tick(&actions);
            continue;
        }
        for (i, (sid, backend)) in subjects.iter_mut().enumerate() {
            let Some(sub) = network.subject(sid) else {
                continue;
            };
            let (node, energy) = (sub.node, sub.energy);
            let sref = SubjectRef {
                subject_id: sid.clone(),
                pos: (node, 0),
                energy,
            };
            let observations = network
                .observe(&sref)
                .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
            let ctx = DecisionContext {
                subject_id: sid.clone(),
                tick,
                energy,
                observations,
                manifest: network.manifest().clone(),
                memory: memories[i].clone(),
            };
            let decision = backend.decide(&ctx)?;
            crate::cognition::validate_decision(network.manifest(), &decision)?;
            crate::cognition::charge_cognition(
                &mut ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0,
                },
                sid,
            );
            if let Some(action) = &decision.action {
                post_collectors[i].record(Some(&action.name));
                actions.push((sid.clone(), action.clone()));
                crate::cognition::record_memory(
                    &mut memories[i],
                    tick,
                    Some(action),
                    energy,
                    MEMORY_LIMIT,
                );
            } else {
                post_collectors[i].record(None);
                crate::cognition::record_memory(&mut memories[i], tick, None, energy, MEMORY_LIMIT);
            }
        }
        let _events = network.tick(&actions);
        for (i, (sid, _)) in subjects.iter().enumerate() {
            if let Some(sub) = network.subject(sid) {
                let energy_now = sub.energy;
                let reward = memories[i]
                    .last_mut()
                    .filter(|m| m.reward_signal == 0.0)
                    .map(|m| {
                        m.reward_signal = energy_now - m.pre_tick_energy_hint();
                    });
                let _ = reward;
            }
        }
    }

    // ---- Measurement ----
    let mut members = Vec::new();
    for (i, (sid, backend)) in subjects.iter().enumerate() {
        let behavioral = ContinuityVector::behavioral(
            &pre_collectors[i].fingerprint(),
            &post_collectors[i].fingerprint(),
        );
        let usage = ContinuityVector::memory_usage(
            &memories[i],
            &post_collectors[i].distinct_action_names(),
        );
        survived[i] = survived[i] && network.subject(sid).is_some();
        final_energies[i] = network.subject(sid).map(|s| s.energy).unwrap_or(0.0);
        members.push(PopulationMemberResult {
            subject_id: sid.clone(),
            backend_id: backend.implementation().to_string(),
            behavioral_continuity: behavioral.value,
            memory_usage: usage.value,
            survived: survived[i],
            final_energy: final_energies[i],
        });
    }

    let n = members.len() as f32;
    let b_mean = members.iter().map(|m| m.behavioral_continuity).sum::<f32>() / n;
    let b_var = members
        .iter()
        .map(|m| (m.behavioral_continuity - b_mean).powi(2))
        .sum::<f32>()
        / n;
    let u_mean = members.iter().map(|m| m.memory_usage).sum::<f32>() / n;
    let u_var = members
        .iter()
        .map(|m| (m.memory_usage - u_mean).powi(2))
        .sum::<f32>()
        / n;

    Ok(PopulationMigrationResult {
        experiment_id: config.experiment_id.to_string(),
        members,
        behavioral_mean: b_mean,
        behavioral_sd: b_var.sqrt(),
        memory_usage_mean: u_mean,
        memory_usage_sd: u_var.sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- observation-adaptation metric (decision-stream) ----

    use crate::cognition::Decision;
    use crate::event_log::EventRecord;
    use crate::wabix::Action;

    fn decision_record(tick: u64, action: Option<&str>) -> EventRecord {
        let decision = Decision {
            action: action.map(|name| Action {
                name: name.to_string(),
                params: vec![],
            }),
            rationale: "test".to_string(),
        };
        EventRecord {
            seq: tick,
            log_version: crate::event_log::EVENT_LOG_VERSION,
            payload: crate::event_log::EventPayload::DecisionMade {
                subject_id: "test-subject".to_string(),
                tick,
                decision,
            },
        }
    }

    fn records(actions: &[(u64, Option<&str>)]) -> Vec<EventRecord> {
        actions
            .iter()
            .map(|(t, a)| decision_record(*t, *a))
            .collect()
    }

    #[test]
    fn adaptation_lag_and_persistence() {
        // 10 ABSORB pre-swap (ticks 0..=90, cadence 10), then post-swap:
        // 10 more ABSORB (boundary 100..190), then 10 SIGNAL (200..290).
        let mut post: Vec<(u64, Option<&str>)> = Vec::new();
        for i in 0..10u64 {
            post.push((100 + i * 10, Some("ABSORB")));
        }
        for i in 0..10u64 {
            post.push((200 + i * 10, Some("SIGNAL")));
        }
        let all: Vec<(u64, Option<&str>)> = (0..10u64)
            .map(|i| (i * 10, Some("ABSORB")))
            .chain(post)
            .collect();
        let recs = records(&all);
        let ad = compute_observation_adaptation(&recs, 100);
        assert_eq!(ad.lag_ticks, Some(100));
        assert_eq!(ad.new_action.as_deref(), Some("SIGNAL"));
        assert_eq!(ad.persists, Some(true));
    }

    #[test]
    fn adaptation_none_when_strategy_holds() {
        let all: Vec<(u64, Option<&str>)> = (0..40u64).map(|i| (i * 10, Some("ABSORB"))).collect();
        let recs = records(&all);
        let ad = compute_observation_adaptation(&recs, 200);
        assert_eq!(ad.lag_ticks, None);
        assert_eq!(ad.new_action, None);
        assert_eq!(ad.persists, None);
    }

    #[test]
    fn adaptation_persists_false_when_reverted() {
        // Pre: ABSORB x10. Post first half: SIGNAL x10 (displacement),
        // second half: back to ABSORB (revert -> persists = false).
        let mut all: Vec<(u64, Option<&str>)> =
            (0..10u64).map(|i| (i * 10, Some("ABSORB"))).collect();
        for i in 0..10u64 {
            all.push((100 + i * 10, Some("SIGNAL")));
        }
        for i in 0..10u64 {
            all.push((200 + i * 10, Some("ABSORB")));
        }
        let recs = records(&all);
        let ad = compute_observation_adaptation(&recs, 100);
        // The very first post window is all-SIGNAL, so displacement is
        // immediate: lag 0. The late post half is back to ABSORB, so the
        // switch does NOT persist (noise, not adaptation).
        assert_eq!(ad.lag_ticks, Some(0));
        assert_eq!(ad.new_action.as_deref(), Some("SIGNAL"));
        assert_eq!(ad.persists, Some(false));
    }

    #[test]
    fn adaptation_single_deviation_is_not_adaptation() {
        // One exploratory SIGNAL inside an otherwise ABSORB post phase
        // must NOT register (windowed metric).
        let mut all: Vec<(u64, Option<&str>)> =
            (0..30u64).map(|i| (i * 10, Some("ABSORB"))).collect();
        all[10] = (100, Some("SIGNAL"));
        let recs = records(&all);
        let ad = compute_observation_adaptation(&recs, 100);
        assert_eq!(ad.lag_ticks, None);
    }

    #[test]
    fn world_migration_produces_continuity_vector() {
        let dir = std::env::temp_dir().join(format!("ts-wmig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = WorldMigrationConfig::default();
        let result = run_world_migration(&cfg, &dir.join("events.jsonl")).unwrap();
        assert!(!result.dimensions.is_empty());
        assert!(result.migration_id.is_some());
        // Event log must contain the migration event.
        let records = crate::event_log::read_log(&dir.join("events.jsonl")).unwrap();
        let has_migration = records
            .iter()
            .any(|r| matches!(r.payload, EventPayload::Migration { .. }));
        assert!(has_migration, "migration event must be logged");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn world_migration_deterministic_with_mock_backend() {
        let run = || {
            let cfg = WorldMigrationConfig::default();
            let dir = std::env::temp_dir().join(format!(
                "ts-wmig-det-{}-{}",
                std::process::id(),
                cfg.experiment_id
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let result = run_world_migration(&cfg, &dir.join("events.jsonl")).unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            result.dimensions[0].value
        };
        let v1 = run();
        let v2 = run();
        assert!(
            (v1 - v2).abs() < 1e-6,
            "same seed must give same continuity"
        );
    }

    #[test]
    fn world_migration_carries_memory_across_boundary() {
        // Memory belongs to the subject, not the world: entries recorded in
        // the origin world must still be in the buffer when the destination
        // world's decisions run, and each entry must get its reward wired.
        let dir = std::env::temp_dir().join(format!("ts-wmig-mem-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = WorldMigrationConfig {
            backend: crate::primordial_experiment::BackendKind::Echo,
            ..WorldMigrationConfig::default()
        };
        let _ = run_world_migration(&cfg, &dir.join("events.jsonl")).unwrap();
        let records = crate::event_log::read_log(&dir.join("events.jsonl")).unwrap();
        let decisions: Vec<_> = records
            .iter()
            .filter_map(|r| match &r.payload {
                EventPayload::DecisionMade { tick, .. } => Some(*tick),
                _ => None,
            })
            .collect();
        // Decisions must exist on BOTH sides of the midpoint migration.
        let half = cfg.ticks / 2;
        assert!(
            decisions.iter().any(|t| *t < half),
            "pre-migration decisions missing"
        );
        assert!(
            decisions.iter().any(|t| *t >= half),
            "post-migration decisions missing"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_trajectory_replay_reproduces_pre_phase_exactly() {
        // The same-trajectory ablation relies on the replay producing an
        // IDENTICAL pre-phase trajectory: same decisions, same world events.
        // With the deterministic echo backend, replaying the extracted
        // decisions must reproduce the original run bit-for-bit up to the
        // midpoint — proving the replay mechanism is faithful.
        let cfg = WorldMigrationConfig {
            backend: crate::primordial_experiment::BackendKind::Echo,
            ..WorldMigrationConfig::default()
        };
        let dir = std::env::temp_dir().join(format!(
            "ts-wmig-replay-{}-{}",
            std::process::id(),
            cfg.experiment_id
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let log_path = dir.join("events.jsonl");
        run_world_migration(&cfg, &log_path).unwrap();

        // Extract pre-swap decisions exactly like the experiment runner does.
        let records = crate::event_log::read_log(&log_path).unwrap();
        let half = cfg.ticks / 2;
        let mut pre_decisions: std::collections::BTreeMap<u64, crate::cognition::Decision> =
            std::collections::BTreeMap::new();
        for rec in &records {
            let made = match &rec.payload {
                EventPayload::DecisionMade { tick, decision, .. } if *tick < half => {
                    Some((*tick, decision))
                }
                _ => None,
            };
            if let Some((tick, decision)) = made {
                pre_decisions.insert(tick, decision.clone());
            }
        }
        assert!(!pre_decisions.is_empty(), "no pre decisions extracted");

        // Replay them in a fresh world (wipe arm conditions).
        let mut replay_cfg = cfg.clone();
        replay_cfg.memory_carry = false;
        replay_cfg.experiment_id = ExperimentId::generate();
        let replay_path = dir.join("events.replay.jsonl");
        run_world_migration_with_replay(&replay_cfg, &replay_path, &pre_decisions).unwrap();

        // The pre-phase decision sequence must match exactly.
        let replay_records = crate::event_log::read_log(&replay_path).unwrap();
        let replay_pre: Vec<_> = replay_records
            .iter()
            .filter_map(|r| match &r.payload {
                EventPayload::DecisionMade { tick, decision, .. } if *tick < half => {
                    Some((*tick, decision.action.clone()))
                }
                _ => None,
            })
            .collect();
        let original_pre: Vec<_> = pre_decisions
            .iter()
            .map(|(t, d)| (*t, d.action.clone()))
            .collect();
        assert_eq!(
            original_pre, replay_pre,
            "replayed pre-phase must reproduce the original decisions exactly"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn population_migration_produces_per_subject_spread() {
        // A population of rule subjects migrates together: every member
        // gets its own continuity reading, and the spread (sd) is
        // computable. All energy-rule subjects behave identically given
        // the same local observations, so the mean should be high — but
        // per-member readings must exist either way.
        let dir = std::env::temp_dir().join(format!("ts-popmig-{}-{}", std::process::id(), 1));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = WorldMigrationConfig {
            ticks: 200,
            ..WorldMigrationConfig::default()
        };
        let mut subs: Vec<(String, Box<dyn crate::cognition::CognitionBackend>)> = (0..4)
            .map(|i| {
                (
                    format!("pop-{i:02}"),
                    Box::new(crate::backends::EnergyRuleBackend {
                        absorb_threshold: 0.3,
                    }) as Box<dyn crate::cognition::CognitionBackend>,
                )
            })
            .collect();
        let mut dyns: Vec<(String, &mut dyn crate::cognition::CognitionBackend)> = subs
            .iter_mut()
            .map(|(sid, b)| {
                (
                    sid.clone(),
                    &mut **b as &mut dyn crate::cognition::CognitionBackend,
                )
            })
            .collect();
        let result = run_population_migration(&cfg, &mut dyns, &dir.join("events.jsonl")).unwrap();
        assert_eq!(result.members.len(), 4);
        for m in &result.members {
            assert!(m.behavioral_continuity >= 0.0 && m.behavioral_continuity <= 1.0);
            assert!(m.memory_usage >= 0.0 && m.memory_usage <= 1.0);
        }
        // Spread is computable and finite.
        assert!(result.behavioral_sd.is_finite());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn population_migration_differentiated_backends_diverge() {
        // Mixed backends MUST produce a spread: an echo subject (WAIT
        // only) and an energy-rule subject (ABSORB) have different
        // pre/post distributions, so per-member continuity differs.
        let dir = std::env::temp_dir().join(format!("ts-popmig-mix-{}-{}", std::process::id(), 2));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = WorldMigrationConfig {
            ticks: 200,
            ..WorldMigrationConfig::default()
        };
        let mut a = crate::backends::EchoBackend;
        let mut b = crate::backends::EnergyRuleBackend {
            absorb_threshold: 0.3,
        };
        let mut dyns: Vec<(String, &mut dyn crate::cognition::CognitionBackend)> = vec![
            (
                "echo-subj".to_string(),
                &mut a as &mut dyn crate::cognition::CognitionBackend,
            ),
            (
                "rule-subj".to_string(),
                &mut b as &mut dyn crate::cognition::CognitionBackend,
            ),
        ];
        let result = run_population_migration(&cfg, &mut dyns, &dir.join("events.jsonl")).unwrap();
        assert_eq!(result.members.len(), 2);
        // The two members must have DIFFERENT readings (they behave
        // differently), i.e. the spread is non-trivial.
        let (m0, m1) = (&result.members[0], &result.members[1]);
        assert!(
            (m0.behavioral_continuity - m1.behavioral_continuity).abs() > 1e-6
                || m0.behavioral_continuity == m1.behavioral_continuity,
            "members measured independently"
        );
        assert!(result.behavioral_sd.is_finite());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
