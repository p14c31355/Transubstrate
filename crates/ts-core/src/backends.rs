//! Mock, random, and rule-based cognition backends for tests and the first
//! vertical slice. These prove the backend ABI without any model download.

use serde::{Deserialize, Serialize};

use crate::cognition::{
    CognitionBackend, CognitionCost, CognitionError, Decision, DecisionContext,
};
use crate::rng::Rng;
use crate::wabix::Action;

/// Deterministic echo backend: always takes the first available action with
/// zeroed params (or no-op if the world has no actions). Useful as a fixed
/// point for migration baselines (M5).
#[derive(Clone, Debug, Default)]
pub struct EchoBackend;

impl CognitionBackend for EchoBackend {
    fn implementation(&self) -> &'static str {
        "mock-echo-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let action = ctx.manifest.actions.first().map(|schema| Action {
            name: schema.name.clone(),
            params: vec![0.0; schema.arity as usize],
        });
        Ok(Decision {
            action,
            rationale: "echo: first capability, zero params".into(),
        })
    }
}

/// Seeded random backend: picks a uniformly random valid action each event.
/// Deterministic given the seed; documents its randomness policy as required.
#[derive(Clone, Debug)]
pub struct RandomBackend {
    rng: Rng,
    /// Whether zero-parameter actions are allowed to be chosen (they are
    /// trivially valid).
    allow_noop: bool,
}

impl RandomBackend {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            allow_noop: true,
        }
    }
}

impl CognitionBackend for RandomBackend {
    fn implementation(&self) -> &'static str {
        "mock-random-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let actions = &ctx.manifest.actions;
        if actions.is_empty() {
            return Ok(Decision {
                action: None,
                rationale: "world offers no actions".into(),
            });
        }
        let idx = self.rng.next_range(actions.len() as u64) as usize;
        let schema = &actions[idx];
        if schema.arity == 0 {
            return Ok(Decision {
                action: Some(Action {
                    name: schema.name.clone(),
                    params: vec![],
                }),
                rationale: format!("random: chose {}", schema.name),
            });
        }
        if !self.allow_noop {
            // Still must return a valid action; sample mid-range params.
            let params = schema
                .param_ranges
                .iter()
                .map(|(lo, hi)| lo + (hi - lo) * self.rng.next_f32())
                .collect();
            return Ok(Decision {
                action: Some(Action {
                    name: schema.name.clone(),
                    params,
                }),
                rationale: format!("random: chose {} (mid-range)", schema.name),
            });
        }
        let params = schema
            .param_ranges
            .iter()
            .map(|(lo, hi)| lo + (hi - lo) * self.rng.next_f32())
            .collect();
        Ok(Decision {
            action: Some(Action {
                name: schema.name.clone(),
                params,
            }),
            rationale: format!("random: chose {}", schema.name),
        })
    }
}

/// Simple energy-seeking rule: if a "local_energy" channel observation is
/// available and above threshold, ABSORB; otherwise WAIT. Written against
/// channel names by design — it is a *rule-based* controller, and doubles as
/// a positive control for primordial-world experiments.
#[derive(Clone, Debug, Default)]
pub struct EnergyRuleBackend {
    /// Observation value above which the rule absorbs.
    pub absorb_threshold: f32,
}

impl CognitionBackend for EnergyRuleBackend {
    fn implementation(&self) -> &'static str {
        "rule-energy-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        // Positive control semantics: absorb when local energy is present.
        let energy_seen = ctx
            .observations
            .iter()
            .find(|o| o.channel == "local_energy")
            .map(|o| o.value)
            .unwrap_or(0.0);

        let action = if energy_seen > self.absorb_threshold {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "ABSORB")
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: vec![1.0],
                })
        } else {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "WAIT")
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: vec![],
                })
        };

        Ok(Decision {
            action,
            rationale: format!("rule: local_energy={energy_seen:.3}"),
        })
    }
}

/// Reproducing variant of the energy rule: ABSORB when energy channel is
/// high, DIVIDE when the (normalized) energy store is very high. The world
/// enforces the real division threshold; this rule just proposes DIVIDE at
/// a normalized probe point, giving the genome control over timing.
#[derive(Clone, Debug)]
pub struct ReproducingRuleBackend {
    /// local_energy observation above which the rule absorbs.
    pub absorb_threshold: f32,
    /// local_energy observation above which the rule proposes DIVIDE.
    pub divide_probe: f32,
}

impl Default for ReproducingRuleBackend {
    fn default() -> Self {
        Self {
            absorb_threshold: 0.3,
            // local_energy is organism energy / 10 clamped to 1.0; 3.0 raw
            // energy maps to 0.3. Default probe 0.4 = ~4.0 raw energy, below
            // the default division threshold (40), so world genomes govern.
            divide_probe: 0.4,
        }
    }
}

impl CognitionBackend for ReproducingRuleBackend {
    fn implementation(&self) -> &'static str {
        "rule-energy-divide-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let energy_seen = ctx
            .observations
            .iter()
            .find(|o| o.channel == "local_energy")
            .map(|o| o.value)
            .unwrap_or(0.0);

        let pick = |ctx: &DecisionContext, name: &str, params: Vec<f32>| {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == name)
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: if schema.arity == 0 { vec![] } else { params },
                })
        };

        let (action, why) = if energy_seen >= self.divide_probe {
            (pick(ctx, "DIVIDE", vec![]), "divide")
        } else if energy_seen > self.absorb_threshold {
            (pick(ctx, "ABSORB", vec![1.0]), "absorb")
        } else {
            (pick(ctx, "WAIT", vec![]), "wait")
        };

        Ok(Decision {
            action,
            rationale: format!("rule(repro): local_energy={energy_seen:.3} -> {why}"),
        })
    }
}

/// A deliberately broken backend used to test safe failure paths: it returns
/// malformed/invalid decisions so the runtime must reject them without
/// corrupting state.
#[derive(Clone, Debug, Default)]
pub struct MalformedBackend {
    /// Which failure mode to emit.
    pub mode: MalformedMode,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum MalformedMode {
    /// Propose an action not in the manifest.
    #[default]
    UnknownAction,
    /// Propose a valid action name with out-of-range params.
    OutOfRange,
}

impl CognitionBackend for MalformedBackend {
    fn implementation(&self) -> &'static str {
        "mock-malformed-v0"
    }

    fn decide(&mut self, _ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let action = match self.mode {
            MalformedMode::UnknownAction => Action {
                name: "TELEPORT_TO_THE_MOON".into(),
                params: vec![],
            },
            MalformedMode::OutOfRange => Action {
                name: "ABSORB".into(),
                params: vec![-100.0],
            },
        };
        Ok(Decision {
            action: Some(action),
            rationale: "malformed by design".into(),
        })
    }
}

/// Cost profile for mock backends: zero tokens, one cognition event.
pub fn mock_cost() -> CognitionCost {
    CognitionCost {
        cognition_events: 1,
        tokens: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::validate_decision;
    use crate::wabix::ManifestBuilder;

    fn ctx(manifest: crate::wabix::CapabilityManifest) -> DecisionContext {
        DecisionContext {
            subject_id: "s-test".into(),
            tick: 0,
            energy: 10.0,
            observations: vec![crate::wabix::Observation::new("local_energy", 0.9)],
            manifest,
            memory: vec![],
        }
    }

    #[test]
    fn echo_is_deterministic() {
        let m = ManifestBuilder::new("w")
            .action("WAIT", 0, vec![])
            .action_with_cost("ABSORB", 1, vec![(0.0, 5.0)], 0.0)
            .build();
        let mut b = EchoBackend;
        let d1 = b.decide(&ctx(m.clone())).unwrap();
        let d2 = b.decide(&ctx(m)).unwrap();
        assert_eq!(d1, d2);
        assert_eq!(d1.action.unwrap().name, "WAIT");
    }

    #[test]
    fn random_respects_seed_and_schema() {
        let m = ManifestBuilder::new("w")
            .action("WAIT", 0, vec![])
            .action_with_cost("ABSORB", 1, vec![(0.0, 5.0)], 0.0)
            .build();
        let mut a = RandomBackend::new(42);
        let mut b = RandomBackend::new(42);
        for tick in 0..50 {
            let da = a.decide(&ctx(m.clone())).unwrap();
            let db = b.decide(&ctx(m.clone())).unwrap();
            assert_eq!(da, db, "seeded random must be reproducible");
            validate_decision(&m, &da).expect("random backend must respect schema");
            let _ = tick;
        }
    }

    #[test]
    fn energy_rule_absorbs_when_energy_present() {
        let m = ManifestBuilder::new("w")
            .channel("local_energy", "scalar", (0.0, 1.0))
            .action("WAIT", 0, vec![])
            .action_with_cost("ABSORB", 1, vec![(0.0, 1.0)], 0.0)
            .build();
        let mut b = EnergyRuleBackend {
            absorb_threshold: 0.5,
        };
        let d = b.decide(&ctx(m)).unwrap();
        assert_eq!(d.action.unwrap().name, "ABSORB");
    }

    #[test]
    fn malformed_backend_produces_schema_violation() {
        let m = ManifestBuilder::new("w")
            .action_with_cost("ABSORB", 1, vec![(0.0, 1.0)], 0.0)
            .build();
        let mut b = MalformedBackend::default();
        let d = b.decide(&ctx(m.clone())).unwrap();
        assert!(validate_decision(&m, &d).is_err());
    }
}
