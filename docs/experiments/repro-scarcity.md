# Reproduction Under Binding Scarcity

Status: OBSERVED (deterministic backends, threshold sweep; single seed per
cell)
Date: 2026-10-11
Harness: `run-primordial --backend reproducing-rule --divide-threshold <t>
--regen <r>` (`--divide-threshold` flag + founder-genome setter added
here; plus a mutation-floor fix the experiment forced)

## HYPOTHESIS

With a divide threshold reachable at the binding-scarcity equilibrium
energy (PR#64: equilibrium mean energy ~4 at regen 0.0005), lineages
expand toward the carrying-capacity limit and stabilize there.

## DISCOVERED BLOCKER (fixed in this PR)

The first run with threshold 3.0 produced ZERO sustaining births:
children inherited a division threshold of 20.0 despite the parent's 3.0.
Cause: `Genome::mutate` clamped `division_threshold` to a fixed
**20..100** band — a v0 design constant that assumes resource-rich
worlds. A low-threshold founder's lineage could never persist: every
child mutated back into a threshold it could not reach, then starved.

**Fix**: the mutation floor now follows the parent when the parent is
below 20 (`threshold_lo = min(20.0, parent.division_threshold)`).
Conventional genomes (>= 20) keep the original 20..100 band — existing
behavior is unchanged for all prior experiments.

## OBSERVED (reproducing-rule, regen 0.0005, 3000 ticks, seed 42)

| threshold | survivors | deaths | births | max gen | outcome |
|-----------|-----------|--------|--------|---------|---------|
| 3.0       | 0         | 32     | 24     | 2       | over-reproduction collapse |
| 4.0       | 0         | 32     | 24     | 2       | over-reproduction collapse |
| 5.0       | 4         | 24     | 20     | 2       | collapse |
| **6.0**   | **16**    | **0**  | **8**  | **1**   | **stable growth to capacity** |

Sustained run at threshold 6.0 (6000 ticks): **16 survivors, 0 deaths,
8 births** — the population doubles 8 → 16 and holds; 16 is exactly the
carrying capacity measured in PR#64.

## INTERPRETATION

- **Three population regimes selected by one gene**: threshold 3.0-5.0
  produces an r-strategy over-reproduction collapse (children born at
  0.9-1.5 energy starve before maturing; division debt kills the
  population), threshold 6.0 produces growth-to-capacity stability. The
  divide threshold is a life-history parameter whose value decides
  between collapse and persistence.
- **Lineages DO expand to the carrying-capacity limit and stabilize
  there** — the hypothesis holds at threshold 6.0: 8 founders → 16
  organisms = the field supply's limit, then zero further division (the
  field cannot support a third child's energy budget).
- **The mutation-floor fix was a precondition**: without it, no
  low-threshold lineage could exist for more than one generation. This is
  a concrete example of a genome-encoding constant silently gating an
  entire class of evolutionary experiments.

## CAVEATS

- Deterministic backends, one seed per cell; the regime boundaries
  (collapse below ~5, stability at ~6) are seed-conditional until
  replicated.
- The equilibrium population (16) was already known from PR#64; this
  experiment shows a threshold at which the population REACHES it via
  reproduction rather than being seeded there.
- mutation_rate remains 0.1 (sigma ±1.0 on the threshold); a fuller
  evolutionary treatment would vary it.
