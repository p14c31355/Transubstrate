# Selection vs Drift — and the Mutation Ratchet

Status: OBSERVED (n=5 seeds for the drift replication) + ENGINEERING
CLAIMS (the clamp bug and the regime trade-off)
Date: 2026-10-11
Harness: post-hoc analysis of BirthRecorded logs (PR#71) + the
`Genome::mutate` fix + probe-sweep reruns

## QUESTION (from PR#71)

The division threshold drifted UPWARD 6.0 → 7.4-9.9 in 5/5 seeds. Is
that selection (high-threshold parents leave more surviving offspring)
or drift (or something else)?

## FITNESS MEASUREMENTS: NO INDIVIDUAL-LEVEL ADVANTAGE (n=1 seed
analyzed in depth, seed 42; n=5 for the drift itself)

Using the BirthRecorded logs:

1. **threshold vs lifetime birth count**: correlation −0.106.
   Low-threshold parents reproduce MORE (≈6: 3.53 births; ≈7: 2.80;
   ≈9: 2.50). If anything, the raw fitness signal points the WRONG way.
2. **threshold vs lifetime**: −0.105, but CONFOUNDED by birth cohort
   (late generations are born into the starvation phase and die young
   regardless of genotype).
3. **Within-family sibling comparison** (483 dying sibling pairs with
   differing thresholds, same parent, near-same birth tick): the
   higher-threshold sibling outlived the lower in 244/480 = **0.508** —
   a coin flip. Threshold has NO detectable effect on survival.

## ROOT CAUSE FOUND: THE MUTATION RATCHET

While re-reading `Genome::mutate` to check the jitter band:

```rust
let threshold_lo = 20.0f32.min(self.division_threshold);
division_threshold: jitter(..., threshold_lo, 100.0),
```

The PR#65 mutation-floor fix (`min(20, parent_value)`) was intended to
let LOW-threshold lineages persist. But at a 6.0 founder it clamps the
lower mutation bound AT THE PARENT'S VALUE: **downward mutation is
structurally impossible**, upward is free. The 5/5-seed directional
drift was neither selection nor drift — it was a mechanical ratchet
built into the mutation operator. (The consistency across seeds that
PR#71 read as "hard to explain by neutral drift" was, in fact,
explained by having no randomness in the downward direction at all.)

**Fixed**: the mutation band is now FIXED (3.0..=100.0) — symmetric
proposal, ecological filtering. Sub-6.0 thresholds fall into the
PR#65 collapse regime and die THERE if they cannot sustain themselves.
Mutation proposes; selection disposes.

## VERIFICATION (post-fix, 100k ticks)

- probe 1.0 (turnover): births now span 3.0-8.42, **138/248 below
  6.0** — the clamp is gone. Summary counters are unchanged
  (17/239/248) because under probe 1.0 the rule proposes DIVIDE at
  energy 10, which passes EVERY genome threshold in the mutated range:
  **the genome is decorrelated from behavior in this regime**.
- probe 0.4 (genomes gate division): 8 births at t=0 then freeze
  (PR#69 regime). The gen-1 thresholds now spread SYMMETRICALLY
  (5.17-6.88) — downward mutants are born. But equilibrium energy
  (~4.4) is below every child's threshold (5.17+), so the freeze
  persists: no generations, no drift possible.

## THE STRUCTURAL TRADE-OFF (open design problem)

The two regimes cannot currently coexist:
- probe 1.0: generations accumulate, but the genome is decoration
  (every proposal passes).
- probe 0.4: the genome gates division, but births freeze (children
  inherit ~half the parent's energy, land at ~3, and the equilibrium
  ~4.4 never reaches their 5.2+ thresholds).

Resolving this needs a design change — e.g. birth stake independent of
parent energy at division, or a reproduction cost curve — which is a
next-PR decision, not a bug fix.

## INTERPRETATION (correcting PR#71)

- PR#71's "directional drift across 5/5 seeds — hard to explain by
  neutral drift" is WITHDRAWN as a selection signal: it was the clamp.
- The corrected statement: under the pre-fix operator, no conclusion
  about selection was possible; under the fixed operator, individual
  fitness shows NO threshold effect (survival coin-flip, negative
  birth-count correlation), and the two live regimes either suppress
  generations or decorrelate the genome. **Whether selection acts on
  this gene in this world remains OPEN and now has the right
  instrumentation to answer.**
- Methodological lesson recorded for the project: before reading a
  directional trend across generations as evolution, verify the
  MUTATION OPERATOR's symmetry. A one-sided mutation band fabricates
  exactly the signature of adaptive evolution.

## CAVEATS

- Fitness measurements in depth on seed 42 only; the drift itself is
  5-seed.
- The sibling test has low power for small lifetime differences.
- The trade-off resolution (birth-stake redesign) may change the
  ecology again.

## NEXT

- Birth-stake redesign so generations accumulate AND genomes gate
  division — the missing regime where selection can be measured.
- Then re-run the drift experiment on the fixed operator.
