# Outcome-Driven Exploration — Signal Emergence Precursor

Status: OBSERVED (n=5 seeds, temperature 0.7, single model)
Date: 2026-10-11
Depends on: signal pull (#30–#33), outcome-visible memory (#36),
`--llm-temperature` CLI option (this PR)

## HYPOTHESIS

If the LLM can see each past decision's observed outcome (reward) in its
memory context, and the world gives SIGNAL a real resource payoff
(signal-driven energy pull), then at temperature > 0 the model will
discover and repeat the SIGNAL strategy — an experience-driven precursor
to signaling behavior, with no human-assigned channel meaning.

## CONFIGURATION

- Backend: `llm:granite4.1:3b-q4_K_M`, `--llm-temperature 0.7` (new CLI
  option; temperature 0 default unchanged)
- Seeds: 42, 7, 99, 123, 2026; ticks 400, world swap at tick 200
- Memory: last 5 decisions rendered WITH outcomes (`outcome +X.XXX`)
- World: network-v0 with `signal_pull_rate` 0.5 (SIGNAL pulls energy from
  ring neighbors)

## OBSERVED RESULT

| seed | pre actions | post actions | deaths | effect (beh/mem) |
|------|------------|--------------|--------|------------------|
| 42   | {SIGNAL 38, WAIT 1, ABSORB 1} | {SIGNAL 5} | 1 | −0.05 / 0.0 |
| 7    | {WAIT 38, ABSORB 2} | {WAIT 40} | 0 | −0.025 / 0.0 |
| 99   | {WAIT 39, ABSORB 1} | {WAIT 40} | 0 | −0.025 / 0.0 |
| 123  | {WAIT 39, ABSORB 1} | {WAIT 33, SIGNAL 4} | 1 | −0.083 / −0.5 |
| 2026 | {ABSORB 38, WAIT 2} | {SIGNAL 23, ABSORB 17} | 0 | −0.55 / −1.0 |

Key rationales (verbatim, temperature 0.7):
- "Increase signal strength based past success."
- "Repeat successful signal behavior."
- "Repeat successful SIGNAL from past ticks." (seed 2026, post-swap)

## INTERPRETATION

- **First observed experience-driven action learning**: the rationales
  explicitly reference past success, and the behavior follows the outcome
  visible in memory. SIGNAL → pull → positive reward → repeat is exactly
  the loop the mechanism chain was built to close.
- **Seed 2026 is the strongest case**: pre-world ABSORB-dominant policy
  converts to 23/40 SIGNAL post-world — behavior change following the new
  world's payoff structure, i.e. adaptation.
- **Fragility**: seed 42's subject learned SIGNAL, then died (SIGNAL cost
  outpaced pull + metabolism); seed 123 died mid-transition. Exploration at
  temperature 0.7 is high-variance: 2/5 seeds show the learning loop, 3/5
  stay conservative or die.
- effect < 0 for adopting seeds reflects the fingerprint metric, not failure:
  changing strategy necessarily lowers behavioral continuity while adaptation
  happens.

## CAVEATS

- Single model, temperature 0.7, one run per seed. The 2/5 learning
  incidence is an observation, not a rate estimate.
- The outcome display shows the raw reward delta; the model sees
  "SIGNAL (outcome +1.6)" — this is numeric, leakage-free, but the
  interpretation "signal attracts resources" is the model's own inference
  from numbers, which is precisely the kind of emergent meaning-formation
  the Goal asks to detect (here on a single subject, not a convention).
- SIGNAL cost (0.5/strength) vs pull (0.5×strength per neighbor) is
  break-even at a single fresh neighbor pull plus decay; sustained SIGNAL
  stints are energy-negative unless harvest follows — consistent with the
  observed deaths.
