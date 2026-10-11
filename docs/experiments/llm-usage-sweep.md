# LLM World Migration — 5-Dimension Sweep (temp 0.3)

Status: OBSERVED (n=5, temperature 0.3, single model; batch-collected with
`sweep-world-migration`)
Date: 2026-10-11
Extends: temperature-sweep.md (which recorded the 2-dimension vector)

## PURPOSE

Re-run the temp-0.3 cell with the five-dimension continuity vector to
record the first LLM `memory_usage` numbers (rule/echo calibrations only
existed before).

## CONFIGURATION

- `sweep-world-migration --seeds 42,7,99,123,2026 --backend
  llm:granite4.1:3b-q4_K_M --llm-temperature 0.3 --ticks 400`
- World: primordial → network (signal pull active in the destination)
- Dimensions: behavioral, memory_retention, memory_usage

## OBSERVED RESULT

| seed | behavioral | memory_retention | memory_usage | post actions |
|------|-----------|------------------|--------------|--------------|
| 42   | 0.950     | 1.000            | 1.000        | {WAIT 40} |
| 7    | 0.950     | 1.000            | 1.000        | {WAIT 40} |
| 99   | 0.975     | 1.000            | 1.000        | {WAIT 40} |
| 123  | 0.000     | 0.000            | 0.000        | {SIGNAL 40} |
| 2026 | 0.950     | 1.000            | 1.000        | {WAIT 40} |

Means ± sd: behavioral 0.765 ± 0.374, memory_retention 0.8 ± 0.4,
memory_usage 0.8 ± 0.4. Effect: −0.195 ± 0.378 / −0.2 ± 0.4 / −0.2 ± 0.4.

## INTERPRETATION

- **First LLM memory_usage numbers**: 4/5 seeds scored 1.0 — the subject
  re-enacted every action it remembered at the boundary (WAIT) in the new
  world. Full usage consistent with the swap-robust fingerprint.
- **Seed 123 is the discriminating case**: pre-world ABSORB-dominant
  (40/40) → post-world SIGNAL-dominant (40/40), with memory_retention 0.0
  AND memory_usage 0.0. The subject adopted a strategy that used NONE of
  its boundary memory — full strategy replacement, not adaptation of the
  remembered policy. This is exactly the signal→pull→reward learning loop
  from PR#37, now visible in the vector as a coordinated 0.0/0.0 pattern.
- The three dimensions agree (0.95/1.0/1.0 vs 0.0/0.0/0.0): the bimodality
  first seen at temp 0.3 in temperature-sweep.md reproduces under the full
  vector, and the two memory dimensions move together with behavioral.
- Effect ≈ −0.2 ± 0.4: on average the swap costs little, but the per-seed
  spread (0 to −0.5-range) is where the strategy-replacement cases live.

## CAVEATS

- Single model, temperature 0.3, one run per seed; LLM sampling is
  stochastic at temp > 0 (seed 123's SIGNAL adoption differs from the
  earlier temp-0.3 record's seed-123 result — same seed, different
  sampling path).
- Action-name granularity: memory_usage cannot distinguish re-enacting
  WAIT for the remembered reason vs by coincidence.
