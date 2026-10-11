# Genome Drift Across Generations — First Genetic-Transmission Data

Status: OBSERVED (n=5 seeds) + INFERENCES (mechanism, not proven)
Date: 2026-10-11
Harness: `EventPayload::BirthRecorded` (new) + post-hoc analysis of the
100k-tick turnover runs (PR#70 harness, probe 1.0, regen 0.0005)

## QUESTION (GOAL §8)

"Can behavior spread genetically/configurationally?" Until now this
was untestable: every experiment froze at generation 1, so mutation had
no substrate. The turnover equilibrium (PR#70) reaches generation 10-12
— the first runs where inherited configurations actually accumulate.

## INSTRUMENTATION

`EventPayload::BirthRecorded { subject_id, parent, tick, genome }` is
now logged at every division — the birth genome (genetic-transmission
record) lives in the event log, so drift analysis runs post-hoc from
logs without re-running anything. Before this PR the lineage tree
(genome_at_birth) existed in memory only and was lost with the process.

## OBSERVED (n=5 seeds, 248 births each, 100k ticks)

Division threshold by generation (founder value 6.0):

| seed | gen-1 mean | late-generation mean | max gen |
|------|-----------|----------------------|---------|
| 42   | 6.215     | 9.315 (gens 11-12)   | 12      |
| 7    | 6.267     | 8.155 (gens 9-10)    | 10      |
| 99   | 6.273     | 7.808 (gens 11-12)   | 12      |
| 123  | 6.124     | 7.432 (gens 10-11)   | 11      |
| 2024 | 6.168     | 8.128 (gens 10-11)   | 11     |

Seed 42 in detail (per-generation means): 6.22 → 6.63 → 6.54 → 6.84 →
7.15 → 7.15 → 7.75 → 7.87 → 8.35 → 8.65 → 9.16 → 9.95.

**The division threshold drifts UPWARD in every seed**: from the
founder 6.0 to 7.4-9.9 by the late generations — a 25-66% increase in
the life-history gene over ~10 generations, with consistent direction
across all 5 seeds.

## INTERPRETATION (INFERENCES, mechanism not proven)

- Consistent direction across 5 independent seeds is hard to explain
  by neutral drift alone — a directional force is present.
- Plausible mechanism (untested): a higher threshold means the parent
  forages longer before dividing, so each child is born with more
  energy (a bigger initial stake) — an investment-per-offspring
  advantage. At the turnover equilibrium the cost of reproducing
  SLOWER is near zero while the cap is full: somebody reproducing is
  enough to hold a slot, so offspring QUALITY can outcompete offspring
  QUANTITY. The drift looks like a spontaneous r→K shift (many-cheap →
  few-expensive offspring) driven by the cap-full regime.
- Consistency check with PR#65: thresholds BELOW 6.0 collapse
  (over-reproduction starvation); the drift never goes down. The
  collapse regime prunes the low side, the cap-full regime rewards the
  high side — asymmetric selection pressure with a ratchet-like
  appearance.
- This is the first OBSERVED instance of inherited configuration
  spreading across generations in Transubstrate. GOAL §8's question
  now has a positive, measured instance — at the level of a
  life-history parameter, under a full-cap regime, in a deterministic
  rule population.

## CAVEATS

- Selection vs drift is NOT separated: only survivors' descendants are
  recorded, and no per-genotype fitness measurement was made. The
  directional consistency across seeds is suggestive, not proof.
- One gene (division_threshold) tracked in depth; the other four
  (absorb_affinity, signal_bias, move_bias, mutation_rate) are logged
  and unanalyzed.
- The drift is observed in the turnover regime only (probe 1.0). Under
  the frozen regime (PR#69) no generations occur, so no drift is
  possible — genetic transmission REQUIRES population turnover.
- Single parameter point (regen 0.0005, threshold 6.0, cap 256).

## NEXT

- Selection-vs-drift test: compare per-genotype reproductive success
  (children per parent) against threshold values — if high-threshold
  parents produce more surviving lineages, selection is supported.
- Track the other four genes (data already logged).
- Test the r→K mechanism: does the drift reverse when the cap is
  raised (making reproduction rate matter again)?
