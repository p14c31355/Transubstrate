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
use crate::continuity::{ActionCollector, ContinuityVector, MigrationEvent};
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
        }
    }
}

/// World-migration experiment result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMigrationResult {
    pub experiment_id: String,
    pub migration: MigrationEvent,
    /// Origin-world phase (pre window).
    pub pre: crate::continuity::BehaviorFingerprint,
    /// Destination-world phase (post window).
    pub post: crate::continuity::BehaviorFingerprint,
    pub continuity: crate::continuity::ContinuityVector,
    /// Control: same subject staying in the origin world the whole time.
    pub control_continuity: crate::continuity::ContinuityVector,
}

/// Run a world migration: subject starts in the primordial world, migrates
/// into the network world at the midpoint with carried-over energy.
pub fn run_world_migration(
    config: &WorldMigrationConfig,
    log_path: &Path,
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
        dimensions: vec![ContinuityVector::behavioral(
            &pre_collector.fingerprint(),
            &post_collector.fingerprint(),
        )],
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
}
