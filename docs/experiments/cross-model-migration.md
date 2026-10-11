# Cross-Model Migration Comparison — granite4.1 vs qwen3.5

Status: OBSERVED (n=5 seeds each, two models, temperature 0)
Date: 2026-10-11
Extends: docs/experiments/llm-model-migration-stats.md

## HYPOTHESIS

The temperature-0 seed agreement observed with granite4.1 reflects prompting
structure, not model-invariant behavior. A second, differently-trained model
(qwen3.5, a thinking-style model) should produce a measurably different
post-migration behavioral fingerprint under the same rule→LLM migration.

## CONFIGURATION

- Backend A: `energy-rule` (rule-energy-v0) — identical to prior records
- Backend B: `llm:qwen3.5:2b-q4_K_M:none` — thinking-style model, reasoning
  disabled via `reasoning_effort: "none"` (new CLI syntax `llm:<model>:none`;
  without it the model burns the completion budget on hidden reasoning and
  returns empty JSON)
- Seeds: 42, 7, 99, 123, 2026; ticks 400; interval 5
- Baseline comparison: granite4.1:3b-q4_K_M with the identical harness
  (llm-model-migration-stats.md: behavioral 0.75, memory 0.0, effect −0.25)

## OBSERVED RESULT

qwen3.5:2b-q4_K_M (n=5):

| seed | behavioral | memory_retention | control | effect |
|------|-----------|------------------|---------|--------|
| 42   | 0.000     | 0.000            | 1.000   | −1.000 |
| 7    | 0.000     | 0.000            | 1.000   | −1.000 |
| 99   | 0.000     | 0.000            | 1.000   | −1.000 |
| 123  | 0.000     | 0.000            | 1.000   | −1.000 |
| 2026 | 0.000     | 0.000            | 1.000   | −1.000 |

granite4.1:3b-q4_K_M (same harness): behavioral 0.75 ± 0.0,
memory_retention 0.0 ± 0.0, effect −0.25 ± 0.0.

Rationales (qwen, post-swap): "No signals detected, waiting for input.",
"Energy sufficient, no signals nearby." — observation-dependent, JSON-valid,
zero schema rejections after the reasoning-effort fix.

## INTERPRETATION

- The temperature-0 tie is BROKEN: granite4.1 and qwen3.5 produce
  significantly different post-migration fingerprints (behavioral 0.75 vs
  0.00) under the identical harness. Post-migration behavior is
  model-dependent, which is exactly what SUBJECT != MODEL predicts and what
  the continuity framework is built to measure.
- qwen3.5's behavioral 0.0 mirrors the rule→echo pattern (full action
  distribution flip), not the granite partial overlap (0.75).
- memory_retention 0.0 for both models remains a FIFO roll-off artifact
  (40 decisions/phase vs buffer limit 8) — model-independent by mechanism,
  consistent with the design follow-up (boundary-anchored sampling).

## CAVEATS

- Two models, one run each per seed, temperature 0. The cross-model
  difference is OBSERVED; generalization to other models is HYPOTHESIZED.
- qwen3.5 required `reasoning_effort: "none"` via the new
  `llm:<model>[:<effort>]` CLI syntax; thinking-style models without it
  return empty completions (finish_reason=length) and are rejected as
  malformed decisions. Endpoints that ignore the field are unaffected.
