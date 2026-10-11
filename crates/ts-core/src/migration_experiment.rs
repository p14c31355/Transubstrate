//! M5: Controller migration experiment — the first real continuity
//! experiment class.
//!
//! Design (A/B with controls, deterministic):
//! - MIGRATION run: one subject in the primordial world runs with backend A
//!   for T/2 ticks, then swaps to backend B at the midpoint (a MigrationEvent
//!   is recorded), and runs T/2 more ticks with B.
//! - CONTROL-A run: same seed, backend A for all T ticks.
//! - CONTROL-B run: same seed, backend B for all T ticks.
//!
//! Behavioral continuity is measured by comparing action fingerprints:
//! pre-migration window vs post-migration window (migration run), and — as
//! the control contrast — control-A's first-half vs second-half fingerprint
//! (no migration; same backend throughout). The migration effect is the
//! difference between the two continuity values.
//!
//! CAVEAT (documented honestly): with rule-based mock backends, differences
//! mostly reflect the controllers' built-in policies, not learned continuity.
//! This harness exists so that when real cognition backends (M7) carry
//! per-subject memory, the same measurement becomes meaningful. Negative /
//! trivial results here are still valid calibration data.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::cognition::{CognitionBackend, CognitionError, DecisionContext, record_memory};
use crate::continuity::{ActionCollector, ContinuityDimension, ContinuityVector, MigrationEvent};
use crate::event_log::{EventLog, EventPayload};
use crate::ids::{ExperimentId, SubjectId};
use crate::primordial::{PrimordialConfig, PrimordialWorld, WORLD_IMPLEMENTATION};
use crate::primordial_experiment::BackendKind;
use crate::replay::{ExperimentManifest, MANIFEST_VERSION};
use crate::resource::ResourceLedger;
use crate::runtime::CognitionCadence;
use crate::wabix::{Action, SubjectRef, World};

/// Configuration for the controller-migration experiment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationExperimentConfig {
    pub experiment_id: ExperimentId,
    pub world: PrimordialConfig,
    pub world_seed: u64,
    pub ticks: u64,
    pub cognition_interval: u64,
    pub start_energy: f32,
    /// Backend before migration.
    pub backend_a: BackendKind,
    /// Backend after migration.
    pub backend_b: BackendKind,
}

impl MigrationExperimentConfig {
    pub fn new(backend_a: BackendKind, backend_b: BackendKind) -> Self {
        Self {
            experiment_id: ExperimentId::generate(),
            world: PrimordialConfig::default(),
            world_seed: 42,
            ticks: 400,
            cognition_interval: 5,
            start_energy: 30.0,
            backend_a,
            backend_b,
        }
    }
}

/// One phase's measurement window.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhaseMeasurement {
    pub label: String,
    pub backend_id: String,
    pub tick_from: u64,
    pub tick_to: u64,
    pub actions_taken: u64,
    pub fingerprint: crate::continuity::BehaviorFingerprint,
    /// Mean organism energy at window end (single-subject experiment).
    pub final_energy: f32,
    pub survived: bool,
}

/// Result of one run (migration or control).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub run_kind: String, // "migration" | "control_a" | "control_b"
    pub phases: Vec<PhaseMeasurement>,
    pub deaths: u32,
    pub births: u32,
}

/// Full experiment result with the continuity comparison.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationExperimentResult {
    pub experiment_id: String,
    pub migration: Option<MigrationEvent>,
    pub migration_run: RunResult,
    pub control_a: RunResult,
    pub control_b: RunResult,
    /// Continuity vector for the migration run (pre vs post fingerprint).
    pub continuity: ContinuityVector,
    /// Control contrast: control-A first-half vs second-half continuity.
    pub control_continuity: ContinuityDimension,
    /// The migration effect estimate: migration_continuity - control_continuity.
    /// Negative = behavior changed more than the no-migration baseline.
    pub migration_effect: f32,
}

/// Run one single-subject phase with a fixed backend, collecting actions.
#[allow(clippy::too_many_arguments)]
fn run_phase(
    world: &mut PrimordialWorld,
    subject_id: &str,
    backend: &mut dyn CognitionBackend,
    backend_id: &str,
    ticks: u64,
    tick_offset: u64,
    cadence: &CognitionCadence,
    ledger: &mut ResourceLedger,
    log: &mut EventLog,
    collector: &mut ActionCollector,
    memory: &mut Vec<crate::cognition::MemoryEntry>,
    memory_limit: usize,
) -> Result<PhaseMeasurement, CognitionError> {
    let tick_from = tick_offset;
    let tick_to = tick_offset + ticks;
    let mut actions_taken = 0u64;

    for local in 0..ticks {
        let tick = tick_offset + local;
        let mut actions: Vec<(String, Action)> = Vec::new();

        if cadence.fires_on(tick) {
            // Single-subject: find the living organism.
            if let Some(org) = world.organism(subject_id) {
                let sref = SubjectRef {
                    subject_id: subject_id.to_string(),
                    pos: org.pos,
                    energy: org.energy,
                };
                let observations = match world.observe(&sref) {
                    Ok(o) => o,
                    Err(e) => {
                        return Err(CognitionError::Unavailable(format!("observe failed: {e}")));
                    }
                };
                let ctx = DecisionContext {
                    subject_id: subject_id.to_string(),
                    tick,
                    energy: sref.energy,
                    observations,
                    manifest: world.manifest().clone(),
                    memory: memory.clone(),
                };
                let decision = backend.decide(&ctx)?;
                crate::cognition::validate_decision(world.manifest(), &decision)?;
                crate::cognition::charge_cognition(
                    ledger,
                    tick,
                    &crate::cognition::CognitionCost {
                        cognition_events: 1,
                        tokens: 0,
                    },
                    subject_id,
                );
                log.push(EventPayload::DecisionMade {
                    subject_id: subject_id.to_string(),
                    tick,
                    decision: decision.clone(),
                });
                if let Some(action) = decision.action {
                    collector.record(Some(&action.name));
                    actions_taken += 1;
                    actions.push((subject_id.to_string(), action.clone()));
                    record_memory(memory, tick, Some(&action), sref.energy, memory_limit);
                } else {
                    collector.record(None);
                    record_memory(memory, tick, None, sref.energy, memory_limit);
                }
            }
        }

        let _events = world.tick(&actions);

        // Reward wiring: energy delta from pre-decision energy.
        if cadence.fires_on(tick) {
            let updated = world.organism(subject_id).and_then(|org| {
                memory
                    .last_mut()
                    .filter(|last| last.reward_signal == 0.0)
                    .map(|last| {
                        last.reward_signal = org.energy - last.pre_tick_energy_hint();
                    })
            });
            let _ = updated;
        }
    }

    let survived = world.organism(subject_id).is_some();
    let final_energy = world.organism(subject_id).map(|o| o.energy).unwrap_or(0.0);

    Ok(PhaseMeasurement {
        label: format!("{backend_id}@{tick_from}-{tick_to}"),
        backend_id: backend_id.to_string(),
        tick_from,
        tick_to,
        actions_taken,
        fingerprint: collector.fingerprint(),
        final_energy,
        survived,
    })
}

/// Run the full migration experiment (migration run + two controls).
pub fn run_migration_experiment(
    config: &MigrationExperimentConfig,
    log_path: &Path,
) -> Result<MigrationExperimentResult, CognitionError> {
    let mut log = EventLog::new(8192)
        .with_sink(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("log sink: {e}")))?;

    let manifest = ExperimentManifest {
        manifest_version: MANIFEST_VERSION,
        experiment_id: config.experiment_id.to_string(),
        experiment_kind: "migration".into(),
        world_implementation: format!("{WORLD_IMPLEMENTATION}+migration-v0"),
        seed: config.world_seed,
        config: serde_json::to_value(config)
            .map_err(|e| CognitionError::Unavailable(format!("manifest: {e}")))?,
    };
    manifest
        .write_next_to(log_path)
        .map_err(|e| CognitionError::Unavailable(format!("manifest write: {e}")))?;

    log.push(EventPayload::ExperimentStarted {
        experiment_id: config.experiment_id.to_string(),
        world_implementation: format!("{WORLD_IMPLEMENTATION}+migration-v0"),
        seed: config.world_seed,
    });

    let half = config.ticks / 2;
    let cadence = CognitionCadence::every(config.cognition_interval);

    // ---- Migration run: subject runs A then B, same world, same seed ----
    let subject = SubjectId::from_static("mig-subject");
    let sid = subject.to_string();

    let mut world = PrimordialWorld::new(config.world.clone(), config.world_seed);
    world.spawn(&sid, config.start_energy);
    log.push(EventPayload::SubjectCreated {
        subject_id: sid.clone(),
        subject_ref: SubjectRef {
            subject_id: sid.clone(),
            pos: (0, 0), // world's actual spawn pos is deterministic; ref updated below
            energy: config.start_energy,
        },
    });

    let mut ledger = ResourceLedger::new(4096);
    let mut collector_a = ActionCollector::new();
    let mut collector_b = ActionCollector::new();
    // Per-subject bounded memory persists ACROSS the migration boundary —
    // this is the whole point: memory belongs to the subject, not the
    // backend (SUBJECT != MODEL).
    let mut memory: Vec<crate::cognition::MemoryEntry> = Vec::new();
    const MEMORY_LIMIT: usize = 8;

    let mut backend_a = config.backend_a.instantiate();
    let phase_a = run_phase(
        &mut world,
        &sid,
        backend_a.as_mut(),
        config.backend_a.backend_id(),
        half,
        0,
        &cadence,
        &mut ledger,
        &mut log,
        &mut collector_a,
        &mut memory,
        MEMORY_LIMIT,
    )?;

    // Record the migration event at the midpoint.
    let migration = MigrationEvent::new(
        &sid,
        "cognition_backend",
        config.backend_a.backend_id(),
        config.backend_b.backend_id(),
        half,
        "incarnation-1".into(),
        "incarnation-2".into(),
    );
    log.push(EventPayload::Migration {
        subject_id: sid.clone(),
        migration_id: migration.migration_id.clone(),
        dimension: migration.dimension.clone(),
        from: migration.from.clone(),
        to: migration.to.clone(),
        tick: half,
    });

    let mut backend_b = config.backend_b.instantiate();
    let phase_b = run_phase(
        &mut world,
        &sid,
        backend_b.as_mut(),
        config.backend_b.backend_id(),
        config.ticks - half,
        half,
        &cadence,
        &mut ledger,
        &mut log,
        &mut collector_b,
        &mut memory,
        MEMORY_LIMIT,
    )?;

    let migration_run = RunResult {
        run_kind: "migration".into(),
        phases: vec![phase_a.clone(), phase_b.clone()],
        deaths: if phase_a.survived && phase_b.survived {
            0
        } else {
            1
        },
        births: 0,
    };

    // ---- Control A: same seed, backend A throughout ----
    let mut world_a = PrimordialWorld::new(config.world.clone(), config.world_seed);
    world_a.spawn(&sid, config.start_energy);
    let mut ledger_a = ResourceLedger::new(4096);
    let mut collector_ca1 = ActionCollector::new();
    let mut collector_ca2 = ActionCollector::new();
    let mut memory_a: Vec<crate::cognition::MemoryEntry> = Vec::new();
    let mut ctrl_backend_a = config.backend_a.instantiate();
    let ctrl_a_phase1 = run_phase(
        &mut world_a,
        &sid,
        ctrl_backend_a.as_mut(),
        config.backend_a.backend_id(),
        half,
        0,
        &cadence,
        &mut ledger_a,
        &mut log,
        &mut collector_ca1,
        &mut memory_a,
        MEMORY_LIMIT,
    )?;
    let ctrl_a_phase2 = run_phase(
        &mut world_a,
        &sid,
        ctrl_backend_a.as_mut(),
        config.backend_a.backend_id(),
        config.ticks - half,
        half,
        &cadence,
        &mut ledger_a,
        &mut log,
        &mut collector_ca2,
        &mut memory_a,
        MEMORY_LIMIT,
    )?;
    let control_a = RunResult {
        run_kind: "control_a".into(),
        phases: vec![ctrl_a_phase1, ctrl_a_phase2],
        deaths: 0,
        births: 0,
    };

    // ---- Control B: same seed, backend B throughout ----
    let mut world_b = PrimordialWorld::new(config.world.clone(), config.world_seed);
    world_b.spawn(&sid, config.start_energy);
    let mut ledger_b = ResourceLedger::new(4096);
    let mut collector_cb1 = ActionCollector::new();
    let mut collector_cb2 = ActionCollector::new();
    let mut memory_b: Vec<crate::cognition::MemoryEntry> = Vec::new();
    let mut ctrl_backend_b = config.backend_b.instantiate();
    let ctrl_b_phase1 = run_phase(
        &mut world_b,
        &sid,
        ctrl_backend_b.as_mut(),
        config.backend_b.backend_id(),
        half,
        0,
        &cadence,
        &mut ledger_b,
        &mut log,
        &mut collector_cb1,
        &mut memory_b,
        MEMORY_LIMIT,
    )?;
    let ctrl_b_phase2 = run_phase(
        &mut world_b,
        &sid,
        ctrl_backend_b.as_mut(),
        config.backend_b.backend_id(),
        config.ticks - half,
        half,
        &cadence,
        &mut ledger_b,
        &mut log,
        &mut collector_cb2,
        &mut memory_b,
        MEMORY_LIMIT,
    )?;
    let control_b = RunResult {
        run_kind: "control_b".into(),
        phases: vec![ctrl_b_phase1, ctrl_b_phase2],
        deaths: 0,
        births: 0,
    };

    // ---- Continuity measurement ----
    let behavioral = ContinuityVector::behavioral(&phase_a.fingerprint, &phase_b.fingerprint);
    let control_continuity = ContinuityVector::behavioral(
        &control_a.phases[0].fingerprint,
        &control_a.phases[1].fingerprint,
    );
    let migration_effect = behavioral.value - control_continuity.value;

    let continuity = ContinuityVector {
        subject_id: sid.clone(),
        migration_id: Some(migration.migration_id.clone()),
        dimensions: vec![behavioral],
    };

    log.push(EventPayload::ExperimentFinished {
        reason: "migration experiment complete".into(),
    });

    Ok(MigrationExperimentResult {
        experiment_id: config.experiment_id.to_string(),
        migration: Some(migration),
        migration_run,
        control_a,
        control_b,
        continuity,
        control_continuity,
        migration_effect,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_backend_migration_has_high_continuity() {
        // Migrating echo -> echo should look like the control (no behavior
        // change): migration effect near zero.
        let dir = std::env::temp_dir().join(format!("ts-mig-same-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = MigrationExperimentConfig::new(BackendKind::Echo, BackendKind::Echo);
        let result = run_migration_experiment(&cfg, &dir.join("events.jsonl")).unwrap();
        assert!(
            (result.migration_effect.abs() < 1e-6),
            "echo->echo must match control exactly; effect={}",
            result.migration_effect
        );
        assert_eq!(result.continuity.dimensions[0].value, 1.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn different_backend_migration_changes_behavior() {
        // echo -> energy-rule: the rule actively absorbs; echo waits.
        // Migration effect must be strongly negative (big behavior change).
        let dir = std::env::temp_dir().join(format!("ts-mig-diff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = MigrationExperimentConfig::new(
            BackendKind::Echo,
            BackendKind::EnergyRule {
                absorb_threshold: 0.3,
            },
        );
        let result = run_migration_experiment(&cfg, &dir.join("events.jsonl")).unwrap();
        assert!(
            result.migration_effect < -0.5,
            "echo->energy-rule must change behavior dramatically; effect={}",
            result.migration_effect
        );
        // The migration event is recorded with deterministic id.
        assert!(result.migration.is_some());
        assert!(
            result
                .migration
                .as_ref()
                .unwrap()
                .migration_id
                .starts_with("mig-")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn control_a_equals_migration_pre_window() {
        // The pre-migration phase of the migration run and control-A's first
        // half use the same seed/backend: their fingerprints must match.
        let dir = std::env::temp_dir().join(format!("ts-mig-ctrl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = MigrationExperimentConfig::new(
            BackendKind::EnergyRule {
                absorb_threshold: 0.3,
            },
            BackendKind::Echo,
        );
        let result = run_migration_experiment(&cfg, &dir.join("events.jsonl")).unwrap();
        let pre = &result.migration_run.phases[0].fingerprint;
        let ctrl = &result.control_a.phases[0].fingerprint;
        assert_eq!(
            pre, ctrl,
            "same seed+backend => same pre-window fingerprint"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_event_is_logged() {
        let dir = std::env::temp_dir().join(format!("ts-mig-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = MigrationExperimentConfig::new(
            BackendKind::Echo,
            BackendKind::EnergyRule {
                absorb_threshold: 0.3,
            },
        );
        let _ = run_migration_experiment(&cfg, &dir.join("events.jsonl")).unwrap();
        let records = crate::event_log::read_log(&dir.join("events.jsonl")).unwrap();
        let has_migration = records
            .iter()
            .any(|r| matches!(r.payload, EventPayload::Migration { .. }));
        assert!(has_migration, "migration event must be in the log");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
