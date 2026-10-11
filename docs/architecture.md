# Architecture

Status: describes what exists on main (M0–M8 + research experiments merged).
Updated incrementally as architecture evolves.

## High-level layout

```
Transubstrate/
├── crates/
│   ├── ts-core/            # core library (everything below)
│   │   └── src/
│   │       ├── ids.rs                   # typed identity (9 ID types)
│   │       ├── wabix.rs                 # WABIX v0: World trait, manifests,
│   │       │                            #   typed Observation/Action, validation
│   │       ├── cognition.rs             # CognitionBackend trait, DecisionContext,
│   │       │                            #   MemoryEntry, cost charging
│   │       ├── backends.rs              # mock backends: echo / random / rules
│   │       ├── llm_backend.rs           # shared LLM backend (OpenAI-compatible)
│   │       ├── resource.rs              # append-only ResourceLedger
│   │       ├── event_log.rs             # JSONL event log (seq + version)
│   │       ├── rng.rs                   # SplitMix64 deterministic RNG
│   │       ├── runtime.rs               # Subject, SubjectRunner, cadence
│   │       ├── experiment.rs            # TestWorld vertical-slice harness
│   │       ├── primordial.rs            # primordial ALife world (grid)
│   │       ├── primordial_experiment.rs # multi-organism experiment runner
│   │       ├── network_world.rs         # network-v0 world (ring topology)
│   │       ├── world_migration.rs       # primordial → network migration
│   │       ├── lineage.rs               # Genome, mutation, LineageTree
│   │       ├── continuity.rs            # MigrationEvent, fingerprints, vector
│   │       ├── migration_experiment.rs  # A/B migration experiment + controls
│   │       ├── benchmark.rs             # reference-8gb benchmark harness
│   │       ├── replay.rs                # manifests + metrics regeneration
│   │       ├── scarcity.rs              # scarcity/drift experiment
│   │       └── competition.rs           # two-phenotype competition
│   └── transubstrate/      # CLI binary
└── docs/
```

## Core data flow (vertical slice)

```
Subject(id, memory)
    │
    ▼
CognitionBackend.decide(DecisionContext)     ── bounded context in
    │                                         ── typed Decision out
    ▼
WABIX validate(manifest, action)             ── reject unknown/out-of-range
    │
    ▼
World.tick(actions)                          ── physics + action application
    │                                        ── metabolism, death, events
    ▼
EventLog.push(...)                           ── JSONL, seq-numbered, versioned
    │
    ▼
replay/inspect                               ── metrics without re-running cognition
```

## Key types

### Identity (`ids.rs`)

Nine typed ID wrappers. All are opaque strings (UUIDv7 text or static
names). **Administrative identity ≠ measured continuity** — a SubjectId
persists across migrations; whether behavior remains continuous is an
empirical question answered by `continuity.rs`, never by the ID itself.

### WABIX (`wabix.rs`)

```
trait World {
    fn implementation(&self) -> &'static str;
    fn manifest(&self) -> &CapabilityManifest;          // discovery
    fn observe(&self, subject) -> Vec<Observation>;     // sensing
    fn validate(&self, action) -> Result<(), WabixError>;
    fn action_cost(&self, action) -> Result<f32, ..>;   // manifest-derived
    fn step(&mut self, subject, action) -> Vec<WorldEvent>;
}
```

- `CapabilityManifest`: observation channels (name/unit/range) + action
  schemas (name/arity/param ranges/cost)
- `WorldEvent`: SubjectDied / ActionApplied / ActionRejected
- Two structurally different worlds implement it: `primordial-v0` (2D grid)
  and `network-v0` (ring topology) — proving the ABI generalizes.
- Multi-organism worlds drive all subjects internally via `tick()`;
  the per-subject `step()` contract is used by TestWorld-style worlds.

### Cognition (`cognition.rs`, `backends.rs`, `llm_backend.rs`)

```
trait CognitionBackend {
    fn implementation(&self) -> &'static str;
    fn decide(&mut self, DecisionContext) -> Result<Decision, CognitionError>;
}
```

- `DecisionContext`: subject_id, tick, energy, observations, manifest,
  bounded memory (`Vec<MemoryEntry>`, reward signals included)
- Backends: echo, random (seeded), energy-rule, reproducing-rule,
  malformed (safe-failure harness), LLM (OpenAI-compatible)
- All decisions pass `validate_decision(manifest, decision)` before
  execution; violations are typed errors, never executed

### Resource accounting (`resource.rs`)

Append-only `ResourceLedger` with bounded in-memory window and exact u128
totals across compaction. Kinds: energy, cognition_events, tokens, cpu_micros,
storage_bytes. Token counts and wall-clock are engineering proxies, not
biological claims.

### Event log (`event_log.rs`)

JSONL records `{seq, log_version, payload}` with payload tagged
`experiment_started | subject_created | decision_made | world_events |
subject_state | resource_consumed | migration | experiment_finished`.
The log is the replay ground truth.

## 6. Worlds behind one ABI (moved to wabix-specification.md)

The full world comparison table lives in
[wabix-specification.md](wabix-specification.md) §6. Summary: three worlds
implement the same trait — `test-world-v0` (counter), `primordial-v0` (2D
grid), `network-v0` (ring). The subject-side experience is identical:
discover manifest → observe → act → events. `world_migration.rs` moves a
SubjectId from primordial to network at a midpoint tick with carried-over
energy.

## Continuity measurement (`continuity.rs`)

- `MigrationEvent`: content-hash id (FNV-1a64) — replay-stable
- `BehaviorFingerprint`: normalized action distribution + L1 distance
- `ContinuityVector`: per-dimension metrics, each with a `computation`
  string; the scalar aggregate is documented-arbitrary and never replaces
  components
- Experiment design: migration run (A→B at midpoint) vs two same-seed
  controls; `migration_effect = continuity(migration) − continuity(control)`

## Benchmark (`benchmark.rs`)

`transubstrate benchmark` measures world tick throughput (1/8/32 organisms),
mock cognition decisions/s, peak host RAM (VmHWM), event log size per 1000
ticks. `generated_at` intentionally omitted for replay determinism.

## Reproducibility model

- Deterministic parts: world physics, mock backends, seeded RNG (SplitMix64)
- Nondeterministic parts: LLM sampling (temperature 0 mitigates, does not
  guarantee) — outputs are recorded in event logs for post-hoc analysis
- Manifests: versioned `ExperimentManifest` (defined; per-run emission is
  the next infrastructure item)
- Replay: `RegeneratedMetrics` from log parsing without re-running cognition

## Design invariants

1. SUBJECT ≠ MODEL: swapping a backend never creates a new subject
2. No semantic leakage: channels are numbers; prompts never say "alive",
   "food", "survive" (enforced by unit test)
3. All model output passes WABIX validation before execution
4. Resource costs are explicit and logged
5. Bounded memory everywhere (event log window, memory limit, ledger window)
6. Reference-hardware first: everything must run on 8-core CPU / 32GB RAM /
   8GB VRAM with no paid cloud

## Milestone status

M0–M8 merged (see docs/status.md). Research experiments: scarcity/selection
(PR#11, negative result), two-phenotype competition (PR#12, positive
selection measured). Next: per-subject memory for LLM path (M11 continuation),
M9 world-migration experiments, M10 benchmark expansion.
