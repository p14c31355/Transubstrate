# Project Status

Last updated: 2026-10-11 (PR#52-#57 arc — memory ablation, signaling economics, cognitive subjects in the signaling world)

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
- **World migration (M9, PR#22–#24, #28)**: second world (`network-v0`, ring
  topology with per-node energy) behind the same WABIX trait;
  `run-world-migration` runs subject → primordial half → network half with
  carried energy, PLUS a same-seed no-swap control and a per-dimension
  effect vector (migration − control). Event log + manifest + replay all
  work for both runs. Measured: energy-rule effect −1.0/−1.0 (swap-fragile),
  LLM effect ±0.0 (swap-robust because conservative).
- **Per-subject bounded memory (M11, PR#17, #20, #23)**: memory limit 8
  (FIFO), wired into the primordial population runner, the model-migration
  harness, AND across the world-migration boundary — memory belongs to the
  subject, not the backend or the world. LLM decisions receive the buffer.
- **memory_retention + memory_usage continuity dimensions (PR#24, #42, #44)**:
  five-dimension ContinuityVector. memory_retention = multiset match of
  (action, reward-bin) keys across the swap (echo→echo 1.0, rule→echo 0.0);
  memory_usage = fraction of boundary-remembered actions re-enacted
  post-swap, immune to FIFO roll-off. Wired into BOTH harnesses; world-
  migration calibration: signal-seeker usage 0.5 (SIGNAL→ABSORB strategy
  flip re-enacted half its boundary memory), control 1.0, effect −0.5.
- **Memory ablation — causal carry-vs-wipe (PR#52, #53)**: `memory_carry:
  bool` in both migration configs + boundary wipe + per-seed ablation arm
  (`--no-memory` CLI; `ablation_effect` in sweep JSON). Rule calibration
  0.0 (true negative). Same-trajectory refinement: the wipe arm REPLAYS
  the carry arm's pre-swap decisions — 5/5 seeds pre-phase exactly
  matched; LLM n=5: behavioral +0.68, retention +0.60, usage +1.00; wipe
  arms jump to uniform SIGNAL saturation — memory causally shapes
  post-swap strategy (docs/experiments/memory-ablation.md).
- **Signaling world economics (PR#54, #55)**: survival-tuned budget (40
  units) — 5/6 followers survive 600 ticks in 5/5 seeds, intake ~2.6×
  null; gradient channels (`nearby_signal_left`/`right`) — followers
  converge on the emitter (proximity 0.98+); EMERGENT carrying capacity
  (2/6 survive at the beacon — node capacity caps co-residency, a
  population cap from economics, not an imposed rule)
  (docs/experiments/signaling-followers.md).
- **Cognitive subjects in the signaling world (PR#56, #57)**: single LLM
  follower (granite, n=3) — 3/3 survive, NULL-SIGNAL discovery (SIGNAL
  strength 0.0 is the LLM's cheap safe action); seed 99 sent real signals
  and converged on the emitter. Two-LLM convention run — NEGATIVE
  (disclosed): both converge on null-signal, no interaction, rule
  emitter's signal ignored at temp 0.3
  (docs/experiments/llm-signaling.md, convention-formation.md).
- **CLI**: `run-test`, `run-primordial`, `run-migration`, `run-world-migration`,
  `sweep-world-migration`, `run-signaling`, `benchmark`, `replay`, `verify`,
  `inspect`. Backends: echo, random, energy-rule, reproducing-rule,
  signal-seeker, `llm:<model>[:<effort>]`.
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
- Memory-retention calibration (PR#24): echo→echo memory_retention 1.0
  (matched 8/8); rule→echo 0.0 (matched 0/8) — the dimension tracks whether
  remembered decisions/outcomes survive the swap.
- World migration baseline (energy-rule, seed 42): behavioral continuity
  0.0 (ABSORB pre → WAIT post), memory_retention 0.0 (matched 0/8).
- World migration with same-world control (PR#28): energy-rule effect
  −1.0/−1.0 (the swap breaks the rule subject; the control run stays at
  1.0/1.0). LLM (granite4.1:3b-q4_K_M, seed 42) effect ±0.0/±0.0 — the
  conservative WAIT policy is swap-robust; swap-fragile vs swap-robust
  cognition is now a measured distinction.
- Real LLM migration (M8): rule→LLM continuity 0.10 / effect −0.90 on 3/3
  seeds; superseded by the 5-seed stats (behavioral 0.75±0) after memory
  context entered the prompt — prompt-sensitivity, not robustness
  (llm-model-migration-stats.md).
- Outcome-driven exploration (PR#37, temp 0.7, n=5): 2/5 seeds discovered
  and repeated SIGNAL from outcome-visible memory ("Repeat successful
  SIGNAL from past ticks"); seed 2026 adapted post-swap (ABSORB 38 →
  SIGNAL 23). First OBSERVED experience-driven action learning without
  human-assigned channel meaning. High variance: 2/5 deaths.
- Temperature sweep (PR#40/#43, batch via sweep-world-migration):
  temp 0.3 dominates — 3/5 seeds learned SIGNAL post-swap with ZERO deaths
  (0.7: 2/5 with 2 deaths; 1.0: 2/4 valid with 1 schema-rejection
  `ABSORBT` — the fail-safe path OBSERVED in an LLM migration).
  Behavioral 0.65 ± 0.314 at 0.3 (bimodal: variance is the adoption
  signature). Post-swap ABSORB → SIGNAL adaptation at 3/5 seeds.
- Cross-model (PR#35): qwen3.5:2b-q4_K_M behavioral 0.00 vs granite 0.75
  under the identical harness — post-migration behavior is model-dependent.
- LLM integration (M7): granite4.1:3b-q4_K_M produced valid typed decisions
  in 80/80 cognition events on the reference GPU; zero schema rejections.

## What has NOT been demonstrated

- No emergence claims of any kind. No signaling conventions have formed or
  been detected (the signal field works mechanically; no experiment has yet
  tested convention formation).
- "Evolution" remains unclaimed: selection on one engineered trait
  (absorb_affinity) is now MEASURED (PR#12: share 0.759, 5/5 seeds), but no
  open-ended evolution, speciation, or niche formation has been tested.
- No signaling CONVENTION (multi-subject, stable) has formed or been
  detected: single-subject experience-driven SIGNAL learning is OBSERVED
  (PR#37), but no experiment has yet tested whether a convention forms
  across subjects or survives individuals.
- LLM world migration exists (PR#26, #28) but the swap did not perturb the
  LLM subject (effect ±0.0); adaptation to a world change is OBSERVED only
  via outcome learning at temp 0.7 (PR#37), not via manifest-channel
  adaptation.
- Memory USE after a swap: LLM numbers now recorded (PR#47 record,
  temp 0.3 n=5): 4/5 seeds usage 1.0 (boundary memory re-enacted;
  swap-robust), 1 seed 0.0 across all three dimensions (full strategy
  replacement ABSORB→SIGNAL — the outcome-learning loop visible in the
  vector). At temperature 0 the learning loop remains untested.
- No machine/process migration yet.
- Multi-seed statistics exist for the rule→LLM migration (5 seeds),
  PR#12 competition (5 seeds), the LLM world migration (5 seeds), outcome
  exploration (5 seeds per temperature, PR#37 + PR#40), and the temperature
  sweep is batch-collected via sweep-world-migration (PR#39); other
  experiments are single-seed observations.
- The temperature-0 seed agreement is prompting structure, not cross-model
  invariance: qwen3.5:2b-q4_K_M diverges from granite4.1 (behavioral 0.00 vs
  0.75) under the identical rule→LLM harness (cross-model-migration.md).

## What is blocked / deferred

- docs/prior-art.md: survey draft landed (PR#7); survey gaps listed in the
  document itself (open-ended evolution lit, sim-to-real, agent checkpoints).
- Field-energy time series in regenerated population samples carries 0.0
  (SubjectState records don't include it; documented limitation).

## Verification status

- 86 unit tests passing; fmt + clippy -D warnings clean.
- CI green on all merged PRs (#1–#24).
- Real release-build CLI runs recorded in PR descriptions (#3, #4, #5, #8,
  #9, #11, #12, #22, #24).
