//! The subject runtime: owns identity, state, resources, and cognition
//! cadence. Subjects are long-lived across backend/world/body replacement;
//! the runtime is where continuity is administered (and later, measured).

use serde::{Deserialize, Serialize};

use crate::cognition::{
    CognitionBackend, CognitionError, Decision, DecisionContext, charge_cognition,
    validate_decision,
};
use crate::event_log::{EventLog, EventPayload};
use crate::ids::{CognitionBackendId, IncarnationId, SubjectId};
use crate::resource::ResourceLedger;
use crate::wabix::{Action, SubjectRef, WabixError, World};

/// How often cognition fires relative to world ticks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitionCadence {
    /// One cognition event every `interval` ticks (tick % interval == 0).
    pub interval: u64,
}

impl CognitionCadence {
    pub fn every(ticks: u64) -> Self {
        assert!(ticks > 0, "cadence interval must be >= 1");
        Self { interval: ticks }
    }

    pub fn fires_on(&self, tick: u64) -> bool {
        tick.is_multiple_of(self.interval)
    }
}

/// Errors from driving a subject through the runtime.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Wabix(#[from] WabixError),
    #[error(transparent)]
    Cognition(#[from] CognitionError),
}

/// The runtime-side subject: identity + bounded memory + resource state.
pub struct Subject {
    pub id: SubjectId,
    pub incarnation: IncarnationId,
    /// Backend *instance* currently powering this subject. Changing this is
    /// a migration (M5), not a new subject.
    pub backend_id: CognitionBackendId,
    /// Bounded memory of past decisions handed to backends as context.
    pub memory: Vec<crate::cognition::MemoryEntry>,
    max_memory: usize,
}

impl Subject {
    pub fn new(id: SubjectId, backend_id: CognitionBackendId, max_memory: usize) -> Self {
        assert!(max_memory > 0);
        Self {
            id,
            incarnation: IncarnationId::generate(),
            backend_id,
            memory: Vec::with_capacity(max_memory),
            max_memory,
        }
    }

    fn remember(&mut self, entry: crate::cognition::MemoryEntry) {
        if self.memory.len() == self.max_memory {
            self.memory.remove(0);
        }
        self.memory.push(entry);
    }
}

/// Drives subjects through worlds. One runner per experiment.
pub struct SubjectRunner {
    pub cadence: CognitionCadence,
    pub ledger: ResourceLedger,
    /// Energy at which a subject dies (world-agnostic accounting; worlds
    /// may override with their own death semantics via WorldEvent).
    pub death_threshold: f32,
    /// Logical clock; worlds may have their own but the runner's tick
    /// numbering is what appears in the event log.
    pub current_tick: u64,
}

impl SubjectRunner {
    pub fn new(cadence: CognitionCadence, ledger_capacity: usize) -> Self {
        Self {
            cadence,
            ledger: ResourceLedger::new(ledger_capacity),
            death_threshold: 0.0,
            current_tick: 0,
        }
    }

    /// Run one world tick for one subject.
    ///
    /// Sequence: world transition with prior action already consumed ->
    /// observe -> (maybe) decide -> validate -> charge cognition ->
    /// stage action for next tick's world step.
    ///
    /// We return the action to be applied on the *next* tick, matching the
    /// world's step-then-observe ordering. Worlds apply actions during step.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        subject: &mut Subject,
        world: &mut dyn World,
        subject_ref: &SubjectRef,
        backend: &mut dyn CognitionBackend,
        pending_action: Option<Action>,
        log: &mut EventLog,
    ) -> Result<Option<Action>, RuntimeError> {
        let tick = self.current_tick;
        self.current_tick += 1;

        // 1. World step applies the staged action from the previous decision.
        let world_events = world.step(subject_ref, pending_action.as_ref());
        log.push(EventPayload::WorldEvents {
            tick,
            events: world_events.clone(),
        });

        // 2. Observe through WABIX.
        let observations = world.observe(subject_ref)?;

        // 3. Cognition event on cadence.
        let mut next_action = None;
        if self.cadence.fires_on(tick) {
            let ctx = DecisionContext {
                subject_id: subject.id.to_string(),
                tick,
                energy: subject_ref.energy,
                observations,
                manifest: world.manifest().clone(),
                memory: subject.memory.clone(),
            };
            let decision: Decision = backend.decide(&ctx)?;
            validate_decision(world.manifest(), &decision)?;

            charge_cognition(
                &mut self.ledger,
                tick,
                &crate::cognition::CognitionCost {
                    cognition_events: 1,
                    tokens: 0, // backends report their own token counts (M7)
                },
                &subject.id.to_string(),
            );

            log.push(EventPayload::DecisionMade {
                subject_id: subject.id.to_string(),
                tick,
                decision: decision.clone(),
            });

            if let Some(action) = &decision.action {
                // Record the cost of the action per manifest so the world
                // can apply/charge it during its own transition.
                let _cost = world.action_cost(action)?;
                next_action = Some(action.clone());
            }

            subject.remember(crate::cognition::MemoryEntry {
                tick,
                action: decision.action.clone().unwrap_or(Action {
                    name: "WAIT".into(),
                    params: vec![],
                }),
                reward_signal: 0.0, // wired to world feedback in M3+
            });
        }

        Ok(next_action)
    }
}
