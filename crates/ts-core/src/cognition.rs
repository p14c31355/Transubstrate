//! Cognition backend ABI.
//!
//! A cognition backend is a replaceable organ. The subject runtime owns
//! persistent state, resources, and identity; the backend only maps bounded
//! context into bounded decisions. Nothing model-specific leaks here.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::resource::ResourceKind;
use crate::wabix::{Action, CapabilityManifest, Observation, WabixError};

/// Bounded context handed to a backend for one decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Stable subject name (administrative identity only).
    pub subject_id: String,
    /// World tick at decision time.
    pub tick: u64,
    /// Current energy (world units).
    pub energy: f32,
    /// Current observations by channel name.
    pub observations: Vec<Observation>,
    /// What actions exist in this world (so decisions respect the manifest).
    pub manifest: CapabilityManifest,
    /// Backend-private memory: prior decisions of this subject (bounded).
    pub memory: Vec<MemoryEntry>,
}

/// One remembered past decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub tick: u64,
    pub action: Action,
    pub reward_signal: f32,
}

/// Backend output: a typed decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub action: Option<Action>,
    /// What the backend chose and why, for the event log (bounded length,
    /// no free-form model text reaches execution).
    pub rationale: String,
}

/// Errors a backend may produce. All fail safely: the runtime treats a
/// backend error as "no action this cognition event" plus a logged event.
#[derive(Error, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CognitionError {
    #[error("backend produced malformed decision: {0}")]
    MalformedDecision(String),
    #[error("backend proposed action violating WABIX schema: {0}")]
    SchemaViolation(String),
    #[error("backend unavailable: {0}")]
    Unavailable(String),
}

/// The cognition backend trait. Implementations: mock, random, rule-based,
/// tiny numerical, local quantized LLM, remote OpenAI-compatible.
pub trait CognitionBackend {
    /// Stable backend implementation identity (e.g. "mock-echo-v0").
    fn implementation(&self) -> &'static str;

    /// One bounded decision. Implementations must be deterministic under
    /// the same context unless they document their randomness policy.
    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError>;
}

/// Consumes resources for one cognition event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitionCost {
    pub cognition_events: u64,
    pub tokens: u64,
}

impl CognitionCost {
    pub fn free() -> Self {
        Self {
            cognition_events: 1,
            tokens: 0,
        }
    }
}

/// Record a backend's cost into a ledger.
pub fn charge_cognition(
    ledger: &mut crate::resource::ResourceLedger,
    tick: u64,
    cost: &CognitionCost,
    consumer: &str,
) {
    ledger.record(crate::resource::ResourceEntry {
        tick,
        kind: ResourceKind::CognitionEvents,
        amount: cost.cognition_events,
        consumer: consumer.into(),
    });
    if cost.tokens > 0 {
        ledger.record(crate::resource::ResourceEntry {
            tick,
            kind: ResourceKind::Tokens,
            amount: cost.tokens,
            consumer: consumer.into(),
        });
    }
}

/// Helper: validate a backend-proposed action against a manifest, converting
/// schema violations into a typed error rather than execution-time surprises.
pub fn validate_decision(
    manifest: &CapabilityManifest,
    decision: &Decision,
) -> Result<(), CognitionError> {
    match &decision.action {
        None => Ok(()),
        Some(a) => crate::wabix::validate_action(manifest, a)
            .map_err(|e: WabixError| CognitionError::SchemaViolation(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wabix::ManifestBuilder;

    #[test]
    fn schema_violation_is_typed_error() {
        let m = ManifestBuilder::new("w").action("WAIT", 0, vec![]).build();
        let d = Decision {
            action: Some(Action {
                name: "TELEPORT".into(),
                params: vec![],
            }),
            rationale: "why not".into(),
        };
        let err = validate_decision(&m, &d).unwrap_err();
        assert!(matches!(err, CognitionError::SchemaViolation(_)));
    }

    #[test]
    fn no_action_is_always_valid() {
        let m = ManifestBuilder::new("w").build();
        let d = Decision {
            action: None,
            rationale: "rest".into(),
        };
        assert!(validate_decision(&m, &d).is_ok());
    }
}
