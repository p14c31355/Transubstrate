# LLM World Migration — Experiment Record

Status: OBSERVED (n=5 seeds, single model, single run per seed)
Date: 2026-10-11

## HYPOTHESIS

An LLM subject migrating primordial → network mid-run will show behavioral
shift (adaptation to the new world's channels) but retain its memory buffer
(memory_retention near 1.0), unlike rule controllers whose behavioral
fingerprint flips completely (0.0) because the new world's manifest routes
their fixed rule to a different action.

## CONFIGURATION

- Backend: `llm:granite4.1:3b-q4_K_M` (local Ollama 0.23.2, RTX 2070 SUPER 8GB)
- Seeds: 42, 7, 99, 123, 2026 (one run each)
- Ticks: 400 (half=200 per world), cognition interval 5 → 80 decisions
- Migration: primordial-v0 → network-v0 at tick 200, carried energy
- Comparison baseline: same experiment with `energy-rule` (seed 42):
  behavioral 0.0 (ABSORB pre → WAIT post), memory_retention 0.0

## METRICS

- behavioral_action_distribution (1 − L1/2)
- memory_retention (multiset match of action:reward-bin, PR#24)

## OBSERVED RESULT

| seed | behavioral | memory_retention |
|------|-----------|------------------|
| 42   | 0.975     | 1.000            |
| 7    | 0.975     | 1.000            |
| 99   | 0.975     | 1.000            |
| 123  | 0.975     | 1.000            |
| 2026 | 0.975     | 1.000            |

Mean ± sd: behavioral 0.975 ± 0.000, memory_retention 1.000 ± 0.000.
80/80 decisions valid, zero deaths in every run.

Action split: pre-world {WAIT 39, ABSORB 1} → post-world {WAIT 40}.
LLM rationales are observation-dependent in both worlds (e.g. "No energy
signal detected." → "Energy high, no signals to absorb.").

## INTERPRETATION

- The LLM subject's behavior is nearly identical across the two worlds
  (L1=0.05): it answers with WAIT under both manifests. This is NOT evidence
  of cross-world skill transfer — the honest reading is that its policy is
  so conservative that the world change barely perturbs it. The rule
  controller, by contrast, flips completely (0.0) because its hard-coded
  channel lookup breaks in the new world.
- Memory retention 1.0 confirms the mechanism (the buffer crosses the
  boundary untouched); it says nothing yet about the memory being USED.
- The 5/5 seed agreement again reflects temperature-0 prompting structure,
  not proven robustness across models.

## CAVEATS

- Single model, single run per seed, temperature 0.
- The network world's decision surface may be too passive (only WAIT/ABSORB
  were ever attractive); a world that rewards SIGNAL/MOVE would discriminate
  adaptation better.
- No same-world control for the LLM was run for this record (the rule
  baseline plays the contrast role); queued in docs/next.md.
