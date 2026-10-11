# Convention Formation Across Cognitive Subjects — First Attempt

Status: OBSERVED NEGATIVE (n=1 seed, 2 LLM followers; disclosed)
Date: 2026-10-11
Harness: `run-signaling --backend <spec> --backend <spec>` (N cognitive
followers via `run_cognitive_signaling`)

## HYPOTHESIS

Two LLM subjects in one signaling world, both able to observe the rule
emitter's signal AND each other's signals, would begin responding to a
signal source — the structural precondition (listen + broadcast, PR#56)
becoming an actual interaction.

## DESIGN

- Network ring (16 nodes), rule emitter SIGNALs strength 1.0 every 3
  ticks, signal pull 0.5.
- TWO cognitive followers (granite4.1:3b-q4_K_M, temp 0.3), each with its
  own bounded memory (FIFO 8), outcome-visible, WABIX-validated
  decisions, gradient channels (`nearby_signal_left`/`right`).
- Detection metric: per-subject action distribution, SIGNAL strength
  histogram, mean proximity to the emitter, survival.

## OBSERVED (seed 42, 600 ticks, 2 cognitive followers)

| subject | SIGNAL params | proximity | final energy | alive |
|---------|--------------|-----------|--------------|-------|
| cog-00  | 0.0 ×600     | 0.000     | 90.00        | true  |
| cog-01  | 0.0 ×600     | 0.000     | 90.00        | true  |

## INTERPRETATION (NEGATIVE, disclosed)

- Both LLM followers independently converged on the **null-signal
  strategy** (SIGNAL strength 0.0 for every tick — the same behavior the
  single-follower run found dominant). With zero strength, neither
  subject's signal is observable to the other: the subjects exchange
  invisible signals and effectively do not interact.
- Neither subject responded to the RULE emitter's strong signal either —
  proximity stayed 0.000 (contrast: the engineered rule followers
  converge at proximity 0.98+ under the same channels, PR#55; the single
  LLM follower converged in 1/3 seeds, PR#56). At temp 0.3 the LLM's
  dominant mode is the zero-cost safe action, not exploration.
- **Conclusion**: convention formation is NOT observed in this
  configuration. The necessary structure exists (subjects that can listen
  and broadcast), but the behavior does not: nothing makes a real signal
  worth sending or a received signal worth answering — and per the
  leakage rule we do not add such a payoff to the prompt.

## WHAT WOULD HAVE TO CHANGE (HYPOTHESES, not implemented)

- Higher sampling temperature (0.7 showed outcome-learning in migration
  runs) to escape the null-signal attractor.
- More seeds: 1/3 single-follower seeds did broadcast and converge; the
  population-level picture at n=3 seeds × 2 subjects is unmeasured.
- A world where signaling has a visible consequence to LISTENERS (the
  pull already concentrates energy at the beacon; listeners must still
  choose to go there — the rule followers do, the LLM at temp 0.3 does
  not).

## CAVEATS

- n=1 seed for the two-subject run; deterministic per seed but the LLM is
  stochastic at temp 0.3.
- No claim that conventions cannot form — only that they did not form
  here, with this model, at this temperature, in this world.
