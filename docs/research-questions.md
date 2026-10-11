# Research Questions

The three Goal questions, with operational definitions and falsifiable
experiment designs. Claims are classified OBSERVED / COMPUTED / INFERRED /
HYPOTHESIZED / SPECULATIVE. Nothing in this document asserts consciousness,
sentience, personhood, or life.

---

## RQ1 — Continuity across substrate replacement

> Can an artificial subject preserve measurable continuity while its cognitive
> model, computational substrate, embodiment, and environment are independently
> replaced?

### Operationalization

- Subject identity = SubjectId (administrative, persists across migrations).
- Continuity = a vector of measured dimensions, never an ID assertion.
- Dimensions implemented: behavioral action-distribution (L1 fingerprint).
- Dimensions planned (require per-subject memory): memory retention, goal
  retention, preference stability.
- Experiment design: migration run (backend A → B at midpoint tick) vs two
  same-seed controls (A-only, B-only). `migration_effect = continuity(migration)
  − continuity(control_A_first_half_vs_second_half)`.

### Current status

- OBSERVED (calibration): echo→echo gives continuity 1.0 / effect 0.0
  (identical to control); echo→energy-rule gives continuity 0.0 / effect −1.0.
  The harness distinguishes substrate change from time passage.
- OBSERVED (M8): energy-rule → local LLM (granite4.1:3b-q4_K_M) gives
  continuity 0.10, control 1.00, effect −0.90 on 3/3 seeds; LLM rationales
  are observation-dependent.
- HYPOTHESIZED: with per-subject memory in the LLM path, memory-retention
  continuity will be measurable as a separate dimension.

### Falsifiability

If migrating backends produced continuity ≈ control continuity (effect ≈ 0)
while observation-dependent rationales persist, the behavioral-continuity
dimension would be falsified as a substrate-sensitivity measure.

---

## RQ2 — Simulation↔physical boundary via WABIX

> Can the same persistent artificial subject cross the boundary between
> artificial-life simulation and the physical world through a common world ABI?

### Operationalization

- The subject interacts only through WABIX: discover manifest → observe
  typed channels → act through validated typed actions → receive events.
- A physical "world" adapter must implement the same `World` trait; the
  subject code must be unchanged.
- Physical-world milestone requires a capability/safety model first
  (CapabilityGrant, ActionLimit, SafetyClass) before any actuator.

### Current status

- OBSERVED: two structurally different simulation worlds (grid `primordial-v0`,
  ring `network-v0`) implement the same trait; `world_migration.rs` moves a
  SubjectId between them with carried-over energy (harness built, experiments
  pending).
- HYPOTHESIZED: a Linux-process adapter (sensors/actuators via devices) can
  implement the same trait without subject-side changes.

### Falsifiability

If a physical adapter requires subject-side changes beyond configuration
(e.g. world-specific action names leak into cognition), the ABI abstraction
is falsified as designed and must be revised.

---

## RQ3 — Emergence in a primordial environment

> With resources, constraints, other organisms, and no pre-existing language:
> can life-like organization, adaptation, signaling, inheritance, ecology, or
> open-ended behavior emerge?

### Operationalization (per phenomenon, defined before running)

- **Selection** (measured — see below): trait frequency shift across
  generations attributable to fitness differences, replicated across seeds,
  with a null condition.
- **Signaling convention**: a stable correlation between a signal-channel
  value and a subsequent behavior by receivers, persisting across
  generations; detection metric defined before running.
- **Cultural transmission**: a convention surviving the death of all
  individuals that used it originally.

### Current status

- OBSERVED (PR#11, negative): under default mutation (0.15 sigma-scale),
  5-seed scarcity drift was +0.0006 (sd 0.0016), below the ±0.02 signal band —
  mutation drift alone is too weak to detect. regen=0 causes extinction.
- OBSERVED (PR#12, positive): two-phenotype competition (affinity 1.3 vs 0.7)
  → high-affinity share mean 0.759 (5/5 seeds), mechanism confirmed (births
  45–52 vs 16–21; deaths 2–5 vs 32–45). Null condition (equal affinity)
  stays balanced.
- COMPUTED: absorb_affinity multiplies extraction (energy gained = available ×
  efficiency × affinity), so the fitness differential is mechanical and tunable.

### Falsifiability

- If the null condition (equal affinity) produced share_A > 0.7 as often as
  the test condition, the selection measurement would be falsified as noise.
- If higher-affinity organisms stopped out-reproducing B at higher metabolism
  (fitness reversal), the selection mechanism claim would be falsified.

---

## Standing integrity rules

- Every result in this repo is labeled OBSERVED / COMPUTED / INFERRED /
  HYPOTHESIZED / SPECULATIVE in the PR that produced it.
- Negative results are merged and kept (PR#11).
- Multi-seed statistics are required before any finding claim; single runs
  are always labeled calibration.
- No claims of consciousness, sentience, personhood, life, identity
  persistence, or open-endedness anywhere in the repository.
