# Signal Pull — Network World Experiment Record

Status: OBSERVED (mechanism + end-to-end agent verification, single seed)
Date: 2026-10-11

## HYPOTHESIS

The network world's default economics made WAIT optimal (nodes regenerate
faster than a lone subject consumes), so SIGNAL/HOP had no payoff and
world-migration adaptation was unmeasurable. Adding signal-driven energy
pull — a signaling node draws `signal_pull_rate × strength` energy per tick
from its ring neighbors — gives SIGNAL a resource payoff and HOP a reason to
chase signal-rich nodes. With that payoff, a signal-seeking controller should
change behavior across the world boundary (SIGNAL in the pull-less world,
ABSORB after the pull concentrates energy).

## CONFIGURATION

- Network world: 16-node ring, `signal_pull_rate` 0.5 (new config field,
  default), pull before field decay, energy-conserving
- Controller: `signal-seeker` (rule-signal-seeker-v0) — survive (ABSORB when
  local_energy low) → seed the signal (when nearby_signal_0 is quiet) →
  harvest (absorb concentrated energy)
- Ticks: 400, migration at tick 200, seed 42

## OBSERVED RESULT

Signal-seeker survives both worlds with zero deaths:

- pre (primordial, no pull): {SIGNAL 35, ABSORB 5}
- post (network, pull active): {ABSORB 40}

The subject flips from signal-seeding to harvesting exactly when it enters
the world where signaling pulls energy. behavioral continuity 0.125,
memory_retention 0.25, effect vs control −0.625/−0.75.

Mechanism tests (unit):
- pull concentrates energy on the signaling node
- energy is conserved (modulo uniform regen)
- pull stops after field decay

## INTERPRETATION

The signal pull closes the loop SIGNAL → resource concentration → ABSORB
payoff. This gives the world-migration experiment a decision surface where
the correct behavior differs per world, making adaptation measurable.
It also creates a candidate signaling-convention substrate: a signal now
broadcasts "resources concentrating here" — whether subjects exploit or
flock to other subjects' signals is exactly the signaling-convention
question from the Goal, now testable.

## CAVEATS

- Single seed for the end-to-end verification; mechanism is unit-tested.
- The pull is an engineered payoff (experimental variable
  `signal_pull_rate`, default 0.5), not an emergent property. Any
  "signaling convention" observed under this economics must be reported as
  such, not as spontaneous emergence.
- Primordial world deliberately has no pull; the asymmetry is the
  experimental variable.
