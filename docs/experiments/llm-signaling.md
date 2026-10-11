# LLM Follower in the Signaling World — Multi-Subject Cognitive Run

Status: OBSERVED (n=3 seeds, engineered rule emitter; small-n caveat)
Date: 2026-10-11
Harness: `run-signaling --backend <spec>` (one cognitive follower replacing
the rule followers; rule emitter unchanged)

## HYPOTHESIS

An LLM subject placed in the signaling world alongside a rule emitter can
(a) survive the ring economics, (b) perceive the signal-gradient channels
added in PR#55, and (c) choose actions that differ measurably from the
rule followers' engineered policy — giving the signaling-convention
research line its first cognitive participant.

## DESIGN

- World: network ring (16 nodes), rule emitter SIGNALs strength 1.0 every
  3 ticks (unchanged), signal pull 0.5; one LLM follower with a
  survival-budget start energy (40 × 3 = 120 units).
- Follower cognition: granite4.1:3b-q4_K_M, temp 0.3, MEMORY_LIMIT 8,
  outcome-visible memory, WABIX-validated decisions, fail-safe errors.
- The follower observes the SAME channels as rule followers:
  `node_energy`, `nearby_signal_0`, `nearby_signal_left`,
  `nearby_signal_right`, `degree` — with no semantic labels beyond the
  channel names (no "food", no "survive").
- New instrumentation: `LlmSignalingResult.signal_strengths` histogram —
  distinguishes real broadcasts (strength > 0) from null-signals
  (strength 0, cost 0).

## OBSERVED (n=3 seeds, 600 ticks, 1 cognitive follower)

| seed | SIGNAL params chosen | final energy | proximity | alive |
|------|---------------------|--------------|-----------|-------|
| 42   | 0.0 ×599, 0.5 ×1    | 80.79        | 0.000     | true  |
| 7    | 0.0 ×600            | 90.00        | 0.000     | true  |
| 99   | 0.0 ×477, 0.2 ×122, 0.5 ×1 | 70.42 | 1.000     | true  |

## INTERPRETATION (INFERENCES; not claims about conventions)

- **The null-signal discovery**: in 2/3 seeds the LLM selects SIGNAL with
  strength 0.0 for (nearly) every decision — a zero-cost "broadcast of
  nothing". This is not a leak (the prompt contains no signaling
  semantics); it is the LLM exploiting the vocabulary's cheapest safe
  action. 3/3 seeds survive; the rule followers died in the default
  budget and could only co-reside 2/6 under gradient steering.
- **Seed 99 sent real signals**: strength 0.2 on 122 ticks AND converged
  on the emitter (proximity 1.000) — the only seed where the follower
  both broadcast with nonzero strength and used the gradient to settle at
  the beacon. With one follower there is no scramble, so it survives
  comfortably.
- **Cognitive follower vs rule follower**: the rule follower's policy is
  reactive (ABSORB at a signaling node, HOP up-gradient); the LLM adds an
  emitter-like mode (choosing to SIGNAL itself). The signaling world now
  has subjects that can BOTH listen and broadcast — the structural
  precondition for a convention to form across subjects.
- n=3 is small; seeds are homogeneous in the LLM's preference for the
  zero-cost action. More seeds and a stronger payoff for real signals are
  the natural next step (e.g. signal-attracts-pull already exists; a
  listener-pays-attention bonus is NOT added — that would leak).

## CAVEATS

- Engineered emitter; one cognitive subject per run (no follower-follower
  interaction yet).
- No claim that the null-signal is "meaning" — it is a measured behavior
  with a zero resource footprint; convention formation remains unclaimed.
- The `signal_strengths` histogram rounds to 1 decimal; sub-0.05
  strengths read as 0.0.
