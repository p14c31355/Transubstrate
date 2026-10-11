# Memory Ablation — Causal Carry-vs-Wipe at the Migration Boundary

Status: OBSERVED (rule calibration n=2; LLM same-trajectory sweep n=5 with
pre-phase trajectories EXACTLY matched; earlier confounded sweep superseded)
Date: 2026-10-11
Harness: `run-migration` / `sweep-world-migration` with the per-seed
ablation arm (memory ablation arc, PR#52 + same-trajectory refinement)

## HYPOTHESIS

The subject's carried memory causally contributes to post-migration
continuity: wiping the buffer at the boundary should reduce memory_usage
and memory_retention relative to carrying it, and may shift behavior.
This upgrades the correlational memory_usage reading (PR#42/#44/#46) to a
causal test: same seed, same world, same backend pair — the only
difference is whether the buffer survives the boundary.

## DESIGN

- `memory_carry: bool` in both migration configs (default true = the
  SUBJECT != MODEL design).
- `memory_carry = false` wipes the buffer at the migration boundary; the
  post-swap cognition starts amnesic.
- **Same-trajectory refinement**: the ablation arm REPLAYS the carry
  arm's pre-swap decisions verbatim (recorded in the carry run's event
  log, fed back via `run_world_migration_with_replay`), so both arms share
  a bit-identical pre-phase trajectory. The carry − wipe contrast then
  isolates the causal effect of the carried memory — LLM sampling noise
  can no longer disguise itself as a pre-phase trajectory difference
  (this removes the confound disclosed in the first version of this
  record).
- `ablation_effect = carry − wipe` per dimension (positive = the carried
  memory helped that dimension).

## CALIBRATION (rule backends — EXPECTED true negative)

rule→echo, seed 42: ablation effect 0.0 across ALL dimensions (behavioral,
memory_retention, memory_usage). Rule backends do not read the memory
buffer, so wiping it cannot change anything — the detector shows no
artifact when there is nothing to detect.

## OBSERVED (LLM: granite4.1:3b-q4_K_M, temp 0.3, n=5 seeds, same-trajectory)

Aggregate ablation_effect (carry − wipe), mean ± population-sd:

| dimension | effect |
|-----------|--------|
| behavioral_action_distribution | +0.680 ± 0.342 |
| memory_retention | +0.600 ± 0.490 |
| memory_usage | +1.000 ± 0.000 |

Carry-run continuity for the same seeds: behavioral 0.680 ± 0.342,
memory_retention 0.600 ± 0.490, memory_usage 1.000 ± 0.000.

Per-seed pre-phase match: **5/5 seeds — carry pre == wipe pre exactly**
(seed 42 previously diverged under the confounded design; now identical).
Post-swap action distributions:

| seed | carry post | wipe post |
|------|-----------|-----------|
| 42   | {WAIT 40} | {SIGNAL 40} |
| 7    | {ABSORB 12, SIGNAL 28} | {SIGNAL 40} |
| 99   | {ABSORB 9, SIGNAL 31} | {SIGNAL 40} |
| 123  | {WAIT 40} | {SIGNAL 40} |
| 2026 | {WAIT 40} | {SIGNAL 40} |

## INTERPRETATION (INFERENCES, now confound-free on the pre phase)

- memory_usage +1.000 ± 0.000: with the pre trajectory fixed, every
  boundary-remembered action is re-enacted in the carry arm and none can
  be in the wipe arm — the dimension is saturated, as expected by
  definition.
- behavioral +0.680: the carry arm CONTINUES the boundary strategy (WAIT
  continuation 3/5, ABSORB→SIGNAL mixture 2/5), while the wipe arm jumps
  to uniform SIGNAL saturation 5/5. Carried memory causally shapes the
  post-swap strategy: amnesic cognition abandons the pre-swap policy and
  defaults to blind SIGNAL exploration.
- memory_retention +0.600: the carry arm's buffer keeps (action, reward)
  keys that match the boundary snapshot; the wipe arm starts from nothing.
  High variance (±0.49) reflects the two carry sub-strategies (pure WAIT
  keeps no reward-bearing keys; ABSORB/SIGNAL learners do).

## CAVEATS

- Post-swap LLM sampling noise remains (each decision is an independent
  sample at temp 0.3); n=5 seeds bound its influence — the wipe arm's
  SIGNAL saturation is uniform 5/5, so the qualitative contrast is stable.
- MEMORY_LIMIT=8 FIFO: the boundary buffer holds only the last 8
  decisions.
- Not claimed: that memory retention is necessary or sufficient for
  subject survival or "identity". The claim is limited to measured
  continuity dimensions.
