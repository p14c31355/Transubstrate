//! Experiment harness: wires subject + world + backend + ledger + event log
//! into a runnable experiment loop. This is the M2 vertical slice — the full
//! SUBJECT → COGNITION → WABIX → WORLD → OBSERVE → DECIDE → ACTION →
//! VALIDATE → WORLD TRANSITION → EVENT LOG path, executing for real.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::backends::{EchoBackend, EnergyRuleBackend, MalformedBackend, RandomBackend};
use crate::cognition::{CognitionBackend, CognitionError};
use crate::event_log::{EventLog, EventPayload};
use crate::ids::{ExperimentId, SubjectId};
use crate::resource::{ResourceEntry, ResourceKind};
use crate::runtime::{CognitionCadence, RuntimeError, Subject, SubjectRunner};
use crate::test_world::TestWorld;
use crate::wabix::SubjectRef;

/// Which cognition backend an experiment run uses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BackendKind {
    Echo,
    Random { seed: u64 },
    EnergyRule { absorb_threshold: f32 },
    Malformed,
}

impl BackendKind {
    pub fn instantiate(&self) -> Box<dyn CognitionBackend> {
        match self {
            BackendKind::Echo => Box::new(EchoBackend),
            BackendKind::Random { seed } => Box::new(RandomBackend::new(*seed)),
            BackendKind::EnergyRule { absorb_threshold } => Box::new(EnergyRuleBackend {
                absorb_threshold: *absorb_threshold,
            }),
            BackendKind::Malformed => Box::new(MalformedBackend::default()),
        }
    }

    pub fn backend_id(&self) -> &'static str {
        match self {
            BackendKind::Echo => "mock-echo-v0",
            BackendKind::Random { .. } => "mock-random-v0",
            BackendKind::EnergyRule { .. } => "rule-energy-v0",
            BackendKind::Malformed => "mock-malformed-v0",
        }
    }
}

/// Full experiment configuration (machine-readable manifest basis, M6).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub experiment_id: ExperimentId,
    pub world_seed: u64,
    pub ticks: u64,
    /// Cognition fires every `cognition_interval` ticks.
    pub cognition_interval: u64,
    pub backend: BackendKind,
    /// Per-tick maintenance cost charged to the subject's energy.
    pub maintenance_cost: f32,
    /// Starting energy for the subject.
    pub start_energy: f32,
}

/// Result summary of one experiment run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub experiment_id: String,
    pub backend_id: String,
    pub ticks_executed: u64,
    pub cognition_events: u64,
    pub decisions_with_action: u64,
    pub decisions_no_action: u64,
    pub backend_errors: u64,
    pub final_energy: f32,
    pub final_counter: f32,
    pub cognition_events_total: u128,
}

/// Run the vertical-slice experiment end to end, writing an event log.
pub fn run(config: &ExperimentConfig, log_path: &Path) -> Result<RunSummary, RuntimeError> {
    let mut log = EventLog::new(4096).with_sink(log_path).map_err(|e| {
        RuntimeError::Cognition(CognitionError::Unavailable(format!("log sink: {e}")))
    })?;

    log.push(EventPayload::ExperimentStarted {
        experiment_id: config.experiment_id.to_string(),
        world_implementation: "test-world-v0".into(),
        seed: config.world_seed,
    });

    let mut world = TestWorld::new(config.world_seed);
    let mut backend = config.backend.instantiate();

    let subject_id = SubjectId::generate();
    let mut subject = Subject::new(
        subject_id.clone(),
        crate::ids::CognitionBackendId::from_static(config.backend.backend_id()),
        64,
    );

    let mut runner = SubjectRunner::new(CognitionCadence::every(config.cognition_interval), 4096);
    runner.death_threshold = 0.0;

    let mut subject_ref = SubjectRef {
        subject_id: subject_id.to_string(),
        pos: (0, 0),
        energy: config.start_energy,
    };

    log.push(EventPayload::SubjectCreated {
        subject_id: subject_ref.subject_id.clone(),
        subject_ref: subject_ref.clone(),
    });

    let mut pending_action = None;
    let mut decisions_with_action = 0u64;
    let mut decisions_no_action = 0u64;
    let mut backend_errors = 0u64;
    let mut cognition_event_ticks = 0u64;

    for _tick in 0..config.ticks {
        // Maintenance cost each tick (world-agnostic resource drain).
        subject_ref.energy -= config.maintenance_cost;

        // Cognition cadence accounting: count cognition opportunities.
        if runner.cadence.fires_on(runner.current_tick) {
            cognition_event_ticks += 1;
        }

        match runner.tick(
            &mut subject,
            &mut world,
            &subject_ref,
            backend.as_mut(),
            pending_action.take(),
            &mut log,
        ) {
            Ok(action) => {
                if runner.cadence.fires_on(runner.current_tick - 1) {
                    if action.is_some() {
                        decisions_with_action += 1;
                    } else {
                        decisions_no_action += 1;
                    }
                }
                pending_action = action;
            }
            Err(RuntimeError::Cognition(err)) => {
                // Safe failure: backend errors are logged and the subject
                // continues without acting this cognition event.
                backend_errors += 1;
                log.push(EventPayload::WorldEvents {
                    tick: runner.current_tick,
                    events: vec![crate::wabix::WorldEvent::ActionRejected {
                        subject: subject_ref.subject_id.clone(),
                        action: "COGNITION".into(),
                        reason: format!("backend error: {err}"),
                    }],
                });
                pending_action = None;
            }
            Err(e) => return Err(e),
        }

        // Energy accounting into the ledger.
        runner.ledger.record(ResourceEntry {
            tick: runner.current_tick,
            kind: ResourceKind::Energy,
            amount: config.maintenance_cost as u64,
            consumer: subject_ref.subject_id.clone(),
        });

        // Death check: subject dies when energy hits the threshold.
        if subject_ref.energy <= runner.death_threshold {
            log.push(EventPayload::WorldEvents {
                tick: runner.current_tick,
                events: vec![crate::wabix::WorldEvent::SubjectDied {
                    subject: subject_ref.subject_id.clone(),
                    cause: "energy exhausted".into(),
                }],
            });
            break;
        }

        // Sampled state logging every 100 ticks.
        if runner.current_tick.is_multiple_of(100) {
            log.push(EventPayload::SubjectState {
                subject_id: subject_ref.subject_id.clone(),
                tick: runner.current_tick,
                pos: subject_ref.pos,
                energy: subject_ref.energy,
            });
        }
    }

    let experiment_id = config.experiment_id.to_string();
    log.push(EventPayload::ExperimentFinished {
        reason: "ticks elapsed or death".into(),
    });

    Ok(RunSummary {
        experiment_id,
        backend_id: config.backend.backend_id().to_string(),
        ticks_executed: runner.current_tick,
        cognition_events: cognition_event_ticks,
        decisions_with_action,
        decisions_no_action,
        backend_errors,
        final_energy: subject_ref.energy,
        final_counter: world.counter(&subject_ref.subject_id),
        cognition_events_total: runner.ledger.total(&ResourceKind::CognitionEvents),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(backend: BackendKind, ticks: u64) -> ExperimentConfig {
        ExperimentConfig {
            experiment_id: ExperimentId::generate(),
            world_seed: 42,
            ticks,
            cognition_interval: 4,
            backend,
            maintenance_cost: 0.01,
            start_energy: 1000.0,
        }
    }

    #[test]
    fn echo_run_produces_log_and_summary() {
        let dir = std::env::temp_dir().join(format!("ts-run-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let cfg = config(BackendKind::Echo, 20);
        let summary = run(&cfg, &path).unwrap();
        assert_eq!(summary.ticks_executed, 20);
        assert_eq!(summary.cognition_events, 5); // ticks 0,4,8,12,16
        assert_eq!(summary.backend_errors, 0);
        // Echo always picks the first action (WAIT), so counter stays 0.
        assert_eq!(summary.final_counter, 0.0);
        let records = crate::event_log::read_log(&path).unwrap();
        assert!(records.len() > 5);
        assert!(matches!(
            records.last().unwrap().payload,
            EventPayload::ExperimentFinished { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn random_run_bumps_counter() {
        let dir = std::env::temp_dir().join(format!("ts-run-r-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let cfg = config(BackendKind::Random { seed: 7 }, 40);
        let summary = run(&cfg, &path).unwrap();
        assert_eq!(summary.ticks_executed, 40);
        // Random backend should sometimes pick BUMP.
        assert!(summary.decisions_with_action > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_backend_fails_safely() {
        let dir = std::env::temp_dir().join(format!("ts-run-m-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let cfg = config(BackendKind::Malformed, 12);
        let summary = run(&cfg, &path).unwrap();
        assert_eq!(summary.backend_errors, 3); // cognition at ticks 0,4,8
        assert_eq!(summary.decisions_with_action, 0);
        // The experiment still completes — failure was safe.
        assert_eq!(summary.ticks_executed, 12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_seed_same_summary() {
        let dir = std::env::temp_dir().join(format!("ts-run-d-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg1 = config(BackendKind::Random { seed: 99 }, 30);
        let cfg2 = config(BackendKind::Random { seed: 99 }, 30);
        let s1 = run(&cfg1, &dir.join("a.jsonl")).unwrap();
        let s2 = run(&cfg2, &dir.join("b.jsonl")).unwrap();
        assert_eq!(s1.final_counter, s2.final_counter);
        assert_eq!(s1.decisions_with_action, s2.decisions_with_action);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn death_stops_run() {
        let dir = std::env::temp_dir().join(format!("ts-run-die-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = ExperimentConfig {
            experiment_id: ExperimentId::generate(),
            world_seed: 42,
            ticks: 10_000,
            cognition_interval: 4,
            backend: BackendKind::Echo,
            maintenance_cost: 1.0,
            start_energy: 50.0,
        };
        let summary = run(&cfg, &dir.join("death.jsonl")).unwrap();
        // 50 energy at 1.0/tick: dies at tick 50 (energy hits 0 after tick 50's drain).
        assert_eq!(summary.ticks_executed, 50);
        assert!(summary.final_energy <= 0.0);
        let records = crate::event_log::read_log(&dir.join("death.jsonl")).unwrap();
        let died = records.iter().any(|r| {
            matches!(
                &r.payload,
                EventPayload::WorldEvents { events, .. }
                    if events.iter().any(|e| matches!(e, crate::wabix::WorldEvent::SubjectDied { .. }))
            )
        });
        assert!(died, "death event must be logged");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
