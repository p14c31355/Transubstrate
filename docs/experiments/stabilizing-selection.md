# Stabilizing Selection on Division Threshold — First Measured Fitness Landscape

Status: OBSERVED (n=5 seeds, 1240 birth genomes aggregate)
Date: 2026-10-11
Harness: founder threshold 8.0 + `--divide-probe 1.0` (turnover regime)
+ BirthRecorded logs + post-hoc fitness aggregation

## QUESTION (chain: PR#71 → PR#72 → this)

PR#72 fixed the mutation ratchet (symmetric 3..100 band) but found a
regime trade-off: turnover (probe 1.0) decorrelates the genome from
behavior because DIVIDE is proposed at energy 10 and every threshold in
the mutated range passes. This PR asks: does that decorrelation HOLD
when thresholds reach the energy cap — and does a selection signal
appear?

## DESIGN

Founder genome division_threshold = 8.0 (near the energy cap of 10).
Symmetric mutation spreads children across ~3-13. Under probe 1.0:
- thresholds BELOW ~10 reproduce (proposals pass),
- thresholds ABOVE 10 can NEVER reproduce (energy cap),
- very low thresholds fall into the PR#65 collapse regime.
The energy cap makes the upper tail sterile — a measurable selection
edge. 5 seeds × 100k ticks, 248 births each.

## OBSERVED: REPRODUCTION RATE BY THRESHOLD (1240 birth genomes, 5 seeds)

| threshold bin | individuals | became parent | reproduction rate |
|---------------|-------------|---------------|-------------------|
| 3             | 10          | 3             | 0.300             |
| 4             | 2           | 1             | 0.500             |
| 5             | 40          | 7             | 0.175             |
| 6             | 122         | 32            | 0.262             |
| 7             | 356         | 124           | 0.348             |
| **8**         | **441**     | **171**       | **0.388**         |
| 9             | 233         | 67            | 0.288             |
| 10            | 66          | 21            | 0.318             |
| 11+           | 10          | **0**         | **0.000**         |

**A humped fitness landscape peaking at the founder value (8)**:

- threshold 11+: ZERO reproduction — the hard sterility edge at the
  energy cap (10 of them born, none ever divided). Selection is
  absolute there.
- thresholds 9-10: depressed rates (0.288/0.318) — the soft edge.
- threshold 5: depressed (0.175) — the low-side edge (the PR#65
  collapse regime prunes fast-splitting, thin-stake lineages).
- The peak sits AT the founder value: 64% of all birth genomes
  (797/1240) fall in bins 7-8.

This is the signature of **STABILIZING SELECTION**: both tails are
penalized, the middle is favored. It contrasts directly with PR#72's
sibling test (coin flip) — that measured one equilibrium window; this
aggregates the full 12-15 generation runs including the selection
edges.

## INTERPRETATION (OBSERVED → first INFERENCES)

1. The turnover-vs-genome-efficacy trade-off (PR#72) is PARTIALLY
   resolved: probe 1.0 does NOT fully decorrelate the genome — the
   decorrelation holds only while thresholds stay below the energy
   cap. Founder 8.0 puts the population at the cap's doorstep, where
   the genome matters absolutely (11+ = sterile).
2. This is the project's first MEASURED fitness landscape for an
   inherited trait: a single-gene stabilizing-selection curve with a
   hard sterility edge, produced by the world's resource economics
   (division cost 2.0 + child stake 0.45×parent) plus the energy cap —
   no explicit fitness function was programmed.
3. GOAL §8 status update: genetic transmission (PR#70/71, corrected
   PR#72) + now a measured selection gradient. The remaining gap to
   "evolution" claims is demonstrating that the distribution SHIFTS
   toward the peak over generations (this run: founder 8.0, peak 8.0 —
   designed to coincide; a shifted-founder run would test tracking).

## CAVEATS

- Founder 8.0 was CHOSEN to sit near the cap — the landscape's peak
  location may be an artifact of that choice rather than an
  independently discovered optimum. The clean test is a
  different-founder run (e.g. 5.0) asking whether the distribution
  MIGRATES toward 8-9.
- Bins 3-4 are small (n=10, n=2).
- Deterministic backends; 5 seeds.
- The sterile-edge mechanism depends on probe 1.0 (proposal at energy
  cap); other probes redraw the landscape.

## DISTRIBUTION MIGRATION TEST: run, NEGATIVE at this timescale

The clean test — founder 5.0 (well below the peak), asking whether the
distribution climbs toward 7-9 — was run on all 5 seeds. Per-generation
threshold means wander between 3.6 and 5.9 for 10-12 generations; **the
distribution does NOT migrate** to the fitness peak in 100k ticks.

Reading: the selection gradient is real (0.175 vs 0.388 reproduction
rates) but the migration speed is bounded by the mutation variance —
per-birth jitter sigma ≈ 1.0 is the same order as the distance to the
peak (3-4 units), so even a 2× reproduction-rate advantage moves the
mean only fractionally per generation. Detecting directional selection
needs either ~10× more generations (500k-1M ticks), a narrower
mutation band, or a steeper landscape.

## STATUS OF THE GOAL §8 CHAIN

- Variation (mutation): OBSERVED, now symmetric (PR#72 fix).
- Transmission: OBSERVED (BirthRecorded; genomes track across 10-15
  generations).
- Selection: OBSERVED as a reproduction-rate gradient (this document).
- Distribution change: NOT DETECTED at 12 generations (negative, with a
  quantified speed argument).

All four ingredients are now measured separately; what remains is a
timescale/mutation-band question, not an instrumentation gap.

## CAVEATS (migration test)

- 10-12 generations × 5 seeds; per-generation means have n≥2 only.
- The peak location (7-9) was measured under founder 8.0; a
  founder-dependent peak would make "migration toward 8" partly
  circular — the founder-5.0 run's flat trajectory is consistent both
  with slow migration and with a peak that sits near 5-6 too.
