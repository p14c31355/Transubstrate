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

use crate::cognition::{CognitionBackend, CognitionError, DecisionContext};
use crate::continuity::{ActionCollector, ContinuityVector, MigrationEvent};
use crate::event_log::{EventLog, EventPayload};
use crate::ids::ExperimentId;
use crate::network_world::{NetworkConfig, NetworkWorld, NETWORK_WORLD_IMPLEMENTATION};
use crate::primordial::{PrimordialConfig, PrimordialWorld, WORLD_IMPLEMENTATION};
use crate::resource::ResourceLedger;
use crate::runtime::CognitionCadence;
use crate::wabix::{SubjectRef, World};

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
    let mut collector_pre = ActionCollector::new();
    let mut collector_post = ActionCollector::new();

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

    let mut pre_collector = ActionCollector::new();
    for local in 0..half {
        let tick = local;
        let mut actions: Vec<(String, Action)> = Vec::new();
        if cadence.fires_on(tick) {
            if let Some(org) = origin.organism(&sid_str) {
                sref.pos = org.pos;
                sref.energy = org.energy;
                let observations = origin.observe(&sref).map_err(|e| {
                    CognitionError::Unavailable(format!("observe failed: {e}"))
                })?;
                let ctx = DecisionContext {
                    subject_id: sid_str.clone(),
                    tick,
                    energy: sref.energy,
                    observations,
                    manifest: origin.manifest().clone(),
                    memory: vec![],
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
                    actions.push((sid_str.clone(), action));
                } else {
                    pre_collector.record(None);
                }
            }
        }
        let world_events = origin.tick(&actions);
        log.push(EventPayload::WorldEvents { tick, events: world_events });
        if let Some(org) = origin.organism(&sid_str) {
            sref.pos = org.pos;
            sref.energy = org.energy;
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
    let node = (config.world_seed % config.network.nodes.max(1)) as u32;
    network.admit_subject(&sid_str, node, carried_energy);

    let mut post_collector = ActionCollector::new();
    for local in 0..(config.ticks - half) {
        let tick = half + local;
        let mut actions: Vec<(String, Action)> = Vec::new();
        if cadence.fires_on(tick) {
            if let Some(sub) = network.subject(&sid_str) {
                let sref = SubjectRef {
                    subject_id: sid_str.clone(),
                    pos: (sub.node, 0),
                    energy: sub.energy,
                };
                let observations = network
                    .observe(&sref)
                    .map_err(|e| CognitionError::Unavailable(format!("observe failed: {e}")))?;
                let ctx = DecisionContext {
                    subject_id: sid_str.clone(),
                    tick,
                    energy: sub.energy,
                    observations,
                    manifest: network.manifest().clone(),
                    memory: vec![],
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
                    actions.push((sid_str.clone(), action));
                } else {
                    post_collector.record(None);
                }
            }
        }
        let world_events = network.tick(&actions);
        log.push(EventPayload::WorldEvents { tick, events: world_events });
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
