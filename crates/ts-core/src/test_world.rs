//! A deterministic test world: fixed observation, one stateful counter,
//! minimal action set. Exists to exercise the full runtime loop without any
//! simulation complexity — the "Hello, World" of WABIX.

use crate::rng::Rng;
use crate::wabix::{
    Action, CapabilityManifest, ManifestBuilder, Observation, SubjectRef, WabixError, World,
    WorldEvent, validate_action,
};

/// TestWorld: exposes `counter` observations and accepts `BUMP` (adds a
/// param-controlled amount to the counter) and `WAIT`.
///
/// Determinism contract: identical action sequences produce identical state
/// trajectories. The RNG field exists to prove seeded worlds stay deterministic
/// (it only generates an observation jitter driven by the seed).
pub struct TestWorld {
    manifest: CapabilityManifest,
    counters: std::collections::BTreeMap<String, f32>,
    rng: Rng,
    /// Cumulative applied actions (for assertions).
    pub applied: Vec<String>,
}

impl TestWorld {
    pub fn new(seed: u64) -> Self {
        Self {
            manifest: ManifestBuilder::new("test-world-v0")
                .channel("counter", "scalar", (0.0, 1_000_000.0))
                .channel("jitter", "scalar", (0.0, 1.0))
                .action("WAIT", 0, vec![])
                .action("BUMP", 1, vec![(0.0, 100.0)])
                .build(),
            counters: std::collections::BTreeMap::new(),
            rng: Rng::new(seed),
            applied: Vec::new(),
        }
    }

    pub fn counter(&self, subject: &str) -> f32 {
        self.counters.get(subject).copied().unwrap_or(0.0)
    }
}

impl World for TestWorld {
    fn implementation(&self) -> &'static str {
        "test-world-v0"
    }

    fn manifest(&self) -> &CapabilityManifest {
        &self.manifest
    }

    fn observe(&self, subject: &SubjectRef) -> Result<Vec<Observation>, WabixError> {
        let counter = self
            .counters
            .get(&subject.subject_id)
            .copied()
            .unwrap_or(0.0);
        Ok(vec![
            Observation::new("counter", counter),
            Observation::new("jitter", (self.rng_state_f32() * 0.001) + 0.5),
        ])
    }

    fn validate(&self, action: &Action) -> Result<(), WabixError> {
        validate_action(&self.manifest, action)
    }

    fn step(&mut self, subject: &SubjectRef, action: Option<&Action>) -> Vec<WorldEvent> {
        let mut events = Vec::new();
        if let Some(a) = action {
            // Execution-time validation (defense in depth).
            if let Err(e) = self.validate(a) {
                events.push(WorldEvent::ActionRejected {
                    subject: subject.subject_id.clone(),
                    action: a.name.clone(),
                    reason: e.to_string(),
                });
                return events;
            }
            match a.name.as_str() {
                "BUMP" => {
                    let entry = self
                        .counters
                        .entry(subject.subject_id.clone())
                        .or_insert(0.0);
                    *entry += a.params[0];
                    self.applied.push(format!("BUMP({})", a.params[0]));
                    events.push(WorldEvent::ActionApplied {
                        subject: subject.subject_id.clone(),
                        action: "BUMP".into(),
                        cost: 0.0,
                    });
                }
                "WAIT" => {
                    self.applied.push("WAIT".into());
                    events.push(WorldEvent::ActionApplied {
                        subject: subject.subject_id.clone(),
                        action: "WAIT".into(),
                        cost: 0.0,
                    });
                }
                other => events.push(WorldEvent::ActionRejected {
                    subject: subject.subject_id.clone(),
                    action: other.into(),
                    reason: "not a step-implemented action".into(),
                }),
            }
        }
        events
    }
}

impl TestWorld {
    fn rng_state_f32(&self) -> f32 {
        // Expose a deterministic derived value without mutating RNG state.
        (self.rng_seed_mirror() >> 11) as f32 / (1u64 << 53) as f32
    }

    fn rng_seed_mirror(&self) -> u64 {
        // The RNG is only used through observe()'s jitter, which must be a
        // pure function of world state for determinism, so we keep a mirror.
        self.rng.peek_state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_changes_counter_deterministically() {
        let mut w1 = TestWorld::new(7);
        let mut w2 = TestWorld::new(7);
        let s = SubjectRef {
            subject_id: "s1".into(),
            pos: (0, 0),
            energy: 100.0,
        };
        let bump = Action {
            name: "BUMP".into(),
            params: vec![2.5],
        };
        w1.step(&s, Some(&bump));
        w1.step(&s, Some(&bump));
        w2.step(&s, Some(&bump));
        w2.step(&s, Some(&bump));
        assert_eq!(w1.counter("s1"), 5.0);
        assert_eq!(w1.counter("s1"), w2.counter("s1"));
        assert_eq!(w1.applied, w2.applied);
    }

    #[test]
    fn invalid_action_rejected_at_execution() {
        let mut w = TestWorld::new(1);
        let s = SubjectRef {
            subject_id: "s1".into(),
            pos: (0, 0),
            energy: 100.0,
        };
        let events = w.step(
            &s,
            Some(&Action {
                name: "TELEPORT".into(),
                params: vec![],
            }),
        );
        match &events[0] {
            WorldEvent::ActionRejected { action, .. } => assert_eq!(action, "TELEPORT"),
            other => panic!("expected rejection, got {other:?}"),
        }
    }
}
