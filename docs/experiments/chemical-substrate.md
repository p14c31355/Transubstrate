# Chemical Substrate — RELEASE Channel 1 Activated

Status: ENGINEERING CLAIMS + OBSERVED (unit tests; substrate live, no
meaning-formation claim)
Date: 2026-10-11
Harness: `PrimordialWorld` (substrate) + unit tests

## WHAT WAS SEALED BEFORE

The primordial world has carried a `chemical_field` since M3 (observable
via the `chemical_0` channel), but NOTHING ever wrote to it: `RELEASE`
only returned energy to the cell's energy field. The semantics-free
substrate existed as plumbing only — GOAL §6 lists
`chemical_0 / chemical_1` as observation channels, and every run had them
pinned at 0.

## WHAT CHANGED

1. **`RELEASE` gains a channel parameter** (arity 1 → 2, matching the
   GOAL §6 vocabulary `RELEASE(channel, amount)`):
   - `params[1] = 0` (default): return energy to the cell's energy field
     (old behavior).
   - `params[1] = 1`: convert organism energy into the cell's chemical
     field. The chemical is NOT energy — nothing absorbs it, it has no
     payoff; it can only be sensed.
2. **Chemical dynamics**: per-tick decay (`chemical_decay`, default 0.1)
   and 4-neighbor diffusion (`chemical_diffusion`, default 0.05) —
   gradient-forming, like the energy field.
3. Manifest updated (`RELEASE` arity 2); the old one-param form is now a
   schema violation (fail-safe rejects it).

## OBSERVED (unit tests, 3 new; 107 total)

- `release_channel_one_writes_the_chemical_field`: channel-1 release
  writes chemical at the organism's cell and costs organism energy.
- `chemical_diffuses_and_decays`: a blob shrinks at the source cell and
  spreads to neighbors over 20 ticks.
- `chemical_has_no_absorb_payoff`: a cell loaded with chemical but empty
  of energy yields ZERO gain from ABSORB — the substrate stays
  meaning-free (nothing can eat it).

## INTERPRETATION

The substrate is now live end-to-end: a subject can mark a cell
(pay energy), the mark spreads and fades, and any subject can observe the
gradient — but no action converts a chemical into survival. Any meaning a
population coordinates onto chemical gradients would be BY CONVENTION
ONLY, which is exactly what the chemical-field meaning-formation question
asks. The substrate now permits that experiment; the experiment itself is
the next queued item.

## CAVEATS

- Schema change: one-param `RELEASE` is rejected from here on (visible in
  the manifest; fail-safe path).
- Single-channel (`chemical_0`); a second channel (`chemical_1`) would
  let two marks coexist — deferred until a meaning-formation design needs
  it.
