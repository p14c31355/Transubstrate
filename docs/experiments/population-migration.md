# Population World Migration — Per-Subject Continuity Spread

Status: OBSERVED (deterministic backends, n=4 members, single seed shown;
spread is the object of measurement)
Date: 2026-10-11
Harness: `run-population-migration` (`run_population_migration` in
world_migration.rs)

## HYPOTHESIS

When a POPULATION migrates across the same world boundary at the same
tick, per-subject continuity is NOT uniform: members with different
policies (and different in-world experiences) show different continuity
readings. The spread itself is the quantity of interest — an
individual-differences view of continuity, absent from the single-subject
harness.

## DESIGN

- 4 members: energy-rule ×2, echo, signal-seeker (deterministic backends;
  same primordial world, same network destination, same boundary tick).
- Each member keeps its own bounded memory (FIFO 8) and its own
  pre/post action collectors. Members that die pre-migration are recorded
  as not survived.
- Per-member behavioral continuity (1 − L1/2 across the boundary) and
  memory usage; population mean ± sd.

## OBSERVED (seed 42, 400 ticks, cognition every 5)

| member | backend | continuity | usage | survived | final energy |
|--------|---------|-----------|-------|----------|--------------|
| pop-00 | rule-energy | 0.0000 | 1.0000 | true | 64.00 |
| pop-01 | rule-energy | 0.0000 | 1.0000 | true | 64.00 |
| pop-02 | mock-echo   | 1.0000 | 1.0000 | true | 0.00 |
| pop-03 | rule-signal-seeker | 0.1250 | 1.0000 | true | 55.00 |

Population: behavioral continuity **0.281 ± 0.418**, memory usage
1.000 ± 0.000.

## INTERPRETATION

- **The spread is real and large**: 0.281 ± 0.418 across one boundary.
  Same-world, same-tick migration produces per-subject continuities from
  0.0 (energy-rule: the primordial ABSORB strategy fully changes in the
  network world, consistent with the single-subject swap-fragile result)
  to 1.0 (echo: WAIT is WAIT in both worlds — continuity of the policy,
  not of the experience).
- **Determinism check**: the two energy-rule members read identically
  (0.0000 both) — same backend + deterministic world = same trajectory
  class; the spread comes from policy differences, not noise.
- **Echo's 1.0 is continuity of form, not of life**: pop-02 kept the same
  action distribution while ending at zero energy. Continuity dimensions
  are independent — this is exactly why the vector is not collapsed into
  a single score (GOAL §4).
- **Memory usage 1.0 for all members**: every member re-enacted all of
  its boundary-remembered actions post-migration — the memory dimension
  does not differentiate members here; the behavioral dimension does.
- All four members survived the boundary; the migration itself is not
  lethal at these parameters.

## CAVEATS

- Deterministic backends, single seed shown; LLM members would add
  sampling variance on top of policy variance (harness accepts them).
- Memory usage is saturating in this configuration (all 1.0) — it
  discriminates only when boundary strategies differ from post strategies
  (as in the single-subject ablation experiments).
