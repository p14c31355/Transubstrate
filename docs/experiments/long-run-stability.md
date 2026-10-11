# Long-Run Stability — 100,000-Tick Primordial Populations

Status: OBSERVED (n=5 seeds, deterministic rule backend)
Date: 2026-10-11
Harness: `run-primordial --ticks 100000 --regen 0.0005
--divide-threshold 6.0` (reproducing-rule)

## QUESTION

The reference experiment target (GOAL §29) calls for 100,000+ world
ticks with reproduction, a complete event log, and no extinction.
Every prior run was ≤6000 ticks (PR#65). Does the population that
reaches the carrying capacity (PR#64/#65) stay there? Does it drift,
crash, or explode?

## CONFIGURATION

8 reproducing-rule founders, regen 0.0005/cell/tick (binding scarcity),
division threshold 6.0 (the only stable regime from PR#65), seeds
42 / 7 / 99 / 123 / 2024, 100,000 ticks each, full event log +
manifest + population samples every 10 ticks.

## OBSERVED (all 5 seeds, identical outcomes)

| metric | result |
|--------|--------|
| survivors | 16 in every seed (8 founders + 8 births) |
| deaths | 0 in every seed — extinction probability 0/5 |
| births | 8, all early; then birth freeze |
| max generation | 1 — evolutionary stagnation |
| field energy | 1281 → 28,948-29,679 (linear accumulation) |
| mean organism energy | 3.95 → 4.33-4.49 (equilibrium) |
| rejected ABSORBs | ~110k ≈ 69% of all cognition events |

The population is PERFECTLY stationary at the carrying capacity for
~95,000 ticks after the initial doubling.

## INTERPRETATION

1. **Long-run stability is real, not a 6k-tick artifact**: the 16-organism
   equilibrium survives 100k ticks in 5/5 seeds with zero deaths and zero
   runaway growth. The two-level carrying-capacity observations
   (node-level PR#55, field-level PR#64) now extend to the 100k-tick
   scale of the reference experiment target.
2. **Surplus and stagnation COEXIST**: the field accumulates ~29k units
   of unclaimed energy (supply 0.128/tick exceeds what 16 stationary
   organisms can extract), yet births stop. The mechanism is a SECOND,
   behavioral carrying capacity: at equilibrium each organism holds ~4.3
   energy, below the 6.0 division threshold. A stationary (never-MOVE)
   population cannot convert field surplus into population growth —
   intake rate per organism, not total energy, is the binding constraint.
3. **Evolutionary stagnation under stability**: with births frozen at
   generation 1, mutation has no substrate. A perfectly stable
   ecology is also a perfectly non-evolving one. In this world,
   variation requires either population turnover (deaths) or movement
   into fresh cells — the reproducing-rule has neither.
4. INFERENCES (not claims): the "life-history gene" regime map from
   PR#65 (threshold < 6 = collapse, 6 = stability) is stable at the
   100k-tick scale; the birth freeze is an intake-rate ceiling
   phenomenon, distinct from the population-cap ceiling.

## CAVEATS

- Deterministic rule backend: 5/5 identical outcomes reflect
  determinism, not statistical confidence about stochastic systems.
- No movement: reproducing-rule never selects MOVE. A mobile
  population might exploit the accumulated surplus (untested here).
- Single parameter point (regen 0.0005, threshold 6.0).

## NEXT (falsifiable hypotheses — first one TESTED and REJECTED)

- **TESTED, REJECTED**: a mobile variant (`mobile-reproducing` — steps
  to an adjacent cell when the SUM of the last-3 decision rewards goes
  non-positive, i.e. outcome-based starvation) DOES move (104 moves in
  100k ticks) but the population CRASHES: 334 total cognition events
  (everyone dead by ~t=35k at the latest), mean organism energy 0.10,
  mean lifetime of the dead 207 ticks. Mechanism: at this supply density
  (regen 0.0005/cell/tick over 256 cells) ALL cells are drained —
  movement costs 0.1/step and lands on equally-empty cells. The
  stationary strategy (sit and wait for regen) is strictly adaptive
  here; movement is a pure loss. The "surplus" is not spatially
  stranded — it is a RATE: supply accrues faster than 16 bodies can
  metabolize it away, but no single cell ever holds enough to pay for
  walking there.
- Still open: raising regen past the intake ceiling converts surplus
  into population growth without movement.
- Still open: movement may pay off at higher supply densities (the
  trade-off is movement cost vs supply density, untested above the
  starvation regime).
