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

### Added (PR#46-#51, merged)
- Multi-subject signaling harness (`signaling.rs` + `run-signaling` CLI):
  emitter/followers on the signal-pull ring vs a no-pull null run —
  attraction 5/5 seeds and ~2.1× intake vs null (the pull mechanism creates
  a measurable follower payoff); survival economics fail under current
  parameters (negative result disclosed)
- Experiment records: temperature sweep matrix (0.3/0.7/1.0),
  signaling-follower detection, experiment index row 18

### Added (PR#52-#57, merged)
- Memory ablation — causal carry-vs-wipe at the migration boundary:
  `memory_carry: bool` + boundary wipe in BOTH harnesses, `--no-memory`
  CLI flag, per-seed ablation arm in `sweep-world-migration` (carry −
  wipe effect in the JSON summary); rule calibration 0.0 (true negative);
  same-trajectory refinement: the wipe arm REPLAYS the carry arm's
  pre-swap decisions (5/5 seeds pre-phase exactly matched) — LLM n=5:
  behavioral +0.68 / retention +0.60 / usage +1.00; wipe arms jump to
  uniform SIGNAL saturation — memory causally shapes post-swap strategy
- Survival-tuned follower run: sweep-sized energy budget (40 units) —
  5/6 followers survive 600 ticks in 5/5 seeds, intake ~2.6× null
- Signal-gradient observation: `nearby_signal_left`/`right` channels —
  followers converge on the emitter (proximity 0.98-0.997); EMERGENT
  carrying capacity (2/6 survive at the beacon — node capacity caps
  co-residency); cue (signal diffusion) separable from payoff (pull
  energy)
- Multi-subject LLM signaling (`run_llm_signaling` + signal_strengths
  histogram): granite n=3 — 3/3 survive; NULL-SIGNAL discovery (SIGNAL
  strength 0.0 is the LLM's cheap safe action, 2/3 seeds); seed 99 sent
  real signals (0.2 ×122) AND converged on the emitter (proximity 1.0)
- Convention-formation harness (`run_cognitive_signaling`, repeated
  `--backend` flags): first attempt NEGATIVE (n=1) — two LLM followers
  independently converge on the null-signal, no interaction, rule
  emitter's signal ignored at temp 0.3; temp-0.7 arm queued

### Added (PR#58-#67, merged)
- Temperature-0.7 convention arm: null-signal is temperature-robust
  (5/6 subject-seeds unchanged across 0.3/0.7); convention formation
  NOT observed in 4 seeds × 2 temps; coupling received-signal→payoff
  would pre-define meaning and is consciously NOT introduced
- Population world migration (`run-population-migration`): 4-member
  mixed-backend population, per-subject continuity spread 0.281±0.418
- Manifest emission verified across all harnesses; scarcity CLI now
  writes event logs (`--seeds` flag added)
- PopulationSampled event: real field_energy in population samples
  (replaces the (alive,0) SubjectState hack; old logs replay compatibly)
- Carrying capacity (`--regen`): regen 0.0005 binds at ≈13-16 organisms;
  field stabilizes 400-600
- Reproduction under binding scarcity + mutation-floor fix:
  Genome::mutate no longer silently clamps low division thresholds
  (min(20, parent)); threshold 6.0 → 16 survivors 0 deaths (grows 8→16
  to the capacity limit); 3.0-5.0 = over-reproduction collapse — divide
  threshold is a life-history gene
- Chemical substrate (RELEASE channel 1): arity 2 matching the GOAL §6
  vocabulary — converts organism energy into the cell's chemical field;
  decay 0.1 + 4-neighbor diffusion 0.05; NO absorb payoff (meaning-free
  by construction); the M3-era sealed substrate is finally writable
- Observation adaptation (`compute_observation_adaptation`): lag_ticks +
  new_action + persists from the decision stream (10-decision windows);
  rule calibration lag 0 stable; granite 0.3 2/5 adapt (lag 50-100,
  persist); temp 1.0 lag-0 switches separated from learning via the
  persistence flag; qwen 1/5 switch reverts
- Long-run stability (100k ticks × 5 seeds): population perfectly
  stationary at the carrying capacity (0 deaths); field surplus
  accumulates ~29k while births freeze — an intake-rate ceiling distinct
  from the population cap; the movement hypothesis (mobile rule breaks
  the ceiling) TESTED and REJECTED — mobile rule crashes because
  movement cost exceeds supply density at regen 0.0005
- Chemical meaning-formation detector: `WorldEvent::ChemicalMarked` logs
  every channel-1 mark; `nearby_chemical_0` channel (4-neighbor gradient
  reach); `DecisionObservation` snapshots per decision; `--markers N`
  mixed populations (ChemMarker writers + LLM observers); `analyze-chem`
  reports observation-based mark_awareness and writer-excluded
  observer_awareness. Rule calibration true-negative; LLM n=5 seeds:
  observer_awareness |a|<0.01 — marks are SENSED but IGNORED (negative
  result, supports the no-payoff-coupling hypothesis)
