# WABIX Specification v0

Status: **v0 — implemented and exercised by three worlds** (TestWorld,
primordial-v0, network-v0). This document specifies the contract as
implemented in `crates/ts-core/src/wabix.rs`.

## 1. Overview

WABIX is the boundary between an artificial subject and whatever world/body
it inhabits. Its design goal is the role of an ABI: subjects are written
against the contract, not against any specific world. Worlds implement the
`World` trait; subjects interact only through it.

Design goals:

- capability discovery is explicit, never assumed
- all actions are typed and validated before execution
- free-form model output never reaches execution directly
- deterministic-friendly (no wall-clock reads inside the trait)
- multi-organism worlds may drive all subjects internally via `tick()`;
  single-subject worlds may implement `step()`

## 2. Core types

### ObservationChannel

```rust
pub struct ObservationChannel {
    pub name: String,
    pub unit: String,          // human-readable label, e.g. "scalar"
    pub range: (f32, f32),     // normalized value range
}
```

### ActionSchema

```rust
pub struct ActionSchema {
    pub name: String,
    pub arity: u8,                    // number of f32 params
    pub param_ranges: Vec<(f32, f32)>,// per-param valid ranges
    pub cost_per_unit: f32,           // world energy cost per unit magnitude
}
```

### CapabilityManifest

```rust
pub struct CapabilityManifest {
    pub world_id: String,
    pub observation_channels: Vec<ObservationChannel>,
    pub actions: Vec<ActionSchema>,
}
```

### Observation / Action

```rust
pub struct Observation { pub channel: String, pub value: f32 }
pub struct Action      { pub name: String, pub params: Vec<f32> }
```

### SubjectRef

```rust
pub struct SubjectRef {
    pub subject_id: String,
    pub pos: (u32, u32),   // world-local position key
    pub energy: f32,       // world-local energy store
}
```

### WorldEvent

```rust
pub enum WorldEvent {
    SubjectDied { subject, cause },
    ActionApplied { subject, action, cost },
    ActionRejected { subject, action, reason },
}
```

## 3. The World trait

```rust
pub trait World {
    fn implementation(&self) -> &'static str;
    fn manifest(&self) -> &CapabilityManifest;                       // discover
    fn observe(&self, subject: &SubjectRef)
        -> Result<Vec<Observation>, WabixError>;                    // observe
    fn validate(&self, action: &Action) -> Result<(), WabixError>;  // validate
    fn action_cost(&self, action: &Action) -> Result<f32, WabixError>;
    fn step(&mut self, subject: &SubjectRef, action: Option<&Action>)
        -> Vec<WorldEvent>;                                         // act
}
```

Contract:

1. `implementation()` returns a stable implementation id (distinct from
   world *instances*).
2. `manifest()` is the complete capability set; subjects must never assume
   a channel or action that is not in the manifest.
3. `observe()` may restrict visibility (local sensing); the manifest declares
   what *exists*, not who *sees what*.
4. `validate()` checks action name + arity + param ranges against the
   manifest. Shared reference implementation: `validate_action()`.
5. `action_cost()` is `cost_per_unit × Σ|params|`, valid only after
   `validate()` passes.
6. `step()` applies one validated action for one subject. Multi-organism
   worlds (primordial, network) implement `step()` as unreachable and drive
   all subjects through `tick()` instead; this is documented per-world.

## 4. Validation semantics

Shared reference implementation `validate_action(manifest, action)`:

- unknown action name → `WabixError::UnknownAction`
- wrong param count → `WabixError::InvalidParameters`
- param out of declared range → `WabixError::InvalidParameters`

Worlds must call `validate_action` both at validation time (cognition) and
at execution time inside `step()`/`tick()` (defense in depth). An action
that validated at cognition time but fails at execution (e.g. insufficient
energy) is reported as `ActionRejected` in the event log; the subject keeps
its energy and the action is lost.

## 5. Failure semantics

| Failure | Where | Behavior |
|---|---|---|
| Unknown action / bad params | cognition-time validation | typed `WabixError`; decision rejected before execution |
| Execution-time rejection (e.g. insufficient energy) | world step/tick | `ActionRejected` event; subject keeps energy; action lost |
| Backend error (network, malformed output) | cognition | `CognitionError`; logged as `ActionRejected` with reason; subject continues |
| Dead subject observed | observe | `WabixError::UnknownChannel` |

## 6. World implementations (v0)

| World | implementation id | Topology | Channels | Actions |
|---|---|---|---|---|
| TestWorld | `test-world-v0` | n/a (counter) | counter, jitter | WAIT, BUMP |
| PrimordialWorld | `primordial-v0` | 2D grid | local_energy, chemical_0, nearby_signal_0 | WAIT, ABSORB, RELEASE, SIGNAL, MOVE, DIVIDE |
| NetworkWorld | `network-v0` | ring | node_energy, nearby_signal_0, degree | WAIT, ABSORB, SIGNAL, HOP |

Multi-organism worlds (primordial, network) implement `step()` as
`unreachable!()` and drive all subjects through `tick()`; this is documented
per-world and is a sanctioned v0 pattern. `network-v0` additionally provides
`admit_subject(subject_id, node, energy)` — the entry point for
world-migration with carried-over energy while preserving SubjectId.

## 7. Versioning & compatibility

- WABIX spec version: **v0** (this document).
- The event log carries its own `log_version` (currently 1), independent of
  the WABIX spec version.
- Compatibility policy for v0→v1:
  - adding new channels/actions to a manifest is backward-compatible for
    subjects (they must never assume capabilities not in the manifest)
  - removing/renaming channels or actions, or changing param semantics, is
    a breaking change → new world implementation id (e.g. `primordial-v1`)
    and a manifest version bump
  - subjects must handle `WabixError::UnknownAction` / `UnknownChannel`
    gracefully (fail safely) — this is what makes additive evolution safe
- Capability negotiation in v0 is implicit: subjects discover via
  `manifest()` before acting. Explicit negotiation (requesting/releasing
  capabilities) is deferred to the capability/safety model (v1).

## 8. Future: capability & safety model (v1 sketch)

Planned concepts (not implemented): `CapabilityGrant`, `Permission`,
`ActionLimit` (MaxMagnitude, MaxRate), `Deadline`, `SafetyClass`,
`Reversible`, `AuditEvent`. These gate physical-world adapters; simulations
run without them. No physical adapter will be attempted before this model
is specified.
