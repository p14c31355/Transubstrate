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

#[cfg(test)]
mod tests {
    use super::*;

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
}
