//! M6: Experiment manifests and metrics regeneration from event logs.
//!
//! Scientific reproducibility requirement: a completed experiment must be
//! analyzable later WITHOUT rerunning cognition. The event log is ground
//! truth; this module parses it and regenerates population metrics, action
//! distributions, and event counts.
//!
//! Replay determinism contract: the deterministic parts of an experiment
//! (world physics, non-random backends) reproduce exactly from config.
//! Random backends reproduce given the same seed. A manifest records the
//! full configuration so a run can be re-executed bit-for-bit.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::event_log::{EventPayload, EventRecord, read_log};
use crate::primordial_experiment::PopulationSample;

/// Machine-readable experiment manifest: everything needed to re-execute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentManifest {
    pub manifest_version: u32,
    pub experiment_id: String,
    /// Which experiment type produced this log: "test" | "primordial" |
    /// "migration".
    pub experiment_kind: String,
    pub world_implementation: String,
    pub seed: u64,
    /// Type-specific configuration as JSON (schema per experiment kind).
    pub config: serde_json::Value,
}

pub const MANIFEST_VERSION: u32 = 1;

impl ExperimentManifest {
    /// Serialize to pretty JSON with a trailing newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("manifest serialization cannot fail");
        s.push('\n');
        s
    }

    /// Write manifest.json next to the given events.jsonl path.
    pub fn write_next_to(&self, events_path: &Path) -> std::io::Result<()> {
        let manifest_path = events_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("manifest.json");
        std::fs::write(manifest_path, self.to_json())
    }

    /// Read a manifest from disk.
    pub fn read(path: &Path) -> std::io::Result<ExperimentManifest> {
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Metrics regenerated from an event log — no cognition re-execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegeneratedMetrics {
    pub total_records: u64,
    pub subjects_created: u64,
    pub decisions: u64,
    pub decisions_with_action: u64,
    pub decisions_no_action: u64,
    pub action_distribution: std::collections::BTreeMap<String, u64>,
    pub deaths: u64,
    pub births: u64,
    pub migrations: u64,
    pub experiments_finished: u64,
    pub finish_reason: Option<String>,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
    /// Population samples reconstructed from POPULATION SubjectState records.
    pub population_samples: Vec<PopulationSample>,
}

/// Parse a log and regenerate metrics.
pub fn regenerate_metrics(path: &Path) -> std::io::Result<RegeneratedMetrics> {
    let records = read_log(path)?;
    Ok(regenerate_from_records(&records))
}

/// Core metric regeneration from parsed records (testable without files).
pub fn regenerate_from_records(records: &[EventRecord]) -> RegeneratedMetrics {
    let mut m = RegeneratedMetrics {
        total_records: records.len() as u64,
        subjects_created: 0,
        decisions: 0,
        decisions_with_action: 0,
        decisions_no_action: 0,
        action_distribution: std::collections::BTreeMap::new(),
        deaths: 0,
        births: 0,
        migrations: 0,
        experiments_finished: 0,
        finish_reason: None,
        first_tick: None,
        last_tick: None,
        population_samples: Vec::new(),
    };

    for r in records {
        match &r.payload {
            EventPayload::SubjectCreated { subject_id, .. } => {
                m.subjects_created += 1;
                if subject_id.starts_with("org-") && !subject_id.starts_with("org-000") {
                    // Heuristic: ids beyond the initial org-0000..org-0007 block
                    // are births. The authoritative count comes from DIVIDE
                    // events; this is a cross-check path.
                }
            }
            EventPayload::DecisionMade { decision, .. } => {
                m.decisions += 1;
                match &decision.action {
                    Some(a) => {
                        m.decisions_with_action += 1;
                        *m.action_distribution.entry(a.name.clone()).or_insert(0) += 1;
                    }
                    None => m.decisions_no_action += 1,
                }
            }
            EventPayload::WorldEvents { events, .. } => {
                for e in events {
                    match e {
                        crate::wabix::WorldEvent::ActionApplied { action, .. } => {
                            if action.starts_with("DIVIDE->") {
                                m.births += 1;
                            }
                        }
                        crate::wabix::WorldEvent::SubjectDied { .. } => m.deaths += 1,
                        crate::wabix::WorldEvent::ActionRejected { .. } => {}
                    }
                }
            }
            EventPayload::SubjectState {
                subject_id,
                tick,
                pos,
                energy,
            } => {
                if subject_id == "POPULATION" {
                    let alive = pos.0;
                    let total_energy = *energy;
                    m.population_samples.push(PopulationSample {
                        tick: *tick,
                        alive,
                        total_energy,
                        mean_energy: if alive > 0 {
                            total_energy / alive as f32
                        } else {
                            0.0
                        },
                        // Field energy is not carried in SubjectState (only
                        // alive count + total organism energy); regenerated
                        // samples carry 0 here — documented limitation, use
                        // the summary JSON for field-energy time series.
                        field_energy: 0.0,
                    });
                }
                m.first_tick.get_or_insert(*tick);
                m.last_tick = Some(*tick);
            }
            EventPayload::Migration { .. } => m.migrations += 1,
            EventPayload::ExperimentFinished { reason } => {
                m.experiments_finished += 1;
                m.finish_reason = Some(reason.clone());
            }
            EventPayload::ResourceConsumed(_) => {}
            EventPayload::ExperimentStarted { .. } => {}
        }
    }

    m
}

/// Verify a log's structural integrity: seq numbers are strictly increasing
/// from 0, and every record carries the expected log version.
pub fn verify_log_integrity(path: &Path) -> std::io::Result<LogIntegrityReport> {
    let records = read_log(path)?;
    Ok(verify_records_integrity(&records))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogIntegrityReport {
    pub records: u64,
    pub seq_ok: bool,
    pub version_ok: bool,
    pub first_bad_seq: Option<u64>,
    pub log_version_seen: u32,
}

pub fn verify_records_integrity(records: &[EventRecord]) -> LogIntegrityReport {
    let mut report = LogIntegrityReport {
        records: records.len() as u64,
        seq_ok: true,
        version_ok: true,
        first_bad_seq: None,
        log_version_seen: crate::event_log::EVENT_LOG_VERSION,
    };
    for (i, r) in records.iter().enumerate() {
        if r.seq != i as u64 {
            report.seq_ok = false;
            if report.first_bad_seq.is_none() {
                report.first_bad_seq = Some(r.seq);
            }
        }
        if r.log_version != crate::event_log::EVENT_LOG_VERSION {
            report.version_ok = false;
            report.log_version_seen = r.log_version;
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ExperimentId;
    use crate::primordial_experiment::{
        BackendKind, OrganismSpec, PrimordialExperimentConfig, run_primordial,
    };

    fn prim_cfg() -> PrimordialExperimentConfig {
        PrimordialExperimentConfig {
            experiment_id: ExperimentId::generate(),
            world_seed: 42,
            ticks: 100,
            cognition_interval: 5,
            organism: OrganismSpec {
                backend: BackendKind::EnergyRule {
                    absorb_threshold: 0.3,
                },
                start_energy: 20.0,
            },
            ..Default::default()
        }
    }

    #[test]
    fn regenerated_metrics_match_summary() {
        let dir = std::env::temp_dir().join(format!("ts-replay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = prim_cfg();
        let summary = run_primordial(&cfg, &dir.join("events.jsonl")).unwrap();
        let metrics = regenerate_metrics(&dir.join("events.jsonl")).unwrap();

        // Decision and death counts must match the run summary.
        assert_eq!(metrics.deaths, summary.deaths as u64);
        assert_eq!(metrics.births, summary.births as u64);
        assert!(metrics.total_records > 0);
        // Energy-rule organisms all absorb: distribution must have ABSORB.
        assert!(metrics.action_distribution.contains_key("ABSORB"));
        assert_eq!(
            metrics
                .action_distribution
                .get("ABSORB")
                .copied()
                .unwrap_or(0),
            summary.absorb_events
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_integrity_sequential() {
        let dir = std::env::temp_dir().join(format!("ts-replay-int-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = run_primordial(&prim_cfg(), &dir.join("events.jsonl")).unwrap();
        let report = verify_log_integrity(&dir.join("events.jsonl")).unwrap();
        assert!(report.seq_ok, "seq numbers must be strictly sequential");
        assert!(report.version_ok);
        assert_eq!(report.first_bad_seq, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_seq_detected() {
        let records = vec![
            EventRecord {
                seq: 0,
                log_version: 1,
                payload: EventPayload::ExperimentFinished { reason: "x".into() },
            },
            EventRecord {
                seq: 5, // gap!
                log_version: 1,
                payload: EventPayload::ExperimentFinished { reason: "y".into() },
            },
        ];
        let report = verify_records_integrity(&records);
        assert!(!report.seq_ok);
        assert_eq!(report.first_bad_seq, Some(5));
    }

    #[test]
    fn migration_log_regenerates_migrations_count() {
        let dir = std::env::temp_dir().join(format!("ts-replay-mig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = crate::migration_experiment::MigrationExperimentConfig::new(
            BackendKind::Echo,
            BackendKind::EnergyRule {
                absorb_threshold: 0.3,
            },
        );
        let result =
            crate::migration_experiment::run_migration_experiment(&cfg, &dir.join("events.jsonl"))
                .unwrap();
        let metrics = regenerate_metrics(&dir.join("events.jsonl")).unwrap();
        assert_eq!(metrics.migrations, 1);
        assert_eq!(metrics.migrations, result.migration.iter().count() as u64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_round_trip() {
        let manifest = ExperimentManifest {
            manifest_version: MANIFEST_VERSION,
            experiment_id: "exp-1".into(),
            experiment_kind: "primordial".into(),
            world_implementation: "primordial-v0".into(),
            seed: 42,
            config: serde_json::json!({"ticks": 100, "population": 8}),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: ExperimentManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }
}
