# Project Status

Last updated: 2026-10-11 (PR#12 merged — selection measurement)

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
- **Local LLM cognition backend (M7)**: LlmBackend over OpenAI-compatible
  endpoints (Ollama / llama.cpp server); ONE shared instance serves the whole
  population (VRAM does not scale with organism count); strict JSON decision
  schema with safe failure; token accounting into the resource ledger; prompt
  semantic-leakage scan as a unit test. Default model granite4.1:3b-q4_K_M
  (2.1 GB), verified on the actual reference GPU (RTX 2070 SUPER 8GB,
  local Ollama 0.23.2): 80 cognition events, 0 backend errors.
- **Real model migration experiment (M8)**: energy-rule → local LLM at tick
  200, measured with the M5 harness. Seeds 42/7/99 all show behavioral
  continuity 0.10, control continuity 1.00, migration effect −0.90. LLM
  rationales are observation-dependent (e.g. "Energy low, no resources to
  absorb" vs "Emit signal to communicate").
- **Scarcity/selection instrument (PR#11, PR#12)**: multi-seed drift
  measurement and two-phenotype competition. Negative result at default
  mutation (drift +0.0006, below ±0.02 band); positive selection measured
  via direct phenotype competition (shareA 0.759, 5/5 seeds, mechanism
  confirmed). regen=0 → extinction.
- **CLI**: `run-test`, `run-primordial`, `run-migration`, `replay`,
  `verify`, `inspect`. Backends: echo, random, energy-rule,
  reproducing-rule, `llm:<model>`.
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
- Real LLM migration (M8): rule→LLM continuity 0.10 / effect −0.90 on 3/3
  seeds; LLM action window on seed 7: ABSORB 70, SIGNAL 2, WAIT 48.
- LLM integration (M7): granite4.1:3b-q4_K_M produced valid typed decisions
  in 80/80 cognition events on the reference GPU; zero schema rejections.

## What has NOT been demonstrated

- No emergence claims of any kind. No signaling conventions have formed or
  been detected (the signal field works mechanically; no experiment has yet
  tested convention formation).
- "Evolution" remains unclaimed: selection on one engineered trait
  (absorb_affinity) is now MEASURED (PR#12: share 0.759, 5/5 seeds), but no
  open-ended evolution, speciation, or niche formation has been tested.
- No memory-retention continuity: the LLM path has no per-subject memory yet;
  the M8 continuity measures behavioral distribution only.
- No world migration (second world) yet.
- No machine/process migration yet.
- Multi-seed statistics exist only for the M8 migration (3 seeds); all other
  experiments are single-seed observations.
- The 3-seed agreement in M8 reflects temperature-0 prompting structure, not
  proven robustness across models or prompt variants.

## What is blocked / deferred

- docs/prior-art.md: survey draft landed (PR#7); survey gaps listed in the
  document itself (open-ended evolution lit, sim-to-real, agent checkpoints).
- Field-energy time series in regenerated population samples carries 0.0
  (SubjectState records don't include it; documented limitation).
- Per-subject memory for the LLM path: next major work item (docs/next.md).

## Verification status

- 74 unit tests passing; fmt + clippy -D warnings clean.
- CI green on all merged PRs (#1–#12).
- Real release-build CLI runs recorded in PR descriptions (#3, #4, #5, #8, #9, #11, #12).
