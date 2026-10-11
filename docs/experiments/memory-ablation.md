# Memory Ablation — Causal Carry-vs-Wipe at the Migration Boundary

Status: OBSERVED (rule calibration n=2; LLM sweep n=5 — mixed with a
disclosed confound, see CAVEATS)
Date: 2026-10-11
Harness: `run-migration` / `sweep-world-migration` with the per-seed
ablation arm

## HYPOTHESIS

The subject's carried memory causally contributes to post-migration
continuity: wiping the buffer at the boundary should reduce memory_usage
and memory_retention relative to carrying it, and may shift behavior.
This upgrades the correlational memory_usage reading (PR#42/#44/#46) to a
causal test: same seed, same world, same backend pair — the intended only
difference is whether the buffer survives the boundary.

## DESIGN

- `memory_carry: bool` in both migration configs (default true = the
  SUBJECT != MODEL design).
- `memory_carry = false` wipes the buffer at the migration boundary; the
  post-swap cognition starts amnesic.
- The experiment runners execute the carry run AND (when carrying) a
  second, identical migration with the wipe — own experiment id and event
  log per arm (events.jsonl / events.ablation.jsonl / events.control.jsonl).
- `ablation_effect = carry − wipe` per dimension (positive = the carried
  memory helped that dimension).

## CALIBRATION (rule backends — EXPECTED true negative)

rule→echo, seed 42: ablation effect 0.0 across ALL dimensions (behavioral,
memory_retention, memory_usage). Rule backends do not read the memory
buffer, so wiping it cannot change anything — the detector shows no
artifact when there is nothing to detect.

## OBSERVED (LLM: granite4.1:3b-q4_K_M, temp 0.3, n=5 seeds)

Aggregate ablation_effect (carry − wipe), mean ± population-sd:

| dimension | effect |
|-----------|--------|
| behavioral_action_distribution | +0.500 ± 0.397 |
| memory_retention | +0.550 ± 0.458 |
| memory_usage | +0.800 ± 0.400 |

Carry-run continuity for the same seeds: behavioral 0.700 ± 0.313,
memory_retention 0.600 ± 0.490, memory_usage 1.000 ± 0.000.

Per-seed action distributions (40 decisions/phase):

| seed | carry pre → post | wipe pre → post |
|------|------------------|-----------------|
| 42   | {ABSORB 2, WAIT 38} → {WAIT 40} | {SIGNAL 40} → {SIGNAL 2} |
| 7    | {ABSORB 2, WAIT 38} → {WAIT 40} | {ABSORB 2, WAIT 38} → {SIGNAL 40} |
| 99   | {ABSORB 2, WAIT 38} → {WAIT 40} | {ABSORB 2, WAIT 38} → {SIGNAL 40} |
| 123  | {ABSORB 38, WAIT 2} → {ABSORB 9, SIGNAL 31} | {ABSORB 2, WAIT 38} → {SIGNAL 40} |
| 2026 | {ABSORB 38, WAIT 2} → {ABSORB 17, SIGNAL 23} | {ABSORB 2, WAIT 38} → {SIGNAL 40} |

## INTERPRETATION (INFERENCES, not established causation)

- memory_usage +0.8: largely definitional — a wiped buffer cannot be
  re-enacted. The informative part is that carry runs sit at 1.0 while
  wipe runs re-enact NONE of the boundary memory.
- behavioral +0.5: carry runs tend to CONTINUE the boundary strategy
  (WAIT-continuation in 3/5, ABSORB/SIGNAL continuation in 2/5), while
  wipe runs frequently jump to a different post-swap behavior (full
  SIGNAL flip in 4/5 wipe arms). Carried memory is associated with
  behavioral persistence across the swap.
- The outcome-learning seeds (123, 2026: ABSORB→SIGNAL learning WITH
  memory) still learned in the carry arm, while their wipe arms landed on
  immediate SIGNAL saturation — consistent with memory carrying the
  reward history that shapes gradual learning vs blind exploration.

## CAVEATS

- **Pre-phase confound (disclosed)**: at temp 0.3 the LLM samples
  independently per decision, so the PRE phase of a wipe arm does not
  always reproduce the carry arm's pre phase (seed 42: wipe pre = SIGNAL
  40 vs carry pre = {ABSORB 2, WAIT 38}). The carry−wipe contrast
  therefore mixes memory causation with pre-phase trajectory differences.
  A same-trajectory ablation (wipe the buffer but replay the carry arm's
  pre decisions) would isolate memory cleanly — queued.
- MEMORY_LIMIT=8 FIFO: the boundary buffer holds only the last 8
  decisions.
- n=5 seeds with high variance (±0.4-0.46): the aggregate effects are
  directionally consistent but individual seeds differ widely.
- Not claimed: that memory retention is necessary or sufficient for
  subject survival or "identity". The claim is limited to measured
  continuity dimensions.
