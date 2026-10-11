//! Append-only experiment event log.
//!
//! Every meaningful occurrence — world transitions, actions, cognition
//! decisions, resource consumption, migration, birth, death — becomes a
//! JSONL record with a monotonic sequence number. The log is the ground
//! truth from which replay and metrics are reconstructed (M6).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cognition::Decision;
use crate::resource::ResourceEntry;
use crate::wabix::WorldEvent;

/// The experiment log schema version. Bump on incompatible changes.
pub const EVENT_LOG_VERSION: u32 = 1;

/// Discriminated union of all event payloads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    /// Experiment started; carries the manifest/config identity.
    ExperimentStarted {
        experiment_id: String,
        world_implementation: String,
        seed: u64,
    },
    /// A subject was instantiated into the world.
    SubjectCreated {
        subject_id: String,
        subject_ref: crate::wabix::SubjectRef,
    },
    /// A cognition decision was made (action may be None).
    DecisionMade {
        subject_id: String,
        tick: u64,
        decision: Decision,
    },
    /// The observation snapshot at decision time (normalized channels the
    /// subject SAW). Recorded per cognition event so post-hoc analyzers can
    /// correlate behavior with what was observed — e.g. did the subject
    /// behave differently when a chemical mark was in sensing range?
    DecisionObservation {
        subject_id: String,
        tick: u64,
        /// channel_name -> normalized value at decision time.
        observations: Vec<(String, f32)>,
    },
    /// World emitted events for one tick.
    WorldEvents { tick: u64, events: Vec<WorldEvent> },
    /// Resource consumption.
    ResourceConsumed(ResourceEntry),
    /// Subject state changed (position/energy) — sampled, not every tick.
    SubjectState {
        subject_id: String,
        tick: u64,
        pos: (u32, u32),
        energy: f32,
    },
    /// A migration event (M5): subject transitions substrate.
    Migration {
        subject_id: String,
        migration_id: String,
        dimension: String,
        from: String,
        to: String,
        tick: u64,
    },
    /// Population-level sample (primordial runner): alive count, energy
    /// totals, and the REAL field energy. Replaces the old hack of
    /// smuggling population stats through SubjectState with subject_id
    /// "POPULATION".
    PopulationSampled {
        tick: u64,
        alive: u32,
        total_energy: f32,
        mean_energy: f32,
        field_energy: f32,
    },
    /// Experiment finished.
    ExperimentFinished { reason: String },
}

/// One log record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub seq: u64,
    pub log_version: u32,
    pub payload: EventPayload,
}

/// In-memory event log with an optional JSONL sink.
pub struct EventLog {
    next_seq: u64,
    sink: Option<(PathBuf, std::fs::File)>,
    /// Bounded in-memory retention for tests/short runs.
    records: std::collections::VecDeque<EventRecord>,
    max_in_memory: usize,
}

impl EventLog {
    pub fn new(max_in_memory: usize) -> Self {
        Self {
            next_seq: 0,
            sink: None,
            records: std::collections::VecDeque::new(),
            max_in_memory,
        }
    }

    /// Attach a JSONL file sink. Parent dirs are created.
    pub fn with_sink(mut self, path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        self.sink = Some((path.to_path_buf(), file));
        Ok(self)
    }

    pub fn push(&mut self, payload: EventPayload) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        let record = EventRecord {
            seq,
            log_version: EVENT_LOG_VERSION,
            payload,
        };
        if let Some((_, file)) = self.sink.as_mut() {
            let mut line = serde_json::to_string(&record).expect("event serialization cannot fail");
            line.push('\n');
            file.write_all(line.as_bytes())
                .expect("event log write must succeed");
        }
        self.records.push_back(record);
        while self.records.len() > self.max_in_memory {
            self.records.pop_front();
        }
        seq
    }

    pub fn records(&self) -> impl Iterator<Item = &EventRecord> {
        self.records.iter()
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// Read back a JSONL event log from disk (replay path).
pub fn read_log(path: &Path) -> std::io::Result<Vec<EventRecord>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let rec: EventRecord = serde_json::from_str(line).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("line {}: {e}", i + 1),
            )
        })?;
        out.push(rec);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wabix::SubjectRef;

    #[test]
    fn records_get_sequential_seqs() {
        let mut log = EventLog::new(8);
        let a = log.push(EventPayload::ExperimentStarted {
            experiment_id: "e1".into(),
            world_implementation: "test-world-v0".into(),
            seed: 42,
        });
        let b = log.push(EventPayload::ExperimentFinished {
            reason: "done".into(),
        });
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(log.next_seq(), 2);
    }

    #[test]
    fn jsonl_round_trip() {
        let dir = std::env::temp_dir().join(format!("ts-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");

        let mut log = EventLog::new(8).with_sink(&path).unwrap();
        log.push(EventPayload::SubjectCreated {
            subject_id: "s1".into(),
            subject_ref: SubjectRef {
                subject_id: "s1".into(),
                pos: (3, 4),
                energy: 10.0,
            },
        });
        log.push(EventPayload::ExperimentFinished {
            reason: "ok".into(),
        });
        drop(log);

        let records = read_log(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].seq, 0);
        match &records[0].payload {
            EventPayload::SubjectCreated { subject_id, .. } => assert_eq!(subject_id, "s1"),
            other => panic!("unexpected payload: {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
