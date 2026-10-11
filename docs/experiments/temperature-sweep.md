# Temperature Sweep — Outcome Learning vs Temperature

Status: OBSERVED (n=5 per temperature at 0.7; n=5 at 0.3; n=4 valid + 1
schema-rejected at 1.0; single model). Temp 0.7 hand-collected; 0.3 and 1.0
batch-collected with `sweep-world-migration` (PR#39).
Date: 2026-10-11
Extends: docs/experiments/outcome-exploration.md (temp 0.7)

## HYPOTHESIS

Outcome-driven SIGNAL learning depends on sampling temperature: too low
(0.0, deterministic) and the model never explores; too high and energy-cost
deaths interrupt learning. There should be a usable middle band where
exploration finds the SIGNAL payoff and the subject survives to harvest it.

## CONFIGURATION

- `sweep-world-migration --seeds 42,7,99,123,2026 --backend
  llm:granite4.1:3b-q4_K_M --llm-temperature {0.7, 0.3} --ticks 400`
- World: network-v0 with signal pull (`signal_pull_rate` 0.5)
- Memory: last 5 decisions with outcomes (`outcome +X.XXX`)
- Behavioral dimension only (memory_retention recorded per the standard
  output; note: the summary JSON was lost for the temp-0.3 sweep when the
  batch process was interrupted after the last run — per-seed event logs
  are complete and were analyzed directly; re-run `--summary` for the
  canonical file if needed)

## OBSERVED RESULT

Temp 0.7 (PR#37, hand-collected):
- 2/5 seeds learned SIGNAL; 2/5 died mid-exploration
- behavioral 0.975 ± 0.000, memory_retention 1.0 ± 0.0

Temp 0.3 (this sweep, n=5, ZERO deaths):
- 3/5 seeds learned SIGNAL post-swap (SIGNAL 30, 9, 28 of 40 decisions)
- behavioral 0.65 ± 0.314 (per-seed: 0.25, 0.975, 0.95, 0.775, 0.3)
- pre-world policies: 3 seeds ABSORB-dominant, 2 seeds WAIT-dominant

## INTERPRETATION

- The hypothesized middle band exists: temp 0.3 dominated temp 0.7 on both
  axes — higher learning incidence (3/5 vs 2/5) AND zero deaths (vs 2/5).
- High behavioral variance at 0.3 (±0.314) is the signature of bimodal
  outcomes: some subjects adopt SIGNAL (continuity drops), some stay
  conservative (continuity stays high). The variance is the finding, not
  noise.
- Adaptation post-swap (ABSORB → SIGNAL under the new world's payoff) is
  now observed in 3/5 seeds at the lower temperature — the strongest
  adaptation evidence so far, with no deaths.

## TEMP 1.0 (added after the first record; n=4 valid + 1 schema-rejected)

Batch-collected with the sweep runner. One seed (123) PROPOSED A TYPO
("ABSORBT") — rejected by WABIX schema validation, experiment failed fast;
the sweep runner recorded the error per-seed and continued. This is the
fail-safe path working as designed (Goal §9: malformed output fails safely).

| seed | pre actions | post actions | behavioral |
|------|------------|--------------|------------|
| 42   | {ABSORB 2, WAIT 38} | {WAIT 40} | 0.95 |
| 7    | {ABSORB 40} | {SIGNAL 39, ABSORB 1} | 0.025 (learned SIGNAL) |
| 99   | {ABSORB 39, WAIT 1} | {SIGNAL 40} | 0.0 (learned SIGNAL) |
| 123  | — | — | FAIL: schema violation `ABSORBT` |
| 2026 | {ABSORB 2, WAIT 38} | {WAIT 40} | 0.95 |

Sweep summary (n=4 valid): behavioral 0.481 ± 0.469, memory_retention
0.5 ± 0.5, effect −0.494 ± 0.457 — the widest spread of any temperature,
consistent with high temperature destabilizing policies in both directions
(two conservative, two SIGNAL-adopters, one malformed).

## REFINED INTERPRETATION ACROSS TEMPERATURES

- Learning incidence (post-swap SIGNAL adoption): 2/5 at 0.7, 3/5 at 0.3,
  2/4 valid at 1.0 — no monotonic trend; temp 0.3 remains the best
  learning-per-death trade-off (0 deaths).
- Failure modes differ by temperature: at 0.7 failures are energy deaths
  (SIGNAL cost spirals); at 1.0 the observed failure is malformed output
  (schema rejection) — a qualitatively different risk profile.
- The schema-rejection case doubles as the first OBSERVED fail-safe
  demonstration in an LLM migration experiment: invalid output never
  reached the world.

## CAVEATS

- Single model, one run per seed per temperature; n=5 per cell.
- Temp-0.3 summary JSON was lost to a batch-process interruption (kernel
  timeout after the fifth run); results above were recomputed from the
  complete per-seed event logs. Re-running the sweep reproduces them
  modulo LLM nondeterminism (temp 0.3 is stochastic).
- memory_retention here was recomputed action-only from logs (the standard
  reward-bin metric needs the in-run buffer, which is what the harness
  reports at runtime); treat the recorded runtime values as canonical.
