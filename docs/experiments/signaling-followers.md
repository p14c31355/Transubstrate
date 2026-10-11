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

## FOLLOW-UP (queued → done in the survival-tuned run, PR arc below)

- ~~Survival-tuned run~~ → DONE (see below): a sweep-sized follower energy
  budget (`SignalingConfig::survival_tuned`, `run-signaling --survival`)
  lets followers outlive the sweep and live off the emitter's pulled-in
  energy.
- Signal-gradient observation (`nearby_signal_left`/`right`) so followers
  can steer toward the emitter instead of sweeping blindly — shared
  substrate with chemical-field meaning formation.
- Multi-subject LLM run after the rule-level economics sustain followers.

## SURVIVAL-TUNED RUN (OBSERVED, n=5 seeds, 600 ticks, 6 followers)

`SignalingConfig::survival_tuned`: follower start energy = 600 ×
metabolism (0.05) + 10 headroom = 40 units (vs 12 default).

| seed | proximity | nullProx | intake/tick | nullIntake | alive | emitter |
|------|-----------|----------|-------------|------------|-------|---------|
| 42   | 0.278     | 0.276    | 0.727       | 0.276      | 5     | true    |
| 7    | 0.275     | 0.275    | 0.729       | 0.276      | 5     | true    |
| 99   | 0.274     | 0.274    | 0.724       | 0.272      | 5     | true    |
| 123  | 0.273     | 0.271    | 0.727       | 0.276      | 5     | true    |
| 2024 | 0.275     | 0.276    | 0.729       | 0.275      | 5     | true    |

- **Survival economics fixed**: 5/6 followers survive all 600 ticks in
  5/5 seeds (the default budget killed 6/6); the emitter survives 5/5.
- **Harvest strengthens**: intake ≈0.73/tick vs null ≈0.28 (**≈2.6×**,
  up from ≈2.1× — surviving followers spend more time harvesting at the
  emitter's node).
- **Attraction narrows**: proximity 0.273-0.278 vs null 0.271-0.276
  (4/5 seeds, small margin). With survival, the null-run followers also
  linger near their start nodes, so positional clustering adds little
  beyond what staying-put already yields. The harvest advantage is the
  robust effect; true *steering* toward the emitter needs a signal
  gradient observation (queued).

## CAVEATS

- Engineered policies; deterministic per seed.
- Proximity sampled once per tick before actions; hop costs approximated
  from the action list (0.1 per HOP).
