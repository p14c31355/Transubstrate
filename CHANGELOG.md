# Changelog

All notable changes to Transubstrate are documented here.

## [Unreleased]

### Added (feat/project-foundation)
- Cargo workspace: `ts-core` (core library) + `transubstrate` (CLI binary).
- Typed identity system: SubjectId, IncarnationId, CognitionBackendId,
  MachineId, BodyId, WorldId, LineageId, MigrationEventId, ExperimentId —
  administrative identity explicitly separated from measured continuity.
- Resource ledger: append-only, bounded memory, exact totals across
  compaction; kinds for energy, cognition events, tokens, CPU time, storage.
- WABIX v0 skeleton: capability manifests, typed observations/actions,
  shared validation, WABIX errors.
- Cognition backend ABI: bounded decision context, typed decisions,
  schema-violation-as-typed-error, cognition cost charging.
- Append-only event log (JSONL sink + bounded memory) with seq numbers and
  schema version.
- Deterministic SplitMix64 RNG.
- Subject runtime skeleton: cadence-based cognition, tick loop, staging of
  validated actions.
- CI: fmt, clippy, test, release build on GitHub Actions.
- README, CONTRIBUTING, SECURITY, .gitignore.

### Added (feat/vertical-slice, unmerged)
- Cognition backends: EchoBackend (deterministic baseline), RandomBackend
  (seeded), EnergyRuleBackend (positive control), MalformedBackend (safe
  failure testing).
- TestWorld: deterministic two-action world exercising the full WABIX loop.
- Experiment harness: config → run → summary + event log, with maintenance
  costs, death by energy exhaustion, cadence-based cognition.
- CLI: `transubstrate run-test` (real experiment execution) and
  `transubstrate inspect` (event log digest).
- 26 unit tests passing.

### Added (feat/lineage, unmerged)
- Genome: heritable configuration (division threshold, absorb affinity,
  signal bias, move bias, mutation rate) with clamped deterministic mutation
- DivisionPolicy: child energy fraction, division cost
- LineageTree: bounded parent/child lineage records with lineage id and
  generation depth
- PrimordialWorld: DIVIDE action (genome-gated, energy-splitting, mutated
  child on adjacent cell, population cap), organisms carry genome+lineage
- ReproducingRuleBackend: absorb/divide rule for reproduction experiments
- Run summary extended: births, divide_events, distinct_lineages,
  max_generation; CLI reports lineage stats
- Real runs: 4 founders -> 12 alive (8 divisions, gen 1); 8 founders ->
  32 alive, 24 divisions, gen 2; population cap enforced
- 44 tests passing

### Added (feat/continuity-migration, unmerged)
- MigrationEvent with deterministic content-hash id (replay-stable)
- ContinuityVector: interpretable per-dimension continuity with documented
  computation strings; documented_aggregate preserves components and marks
  its own arbitrariness
- BehaviorFingerprint: normalized action distributions, L1 distance
- Migration experiment harness: A/B with two controls (same-seed same-backend
  control contrast); migration effect = migration continuity - control
  continuity
- CLI run-migration command
- Real runs: echo->energy-rule gives continuity 0.0 / effect -1.0 (full
  behavior change); echo->echo gives continuity 1.0 / effect 0.0 (identical
  to control) — harness calibration confirmed
- 54 tests passing

### Added (feat/replay, unmerged)
- ExperimentManifest: machine-readable, versioned re-execution record
- Metrics regeneration from event logs without cognition re-execution
  (RegeneratedMetrics: decisions, action distribution, deaths, births,
  migrations, population samples)
- Log integrity verification (sequential seq numbers, schema version)
- CLI replay + verify commands
- Verified against real logs: regenerated ABSORB count == run summary
  count; migration logs count 1 migration; tampered seq detected
- 59 tests passing

### Added (feat/second-world, unmerged)
- NetworkWorld (`network-v0`): ring-topology world behind WABIX — nodes,
  node energy, decaying signal field, WAIT/ABSORB/SIGNAL/HOP, metabolism,
  death, admit_subject() for world-migration with carried-over energy
- WorldMigrationConfig/Result + run_world_migration: subject migrates from
  primordial (grid) to network (ring) at midpoint with carried-over energy;
  pre/post fingerprints + same-world control continuity
- 79 tests passing

### Added (bench/reference-8gb, unmerged)
- BenchmarkReport/BenchmarkSection (reference-8gb): world tick throughput
  (1/8/32 orgs), mock cognition decisions/sec, peak host RAM (VmHWM),
  event log bytes per 1000 ticks (8 orgs)
- CLI `benchmark [--out report.json]`
- Measured on reference machine: ~139k ticks/s (1 org), ~130k (8 orgs),
  ~84k (32 orgs), mock decisions ~5.8M/s, peak host RAM ~3.4 MiB,
  ~315KB log per 1000 ticks (8 orgs)

### Added (PR#20, merged)
- Migration harness now carries per-subject bounded memory ACROSS the
  migration boundary (memory belongs to the subject, not the backend)
- MemoryEntry.pre_tick_energy: reward = post-tick energy − pre-decision
  energy; shared record_memory() helper (FIFO limit param)
- Primordial experiment runner also records pre_tick_energy

### Added (PR#30-#33, merged)
- NetworkWorld: signal-driven energy pull — a signaling node draws
  `signal_pull_rate * strength` energy per tick from each ring neighbor
  (energy-conserving, applied before field decay). SIGNAL now has a real
  resource payoff and HOP gets a reason to chase signal-rich nodes.
- SignalSeekerBackend (`signal-seeker`): positive control for the network
  world — survive (ABSORB when local_energy low) -> seed the signal ->
  harvest. Verified end-to-end: pre-world {SIGNAL 35, ABSORB 5} ->
  post-world {ABSORB 40}, zero deaths.
- Tests: pull concentrates energy on the signaling node, conserves totals
  (modulo uniform regen), and stops after field decay.

### Added (PR#35-#45, merged)
- LlmBackendConfig.reasoning_effort: thinking-style models (qwen3.5) return
  empty completions without it; CLI syntax `llm:<model>[:<effort>]` with a
  known-effort allowlist
- `--llm-temperature` for run-world-migration (default 0 = deterministic)
- LLM memory context now renders each past decision WITH its observed
  outcome (leakage-free)
- Cross-model migration record: granite behavioral 0.75 vs qwen 0.00 —
  temperature-0 tie broken; post-migration behavior is model-dependent
- Outcome-driven exploration: at temp 0.7, 2/5 seeds learned SIGNAL from
  outcome-visible memory; seed 2026 adapted post-swap (ABSORB 38 → SIGNAL 23)
- Temperature sweep (sweep-world-migration batch runner, n=5/temp):
  temp 0.3 dominates — 3/5 SIGNAL learning, 0 deaths (vs 2/5 with 2 deaths
  at 0.7); temp 1.0: 2/4 valid + 1 schema-rejection (`ABSORBT` typo) —
  WABIX fail-safe OBSERVED end-to-end in an LLM migration
- memory_usage continuity dimension (5-dim vector): fraction of boundary-
  remembered actions re-enacted post-swap, immune to FIFO roll-off; wired
  into BOTH harnesses (rule→LLM 1.0, rule→echo 0.0, world signal-seeker
  0.5 = strategy flip, control 1.0)
- sweep-world-migration subcommand: multi-seed A/B batch runner emitting
  mean ± population-sd per dimension (JSON stdout + --summary file)
