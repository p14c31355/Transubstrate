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

/// Simple signal-seeking rule for the network world: SIGNAL when
/// "nearby_signal_0" is below threshold (to attract energy), then ABSORB
/// when the pulled energy has concentrated. Doubles as a positive control
/// for network-world experiments (mirrors EnergyRuleBackend's style).
#[derive(Clone, Debug)]
pub struct SignalSeekerBackend {
    /// nearby_signal_0 below which the rule emits SIGNAL.
    pub signal_threshold: f32,
}

impl Default for SignalSeekerBackend {
    fn default() -> Self {
        Self {
            signal_threshold: 0.1,
        }
    }
}

impl CognitionBackend for SignalSeekerBackend {
    fn implementation(&self) -> &'static str {
        "rule-signal-seeker-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let signal_seen = ctx
            .observations
            .iter()
            .find(|o| o.channel == "nearby_signal_0")
            .map(|o| o.value)
            .unwrap_or(0.0);
        // local_energy gates survival: when own energy is low, absorb
        // whatever the current world offers (works in both worlds).
        let energy_seen = ctx
            .observations
            .iter()
            .find(|o| o.channel == "local_energy")
            .map(|o| o.value)
            .unwrap_or(0.0);

        // Priority: survive (absorb when energy is low) → seed the signal
        // (when no signal is present) → harvest (when a signal is present).
        let action = if energy_seen < self.signal_threshold {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "ABSORB")
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: vec![1.0],
                })
        } else if signal_seen < self.signal_threshold {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "SIGNAL")
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: vec![1.0],
                })
        } else {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "ABSORB")
                .map(|schema| Action {
                    name: schema.name.clone(),
                    params: vec![1.0],
                })
        };

        Ok(Decision {
            action,
            rationale: format!(
                "rule(signal-seeker): local_energy={energy_seen:.3} nearby_signal_0={signal_seen:.3}"
            ),
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

/// Positive control for the chemical substrate: behaves like the energy
/// rule, but when the local energy observation is VERY high (surplus), it
/// spends surplus on RELEASE channel 1 — writing the cell's chemical
/// field. A deterministic "marker" organism: it gives the world chemical
/// marks WITHOUT any semantics attached, so later runs can ask whether
/// anyone else's behavior correlates with the marks. The rule reads and
/// writes channel names by design (rule-based controllers are named-channel
/// machines, not subjects under study).
#[derive(Clone, Debug)]
pub struct ChemMarkerBackend {
    /// local_energy above which the rule absorbs (like EnergyRuleBackend).
    pub absorb_threshold: f32,
    /// local_energy above which the rule releases into the chemical field.
    pub mark_threshold: f32,
}

impl Default for ChemMarkerBackend {
    fn default() -> Self {
        Self {
            absorb_threshold: 0.3,
            // Mark only in surplus: above 0.8 the organism has >= 8.0 raw
            // energy, enough to pay for a mark and keep metabolizing.
            mark_threshold: 0.8,
        }
    }
}

impl CognitionBackend for ChemMarkerBackend {
    fn implementation(&self) -> &'static str {
        "rule-chem-marker-v0"
    }

    fn decide(&mut self, ctx: &DecisionContext) -> Result<Decision, CognitionError> {
        let energy_seen = ctx
            .observations
            .iter()
            .find(|o| o.channel == "local_energy")
            .map(|o| o.value)
            .unwrap_or(0.0);

        let action = if energy_seen > self.mark_threshold {
            ctx.manifest
                .actions
                .iter()
                .find(|a| a.name == "RELEASE" && a.arity >= 2)
                .map(|schema| Action {
                    name: schema.name.clone(),
                    // params: [amount, channel] — channel 1 = chemical.
                    params: vec![0.5, 1.0],
                })
        } else if energy_seen > self.absorb_threshold {
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
            rationale: format!("chem-marker rule: local_energy={energy_seen:.3}"),
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

/// Mobile variant of the reproducing rule: behaves identically to
/// ReproducingRuleBackend while the cell feeds it, but after `starve_patience`
/// consecutive unproductive decisions (WAIT on a drained cell) it takes a
/// RANDOM step to an adjacent cell. This is the minimal movement heuristic
/// that requires NO extra observation channels: the subject only knows its
/// own intake history. Tests the long-run-stability hypothesis that a
/// stationary population cannot convert field surplus into growth, while a
/// mobile one can (the world's MOVE action needs a direction param; the
/// world resolves it; see primordial MOVE handling).
#[derive(Clone, Debug)]
pub struct MobileReproducingRuleBackend {
    /// local_energy observation above which the rule absorbs.
    pub absorb_threshold: f32,
    /// local_energy observation above which the rule proposes DIVIDE.
    pub divide_probe: f32,
    /// Number of remembered ABSORB outcomes that must all be unproductive
    /// (reward <= 0) before the rule takes a step.
    pub starve_patience: u32,
}

impl MobileReproducingRuleBackend {
    pub fn new(absorb_threshold: f32, divide_probe: f32, starve_patience: u32) -> Self {
        Self {
            absorb_threshold,
            divide_probe,
            starve_patience,
        }
    }
}

impl Default for MobileReproducingRuleBackend {
    fn default() -> Self {
        Self::new(0.3, 0.4, 3)
    }
}

impl CognitionBackend for MobileReproducingRuleBackend {
    fn implementation(&self) -> &'static str {
        "rule-energy-divide-move-v0"
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

        // Starvation is judged from ABSORB ATTEMPT OUTCOMES: the cell is
        // "dead" when the subject recently TRIED to absorb at least twice
        // and the attempt rewards sum to nothing. Two earlier designs fail:
        // energy-level tests never fire (absorbs are only proposed when the
        // cell holds regen, so absorb memories are successes), and
        // all-decision reward sums ALWAYS fire at equilibrium — DIVIDE
        // proposals get rejected because the GENOME threshold is not met,
        // which says nothing about cell quality, so the subject would
        // wander forever paying move costs (observed: total population
        // crash in every regen regime). Absorb attempts are the only
        // decision type whose outcome tracks the CELL.
        let recent_absorbs: Vec<&crate::cognition::MemoryEntry> = ctx
            .memory
            .iter()
            .rev()
            .filter(|m| m.action.name == "ABSORB")
            .take(self.starve_patience as usize)
            .collect();
        let cell_dead = recent_absorbs.len() >= 2
            && recent_absorbs.iter().map(|m| m.reward_signal).sum::<f32>() <= 0.0;

        let (action, why) = if cell_dead {
            // MOVE params: (dx, dy) each 0..1 mapped to -1..=1 steps.
            // Cycle deterministically through the 4 directions from
            // the tick (the context carries no RNG).
            let step = (ctx.tick / self.starve_patience.max(1) as u64) % 4;
            let (dx, dy) = match step {
                0 => (1.0, 0.5), // +x
                1 => (0.0, 1.0), // +y
                2 => (1.0, 1.0), // +x +y
                _ => (0.5, 0.0), // -x
            };
            (pick(ctx, "MOVE", vec![dx, dy]), "move (cell dead)")
        } else if energy_seen >= self.divide_probe {
            (pick(ctx, "DIVIDE", vec![]), "divide")
        } else if energy_seen > self.absorb_threshold {
            (pick(ctx, "ABSORB", vec![1.0]), "absorb")
        } else {
            (pick(ctx, "WAIT", vec![]), "wait")
        };

        Ok(Decision {
            action,
            rationale: format!("rule(repro+move): local_energy={energy_seen:.3} -> {why}"),
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
