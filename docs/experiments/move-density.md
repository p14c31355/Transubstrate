# Movement vs Supply Density — and the Turnover Equilibrium Discovery

Status: OBSERVED (rule backends, n=5 seeds for the headline result) +
ENGINEERING CLAIMS (metric design)
Date: 2026-10-11
Harness: `run-primordial --backend mobile-reproducing --divide-probe P
--regen R` (new `--divide-probe` flag)

## QUESTION (from PR#69's open item)

PR#69 rejected movement at regen 0.0005 (movement cost > supply
density). Does movement pay at HIGHER supply densities? The follow-up
was flagged as a one-flag experiment.

## TWO RULE-DESIGN BUGS FOUND AND FIXED FIRST

The mobile rule's starvation detector needed two corrections, each a
real lesson about what the world's signals CAN mean:

1. **Energy-level tests never fire**: the rule absorbs only when the
   cell holds regen, so its absorb memories are successes — a subject
   that forages productively looks "never starving" no matter what.
2. **All-decision reward sums ALWAYS fire**: at equilibrium the rule
   mostly proposes DIVIDE (rejected — the GENOME threshold 6.0 is not
   met), and every rejection pays metabolism. The reward sum goes
   negative everywhere, so the subject wanders forever paying move
   costs — population crash in EVERY regen regime (0.005/0.02/0.05:
   16 deaths, 334 cognition events each). A rejected DIVIDE says
   nothing about cell quality; conflating them was a design bug.

Final design: the cell is "dead" when the last 2+ ABSORB attempts sum
to non-positive reward. Absorb attempts are the only decision type
whose outcome tracks the CELL.

## STRUCTURAL FINDING: cell quality is (almost) unobservable

With `--divide-probe 0.4` (default), the rule proposes DIVIDE at
energy 4.0 and never accumulates absorb-failure memories at all —
movement never fires in any regime. The information needed to decide
"my cell is bad, move" EXISTS in the world (absorb outcomes land in
memory with rewards) but the rule's foraging gate prevents the
information from ever being gathered. **A rule that stops foraging
cannot learn that it should forage elsewhere.**

## THE DISCOVERY: --divide-probe 1.0 creates a TURNOVER equilibrium

Raising the probe to 1.0 (= never gate foraging on energy, keep
absorbing and proposing DIVIDE) at 100k ticks, n=5 seeds:

| seed | pop trajectory | deaths | births | max gen | mean lifetime of dead |
|------|----------------|--------|--------|---------|----------------------|
| 42   | 8→256→17       | 239    | 248    | **12**  | 6591 ticks           |
| 7    | 8→256→17       | 239    | 248    | **10**  | 6162 ticks           |
| 99   | 8→256→16       | 240    | 248    | **12**  | 6803 ticks           |
| 123  | 8→256→16       | 240    | 248    | **11**  | 6885 ticks           |
| 2024 | 8→256→17       | 239    | 248    | **11**  | 5547 ticks           |

**OVERSHOOT-COLLAPSE-TURNOVER dynamics**: the population grows to the
hard cap (256), exhausts the field (starvation death spiral), and
settles at a NEW equilibrium (~16-17) where intake meets metabolism.
Meanwhile the lineage reaches generation 10-12 — the FIRST runs where
mutation accumulates across generations (every prior experiment
froze at generation 1).

## INTERPRETATION

1. The population dynamics REGIME is controlled by the rule's
   reproduction gate, not by the environment alone:
   - probe 0.4 → frozen stationary stability (PR#69, gen 1)
   - probe 1.0 → overshoot-collapse-turnover (gen 10-12)
   One rule parameter flips the ecology between "no turnover, no
   evolution" and "massive turnover, 12 generations of variation".
2. Movement per se NEVER pays in this world: not at starvation density
   (PR#69: cost > supply), not at high density (nothing to gain), and
   in between (0.005/0.02) the corrected rule simply never needs to
   move. The second open item from PR#69 is now resolved negatively:
   with a cap-256 world and regen bounded, movement is dominated by
   waiting.
3. ENGINEERING claim: absorb-outcome memory is the only usable
   cell-quality signal under the M3 "resources are not directly
   observable" design. That constraint is what makes the foraging-gate
   placement ecologically decisive.

## CAVEATS

- The turnover runs use `mobile-reproducing` as the carrier only —
  with probe 1.0 it moves ~0-14 times; the dynamics come from the
  foraging gate, not movement.
- Genome drift across the 10-12 generations is RECORDED
  (genome_at_birth in the lineage tree) but not yet analyzed — that is
  the natural next experiment (does the threshold-6.0 lineage drift?
  does mutation re-enter the collapse regime?).
- Deterministic backends; 5 seeds × identical cap arithmetic reflects
  determinism of the rule, though generation depth varies by seed
  (stochasticity enters through world-state, not the backend).

## NEXT

- Genome drift analysis over the 12-generation lineages (data already
  in the event logs).
- The GOAL §8 question "can heritable traits survive generations" is
  now testable with this harness.
