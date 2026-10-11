# LLM Model Migration — Multi-Seed Statistics

Status: OBSERVED (n=5 seeds, single model, single run per seed)
Date: 2026-10-11
Supersedes: the 3-seed numbers in PR#9/#10 (behavioral 0.10 / effect −0.90)
— the prompt changed since M8 (memory context added in PR#17), so the
behavioral value legitimately moved.

## HYPOTHESIS

With per-subject bounded memory now in the decision context, the rule → LLM
migration's behavioral continuity differs from the M8 baseline (0.10) and
the memory_retention dimension quantifies how much remembered pre-swap
content survives into the post-swap buffer.

## CONFIGURATION

- Backend A: `energy-rule` (rule-energy-v0), Backend B:
  `llm:granite4.1:3b-q4_K_M` (local Ollama 0.23.2, RTX 2070 SUPER 8GB)
- Seeds: 42, 7, 99, 123, 2026 (one run each)
- Ticks: 400 (half=200 per phase), cognition interval 5 → 40 decisions/phase
- Same world/seed for migration run and control A (backend A throughout)

## METRICS

- behavioral_action_distribution (1 − L1/2)
- memory_retention (multiset match of action:reward-bin, PR#24)
- control continuity (backend A throughout, same seed)
- migration effect (behavioral − control behavioral)

## OBSERVED RESULT

| seed | behavioral | memory_retention | control | effect |
|------|-----------|------------------|---------|--------|
| 42   | 0.750     | 0.000            | 1.000   | −0.250 |
| 7    | 0.750     | 0.000            | 1.000   | −0.250 |
| 99   | 0.750     | 0.000            | 1.000   | −0.250 |
| 123  | 0.750     | 0.000            | 1.000   | −0.250 |
| 2026 | 0.750     | 0.000            | 1.000   | −0.250 |

Mean ± sd (n=5): behavioral 0.750 ± 0.000, memory_retention 0.000 ± 0.000,
control 1.000 ± 0.000, effect −0.250 ± 0.000.

## INTERPRETATION

- Behavioral continuity moved from 0.10 (M8 baseline) to 0.75 since memory
  context was added to the LLM prompt (PR#17): with recent decisions
  visible, the LLM's post-swap actions converge toward the pre-swap rule
  pattern more often. This is a prompt-sensitivity result, not a
  robustness claim.
- memory_retention 0.0: the pre-swap buffer (rule actions, negative or
  zero rewards) is fully replaced by post-swap LLM entries (different
  actions and/or reward bins) within the 8-entry FIFO window. With 40
  decisions per phase and a limit of 8, the pre-swap content is gone from
  the buffer long before the first post-swap measurement — the dimension
  is measuring buffer dynamics, not backend continuity. A longer limit or
  boundary-anchored sampling would be needed to make this discriminating;
  noted as a design follow-up.
- The 5/5 seed agreement again reflects temperature-0 prompting structure,
  not proven robustness across models.

## CAVEATS

- Single model, single run per seed, temperature 0.
- memory_retention semantics under FIFO roll-off: the dimension currently
  measures content survival in a rolling buffer; with decisions/phase ≫
  limit, pre-swap content always ages out. Treat 0.0 here as "buffer fully
  rolled over", not "memory destroyed".
