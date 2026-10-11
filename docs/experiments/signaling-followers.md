# Signaling-Follower Detection — Multi-Subject Record

Status: OBSERVED (n=5 seeds, engineered follower policy; mixed positive /
negative result, both disclosed)
Date: 2026-10-11
Mechanism: signal pull (PR#30–#33); harness `run-signaling` (this PR)

## HYPOTHESIS

With signal-driven energy pull active, followers on the ring are attracted
to the emitter's signal and harvest more energy near it than in a no-pull
null run — i.e. the pull mechanism creates the follower-payoff substrate
that a signaling convention would need.

## OPERATIONAL DEFINITIONS (fixed before running)

- "signal subject": one designated emitter SIGNALing (strength 1.0) every
  3 ticks on its node; it never ABSORBs.
- "follower": 6 independent subjects with a fixed engineered policy —
  ABSORB while the current node is signaling, HOP forward with spare
  energy to sweep the ring, WAIT otherwise (channel-name rule, same class
  as signal-seeker).
- "attraction": mean fraction of followers on/adjacent to the emitter's
  node vs a null run with `signal_pull_rate = 0` (same seeds, same policy).
- "harvest": mean follower energy intake per tick vs null.

## OBSERVED RESULT (`run-signaling`, 600 ticks, 6 followers, 5 seeds)

| seed | proximity | nullProx | intake/tick | nullIntake | alive | emitter |
|------|-----------|----------|-------------|------------|-------|---------|
| 42   | 0.145     | 0.104    | 0.358       | 0.171      | 0     | false   |
| 7    | 0.151     | 0.102    | 0.359       | 0.172      | 0     | false   |
| 99   | 0.142     | 0.116    | 0.354       | 0.168      | 0     | false   |
| 123  | 0.145     | 0.102    | 0.355       | 0.172      | 0     | false   |
| 2024 | 0.141     | 0.111    | 0.360       | 0.171      | 0     | false   |

Attraction wins 5/5 seeds; intake wins 5/5 seeds (≈2.1× the null intake).

## INTERPRETATION

- POSITIVE: the pull mechanism creates a real, seed-stable follower payoff
  — followers cluster near the emitter (≈1.4× null proximity) and harvest
  ≈2.1× the null intake. The candidate signaling-convention substrate
  (emitter → cue → follower payoff) exists and is measurable.
- NEGATIVE (survival economics): all followers and the emitter die within
  the 600-tick window. Follower start energy (12) plus swept HOP costs
  (0.1/tick) and metabolism (0.05/tick) exceed what pull-gathered energy
  returns before the follower reaches the emitter. The payoff is real but
  not self-sustaining under these parameters.
- Not an emergence claim: the follower policy is engineered; what is
  measured is that the MECHANISM supplies the payoff a convention would
  exploit.

## FOLLOW-UP (queued)

- Survival-tuned run: higher follower start energy, fewer HOPs (larger
  emitter signal radius or stronger pull), or follower-side harvesting at
  pull-concentrated neighbor nodes, so followers survive past the first
  circuit of the ring.
- Multi-subject LLM run after the rule-level economics sustain followers.

## CAVEATS

- Engineered policies; deterministic per seed.
- Proximity sampled once per tick before actions; hop costs approximated
  from the action list (0.1 per HOP).
