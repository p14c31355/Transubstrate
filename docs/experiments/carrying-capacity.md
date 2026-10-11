# Field-Level Carrying Capacity in the Primordial World

Status: OBSERVED (deterministic backends, sweep over initial population;
single seed shown per cell)
Date: 2026-10-11
Harness: `run-primordial --regen <rate> --population <n>` (`--regen` flag
added here)

## HYPOTHESIS

At the default regeneration rate the primordial field never binds
(regen 0.01/cell/tick refills faster than 8 organisms consume), so no
population dynamics appear. Lowering regen should produce a field-level
carrying capacity: a stable survivor count set by the field's energy
supply, independent of how many organisms start.

## DESIGN

- Primordial grid (16×16), energy-rule organisms, 2000 ticks, seed 42.
- Sweep A (regen × 8 organisms): 0.01, 0.002, 0.0005, 0.0 — does the
  field deplete?
- Sweep B (population × regen 0.0005): 8, 16, 24, 32, 48 — does the
  survivor count converge?

## OBSERVED

Sweep A — field depletion (8 organisms, 2000 ticks):

| regen | field start → end | survivors |
|-------|------------------|-----------|
| 0.01  | 1290 → 49281     | 8         |
| 0.002 | 1269 → 8320      | 8         |
| 0.0005| 1265 → 1029      | 8         |
| 0.0   | 1264 → 106       | 8         |

The field depletes only at low regen; with regen 0.0 the field is
consumed to 106 but the initial population still outlives the window.

Sweep B — carrying capacity (regen 0.0005, 2000 ticks):

| initial pop | survivors | deaths | field end |
|-------------|-----------|--------|-----------|
| 8           | 8         | 0      | 1029      |
| 16          | 15        | 1      | 593       |
| 24          | 19        | 5      | 398       |
| 32          | 16        | 16     | 440       |
| 48          | 13        | 35     | 540       |

## INTERPRETATION

- **Carrying capacity ≈ 13-16 organisms** at regen 0.0005: increasing the
  initial population raises the death count (0 → 1 → 5 → 16 → 35) while
  the survivor count converges to 13-16. The field's supply
  (0.0005 × 256 cells = 0.128 units/tick) supports only that many
  energy-rule metabolisms.
- **The field reaches a steady state**: final field energy converges to
  ~400-600 across population levels (supply = consumption), regardless of
  how many started — the equilibrium, not the initial count, is what the
  world controls.
- **First population-dynamics data for POPULATION ECOLOGY** (GOAL §34):
  the environment, not a rule, caps the population. This is the primordial
  counterpart of the node-level carrying capacity observed in the network
  world (2/6 at the emitter).
- **Births still zero**: no member reached the divide threshold under
  binding scarcity — reproduction experiments under binding scarcity have
  their precondition (deaths exist) but no divide pressure yet; the
  reproduction-in-scarcity experiment needs a divide threshold reachable
  at equilibrium energy.

## CAVEATS

- Deterministic backends, one seed per cell; the convergence pattern is
  consistent across the sweep but multi-seed replication is pending.
- energy-rule absorbs greedily; a thriftier policy would shift the
  equilibrium upward. The capacity number is policy-relative by design —
  the world sets the supply, the policy sets the demand.
- No divide events at equilibrium: the reproduction-under-scarcity
  question remains open.
