# Project Status

Last updated: 2026-10-11 (M6 merged)

## What currently works (all OBSERVED, verified on this repository's CI + local runs)

- **Core type system**: typed identity (SubjectId, IncarnationId,
  CognitionBackendId, MachineId, BodyId, WorldId, LineageId,
  MigrationEventId, ExperimentId). Administrative identity is structurally
  separate from measured continuity.
- **WABIX v0**: capability manifests, typed observations/actions, shared
  validation, typed errors. Proven by two worlds implementing the same trait
  (TestWorld, PrimordialWorld).
- **Cognition backend ABI**: bounded decision context, typed decisions,
  schema violations as typed errors, safe failure (backend errors are logged
  and the subject continues without acting).
- **Cognition backends**: echo (deterministic baseline), random (seeded),
  energy-rule (positive control), reproducing-rule (absorb/divide probe),
  malformed (safe-failure harness).
- **Resource accounting**: append-only ledger with bounded memory and exact
  totals across compaction; energy, cognition events, tokens, CPU, storage
  kinds.
- **Event log**: append-only JSONL, seq-numbered, versioned, disk-backed;
  integrity verification (sequential seq, version).
- **Primordial world v0** (`primordial-v0`): 2D grid, energy field with
  diffusion + regeneration, chemical field, decaying signal field, metabolism,
  WAIT/ABSORB/RELEASE/SIGNAL/MOVE/DIVIDE with energy costs, death by
  exhaustion. Deterministic per seed (verified: same seed → same trajectory,
  different seed → divergent).
- **Reproduction/lineage v0**: genome with clamped deterministic mutation,
  DIVIDE gated by genome threshold + energy, child on adjacent cell,
  population cap, LineageTree with parent/child/generation tracking.
- **Migration + measured continuity v0**: MigrationEvent with deterministic
  content-hash ids, BehaviorFingerprint (L1), ContinuityVector with
  per-dimension documented computation, A/B experiment with two same-seed
  controls, migration effect = migration continuity − control continuity.
- **Replay**: metrics regeneration from event logs without re-running
  cognition; regenerated counts cross-validated against run summaries.
- **CLI**: `run-test`, `run-primordial`, `run-migration`, `replay`,
  `verify`, `inspect`.
- **CI**: GitHub Actions — fmt, clippy -D warnings, tests, release build.

## What has been measured (CALIBRATION data, not findings about emergence)

- echo(WAIT-only) organisms starve to extinction in ~100 ticks at 1.0
  metabolism (deterministic).
- energy-rule organisms survive and absorb; with default regen (0.01/cell/tick
  × 256 cells) the field regenerates faster than 8 organisms consume it —
  carrying capacity not yet binding.
- reproducing-rule organisms divide: 8 founders → 32 alive, 24 divisions,
  max generation 2 in 2000 ticks; population cap enforced.
- Migration harness calibration: echo→energy-rule gives behavioral continuity
  0.0 / effect −1.0; echo→echo gives continuity 1.0 / effect 0.0.

## What has NOT been demonstrated

- No emergence claims of any kind. No signaling conventions have formed or
  been detected (the signal field works mechanically; no experiment has yet
  tested convention formation).
- No evolution claims: mutation machinery exists, but no selection pressure
  experiment has shown trait frequency changes across generations.
- No LLM cognition yet — the system runs entirely without models (by design).
- No world migration (second world) yet.
- No machine/process migration yet.
- Continuity measurement has only the behavioral action-distribution
  dimension; memory/goal/preference dimensions await memory-carrying backends.
- The migration experiment's negative controls work, but with stateless mock
  backends the "migration effect" trivially equals the new controller's
  policy — scientifically meaningful migration experiments require M7.

## What is blocked / deferred

- Local LLM backend (M7) requires llama.cpp integration work; not blocked by
  anything external — simply next in queue.
- Field-energy time series in regenerated population samples carries 0.0
  (SubjectState records don't include it; documented limitation).
- docs/prior-art.md: survey in progress (verified sources collected; file
  write pending).

## Verification status

- 59 unit tests passing; fmt + clippy -D warnings clean.
- CI green on all merged PRs (#1–#6).
- Real release-build CLI runs recorded in PR descriptions (#3, #4, #5).
