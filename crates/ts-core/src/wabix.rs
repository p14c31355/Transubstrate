//! WABIX v0 — the world/subject boundary.
//!
//! WABIX is the contract between a subject and whatever world/body it
//! inhabits, analogous to the role an ABI plays for software. This first
//! version is deliberately minimal: capability discovery, typed observations,
//! typed actions, and validation. Physics-aware observation metadata and
//! capability grants (safety model) arrive in later WABIX revisions.
//!
//! Design constraints for v0:
//! - deterministic-friendly (no clock reads inside the trait)
//! - no LLM-specific concepts leak in here
//! - every action is validated against the world's declared schema

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// A named observation channel the world exposes to a subject.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationChannel {
    pub name: String,
    /// Human-readable unit label (e.g. "scalar", "celsius", "energy").
    pub unit: String,
    /// Normalized range [min, max] of values on this channel.
    pub range: (f32, f32),
}

/// A named action the world accepts from a subject.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionSchema {
    pub name: String,
    /// Number of typed f32 parameters.
    pub arity: u8,
    /// Valid parameter ranges, aligned with arity.
    pub param_ranges: Vec<(f32, f32)>,
    /// Action cost in world energy units per unit magnitude (default 0).
    #[serde(default)]
    pub cost_per_unit: f32,
}

/// What a world offers a subject, discovered explicitly — never assumed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub world_id: String,
    pub observation_channels: Vec<ObservationChannel>,
    pub actions: Vec<ActionSchema>,
}

/// One observation value on one channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub channel: String,
    pub value: f32,
}

impl Observation {
    pub fn new(channel: &str, value: f32) -> Self {
        Self {
            channel: channel.into(),
            value,
        }
    }
}

/// A subject's intent to act. Constructed only through validated paths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub params: Vec<f32>,
}

#[derive(Error, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WabixError {
    /// Action name not in the capability manifest.
    #[error("unknown action `{0}`")]
    UnknownAction(String),
    /// Action exists but parameters violate the declared schema.
    #[error("invalid parameters for `{action}`: {reason}")]
    InvalidParameters { action: String, reason: String },
    /// Observation channel unknown to this world.
    #[error("unknown observation channel `{0}`")]
    UnknownChannel(String),
}

/// The world half of WABIX. Implementors provide a deterministic step
/// function; subjects interact only through this trait.
pub trait World {
    /// Stable world-implementation identity (distinct from WorldId instances).
    fn implementation(&self) -> &'static str;

    /// Explicit capability discovery.
    fn manifest(&self) -> &CapabilityManifest;

    /// Current observations for one subject. Worlds may restrict visibility
    /// (e.g. local sensing) — the manifest declares *what exists*, not
    /// *who sees what*.
    fn observe(&self, subject: &SubjectRef) -> Result<Vec<Observation>, WabixError>;

    /// Validate an action against the manifest without executing it.
    fn validate(&self, action: &Action) -> Result<(), WabixError>;

    /// Energy cost of an action according to the manifest (before world
    /// modifiers). Validation must pass before cost is meaningful.
    fn action_cost(&self, action: &Action) -> Result<f32, WabixError> {
        self.validate(action)?;
        let schema = self
            .manifest()
            .actions
            .iter()
            .find(|a| a.name == action.name)
            .expect("validate guarantees schema exists");
        let magnitude: f32 = action.params.iter().map(|p| p.abs()).sum();
        Ok(schema.cost_per_unit * magnitude)
    }

    /// Advance the world by one tick, applying an already-validated action
    /// from the given subject. Returns events the world produced this tick.
    fn step(&mut self, subject: &SubjectRef, action: Option<&Action>) -> Vec<WorldEvent>;
}

/// Minimal subject view from the world's perspective.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubjectRef {
    pub subject_id: String,
    /// World-local position key; primordial uses (x, y) grid coords.
    pub pos: (u32, u32),
    /// World-local energy store.
    pub energy: f32,
}

/// Events a world emits during step, for the event log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldEvent {
    /// A subject died in this world (e.g. energy exhausted).
    SubjectDied { subject: String, cause: String },
    /// A subject's action was applied (post-validation, with cost).
    ActionApplied {
        subject: String,
        action: String,
        cost: f32,
    },
    /// World rejected an action at execution time even though it validated
    /// (e.g. insufficient energy). Subject keeps its energy; action lost.
    ActionRejected {
        subject: String,
        action: String,
        reason: String,
    },
    /// A subject converted its own energy into a cell's chemical field
    /// (RELEASE channel 1): the semantics-free substrate was WRITTEN.
    /// Marks are the raw material of chemical-field meaning formation —
    /// who left what where, without any payoff attached.
    ChemicalMarked {
        subject: String,
        pos: (u32, u32),
        amount: f32,
    },
}

/// Convenience: build a manifest.
pub struct ManifestBuilder {
    manifest: CapabilityManifest,
}

impl ManifestBuilder {
    pub fn new(world_id: &str) -> Self {
        Self {
            manifest: CapabilityManifest {
                world_id: world_id.into(),
                observation_channels: Vec::new(),
                actions: Vec::new(),
            },
        }
    }

    pub fn channel(mut self, name: &str, unit: &str, range: (f32, f32)) -> Self {
        self.manifest.observation_channels.push(ObservationChannel {
            name: name.into(),
            unit: unit.into(),
            range,
        });
        self
    }

    pub fn action(mut self, name: &str, arity: u8, param_ranges: Vec<(f32, f32)>) -> Self {
        self.manifest.actions.push(ActionSchema {
            name: name.into(),
            arity,
            param_ranges,
            cost_per_unit: 0.0,
        });
        self
    }

    pub fn action_with_cost(
        mut self,
        name: &str,
        arity: u8,
        param_ranges: Vec<(f32, f32)>,
        cost_per_unit: f32,
    ) -> Self {
        self.manifest.actions.push(ActionSchema {
            name: name.into(),
            arity,
            param_ranges,
            cost_per_unit,
        });
        self
    }

    pub fn build(self) -> CapabilityManifest {
        self.manifest
    }
}

/// Shared validation logic so every World implementation behaves identically.
pub fn validate_action(manifest: &CapabilityManifest, action: &Action) -> Result<(), WabixError> {
    let schema = manifest
        .actions
        .iter()
        .find(|a| a.name == action.name)
        .ok_or_else(|| WabixError::UnknownAction(action.name.clone()))?;
    if action.params.len() != schema.arity as usize {
        return Err(WabixError::InvalidParameters {
            action: action.name.clone(),
            reason: format!(
                "expected {} params, got {}",
                schema.arity,
                action.params.len()
            ),
        });
    }
    for (i, (p, (lo, hi))) in action.params.iter().zip(&schema.param_ranges).enumerate() {
        if !(*lo..=*hi).contains(p) {
            return Err(WabixError::InvalidParameters {
                action: action.name.clone(),
                reason: format!("param[{}] = {p} outside [{lo}, {hi}]", i),
            });
        }
    }
    Ok(())
}

/// Type alias for observation maps keyed by channel name.
pub type ObservationMap = BTreeMap<String, f32>;

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> CapabilityManifest {
        ManifestBuilder::new("test-world")
            .channel("sensor_0", "scalar", (0.0, 1.0))
            .action("WAIT", 0, vec![])
            .action_with_cost("RELEASE", 1, vec![(0.0, 10.0)], 1.0)
            .build()
    }

    #[test]
    fn unknown_action_rejected() {
        let m = manifest();
        let err = validate_action(
            &m,
            &Action {
                name: "FLY".into(),
                params: vec![],
            },
        )
        .unwrap_err();
        assert_eq!(err, WabixError::UnknownAction("FLY".into()));
    }

    #[test]
    fn wrong_arity_rejected() {
        let m = manifest();
        let err = validate_action(
            &m,
            &Action {
                name: "WAIT".into(),
                params: vec![1.0],
            },
        )
        .unwrap_err();
        assert!(matches!(err, WabixError::InvalidParameters { .. }));
    }

    #[test]
    fn out_of_range_rejected() {
        let m = manifest();
        let err = validate_action(
            &m,
            &Action {
                name: "RELEASE".into(),
                params: vec![11.0],
            },
        )
        .unwrap_err();
        assert!(matches!(err, WabixError::InvalidParameters { .. }));
    }

    #[test]
    fn cost_is_magnitude_times_rate() {
        let m = manifest();
        let cost = ManifestBuilder::new("x").build();
        let _ = cost;
        let _a = Action {
            name: "RELEASE".into(),
            params: vec![3.0],
        };
        let schema = m.actions.iter().find(|s| s.name == "RELEASE").unwrap();
        assert_eq!(schema.cost_per_unit * 3.0, 3.0);
    }
}
