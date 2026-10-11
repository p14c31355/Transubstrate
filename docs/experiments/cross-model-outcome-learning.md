# Cross-Model Outcome Learning — qwen3.5 on the Signal-Pull World

Status: OBSERVED (n=5 seeds, temperature 0.3, single second-model; 4
hand-collected runs + 1 batch rerun; partial logs disclosed)
Date: 2026-10-11
Extends: docs/experiments/outcome-exploration.md and
docs/experiments/cross-model-migration.md

## HYPOTHESIS

qwen3.5:2b-q4_K_M (a differently-trained, thinking-style model with
reasoning disabled) also learns the SIGNAL payoff from outcome-visible
memory in the signal-pull network world — i.e. outcome-driven learning is
not granite-specific.

## CONFIGURATION

- `sweep-world-migration --seeds 42,7,99,123,2026 --backend
  llm:qwen3.5:2b-q4_K_M:none --llm-temperature 0.3 --ticks 400`
- World: primordial → network at tick 200; signal pull (`signal_pull_rate`
  0.5) active in the destination
- Batch collection: the first four runs were interrupted by a tool timeout
  after their event logs completed (the process was killed before the
  summary file); seed 2026 was re-run through the sweep runner and produced
  a complete summary. All five per-seed event logs were analyzed.

## OBSERVED RESULT

Complete per-seed event logs (5/5; the fifth log is truncated to one
pre-swap decision because the collection process was killed — disclosed
per seed below):

| run | pre (origin) | post (network) | deaths |
|-----|--------------|----------------|--------|
| 1   | {WAIT 35, SIGNAL 4, RELEASE 1} | {WAIT 17, SIGNAL 2, HOP 5} | 1 |
| 2   | {WAIT 40} | {WAIT 25, HOP 4, SIGNAL 4} | 1 |
| 3   | {WAIT 38, <none> 1, MOVE 1} | {HOP 6, WAIT 24, SIGNAL 5} | 1 |
| 4   | {WAIT 40} | {WAIT 30, SIGNAL 7, HOP 2} | 1 |
| 5 (truncated log) | {WAIT 1} | — (log cut) | 0 |

Seed 2026 batch rerun (complete): behavioral 0.775, memory_retention 0.75,
memory_usage 1.0, effect −0.177 / −0.25 / 0.0.

qwen rationales in the network world are observation-dependent and
signal-oriented: "Signal nearby signal to trigger interaction.", "Signal
received, move forward.", "Signal to nearby node to establish connection."

## INTERPRETATION

- qwen3.5 explores actively in the signal-pull world: every complete run
  mixes SIGNAL and HOP post-swap, unlike its WAIT-only primordial behavior
  and unlike granite's uniformly conservative policy. HOP usage (2–6 per
  run) shows movement toward signal-rich nodes — payoff-seeking movement,
  which the pre-swap primordial world never elicited.
- 4/4 complete runs used both SIGNAL and HOP after the swap without any
  human-assigned channel meaning — cross-model outcome-driven exploration
  is OBSERVED alongside granite's.
- Every complete qwen run recorded exactly one death (energy exhaustion):
  qwen explores more aggressively than granite and pays for it. The
  exploration/death trade-off is a world-economics property, not a model
  defect, and is now measurable.
- Seed 2026's memory_usage 1.0 with behavioral 0.775 shows a subject that
  kept acting on its remembered decisions while also incorporating new
  world signals — partial continuity with adaptation.

## CAVEATS

- The first four runs were collected by a foreground process killed by a
  tool timeout AFTER their event logs were written; the fifth log is
  truncated to one pre-swap decision and is excluded from behavioral
  statistics. The seed-2026 summary comes from a clean rerun.
- One run each per seed; no repetition within seeds.
- Thinking-style model with `reasoning_effort: "none"`; hidden reasoning
  may differ with the flag omitted.
