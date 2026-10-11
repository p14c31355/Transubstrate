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

### Added (feat/migration-memory, unmerged)
- Migration harness now carries per-subject bounded memory ACROSS the
  migration boundary (memory belongs to the subject, not the backend)
- MemoryEntry.pre_tick_energy: reward = post-tick energy − pre-decision
  energy; shared record_memory() helper (FIFO limit param)
- Primordial experiment runner also records pre_tick_energy
