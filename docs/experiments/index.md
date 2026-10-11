# Experiment Index

Every experiment the repository has recorded, with its status and where the
record lives. Kept in the Goal's scientific-integrity style: results are
labeled OBSERVED / COMPUTED / INFERRED / HYPOTHESIZED, negative results are
kept, and nothing here claims emergence.

| # | Experiment | Record | Status |
|---|-----------|--------|--------|
| 1 | Primordial survival (energy-rule / echo / random) | PR#3 descriptions | OBSERVED (single runs) |
| 2 | Reproduction / lineage growth | PR#4 descriptions | OBSERVED (8 founders → 32 alive, cap enforced) |
| 3 | Controller migration + controls (M5) | PR#5 | OBSERVED (calibration) |
| 4 | Replay determinism (M6) | PR#6 | OBSERVED (regeneration matches summaries) |
| 5 | Real LLM cognition (M7, 80/80 valid) | PR#8 | OBSERVED (single model) |
| 6 | Rule → LLM migration (M8) | PR#9, PR#10 | OBSERVED (3 seeds, continuity 0.10 / effect −0.90) |
| 7 | Scarcity drift under mutation | PR#11 | OBSERVED NEGATIVE (drift +0.0006, below band) |
| 8 | Two-phenotype competition | PR#12 | OBSERVED (shareA 0.759, 5/5 seeds) |
| 9 | World migration, rule controller | PR#22 | OBSERVED (behavioral 0.0 — rule breaks across worlds) |
| 10 | World migration, LLM cognition | docs/experiments/llm-world-migration.md | OBSERVED (5 seeds: behavioral 0.975±0, memory_retention 1.0±0; INTERPRETATION: conservative policy, not skill transfer) |
| 11 | World migration A/B with no-swap control | PR#28 + record update | OBSERVED (effect: rule −1.0/−1.0 = swap-fragile; LLM ±0.0 = swap-robust) |
| 12 | Rule → LLM migration, multi-seed statistics | docs/experiments/llm-model-migration-stats.md | OBSERVED (n=5: behavioral 0.75±0, memory_retention 0.0±0 — buffer fully rolls over; effect −0.25±0) |
| 13 | Cross-model migration (granite vs qwen) | docs/experiments/cross-model-migration.md | OBSERVED (n=5 each: behavioral 0.75 vs 0.00 — temperature-0 tie broken; post-migration behavior is model-dependent) |
| 14 | Signal-driven energy pull (network world) | docs/experiments/signal-pull.md | OBSERVED (signal-seeker flips SIGNAL→ABSORB across the boundary; engineered payoff, not emergence) |
| 15 | Outcome-driven exploration (temp 0.7, signal pull) | docs/experiments/outcome-exploration.md | OBSERVED (2/5 seeds learned SIGNAL from outcome-visible memory; 1 seed adapted post-swap; fragility documented) |
| 16 | Temperature sweep (outcome learning vs temp) | docs/experiments/temperature-sweep.md | OBSERVED (0.3: 3/5 learning, 0 deaths; 0.7: 2/5, 2 deaths; 1.0: 2/4 valid, 1 schema-rejection — fail-safe OBSERVED) |
| 17 | LLM 5-dimension sweep (memory_usage numbers) | docs/experiments/llm-usage-sweep.md | OBSERVED (n=5 temp 0.3: 4/5 usage 1.0 (swap-robust); seed 123 full strategy replacement 0/0/0 — learning loop visible in vector) |
| 18 | Cross-model outcome learning (qwen, signal pull) | docs/experiments/cross-model-outcome-learning.md | OBSERVED (4/4 complete runs mix SIGNAL+HOP post-swap with no channel meaning; 1 death/run — aggressive exploration measured) |
| 18 | Signaling-follower detection (multi-subject) | docs/experiments/signaling-followers.md | OBSERVED mixed (attraction 5/5 and intake ~2.1x vs null; survival economics fail under current params — negative result disclosed) |
| 19 | Memory ablation (causal carry-vs-wipe) | docs/experiments/memory-ablation.md | OBSERVED: rule calibration 0.0 (true negative); LLM n=5 behavioral +0.50 / retention +0.55 / usage +0.80 — with disclosed pre-phase confound |
| 20 | LLM follower in the signaling world | docs/experiments/llm-signaling.md | OBSERVED n=3: null-signal (strength 0) is the LLM's cheap safe action; seed 99 sent real signals AND converged on the emitter (proximity 1.0); 3/3 survive |
| 21 | Convention formation (2 LLM followers) | docs/experiments/convention-formation.md | OBSERVED NEGATIVE (n=1): both converge on null-signal, no interaction; rule emitter's signal also ignored at temp 0.3 |
| 22 | Population world migration | docs/experiments/population-migration.md | OBSERVED: per-subject continuity spread 0.281 ± 0.418 across one boundary (energy-rule 0.0, echo 1.0, signal-seeker 0.125); determinism check passes |
| 23 | Field-level carrying capacity | docs/experiments/carrying-capacity.md | OBSERVED: carrying capacity ≈ 13-16 organisms at regen 0.0005 (survivor count converges as initial population grows 8→48); field reaches supply=consumption steady state; births still zero under scarcity |
| 24 | Reproduction under binding scarcity | docs/experiments/repro-scarcity.md | OBSERVED: threshold 6.0 → population doubles 8→16 and stabilizes AT the carrying capacity; 3.0-5.0 = over-reproduction collapse; mutation-floor fix was the precondition (fixed 20..100 clamp silently gated low-threshold lineages) |

## Running an experiment

```sh
cargo build --release
./target/release/transubstrate run-primordial --help      # population runs
./target/release/transubstrate run-migration --help       # controller migration + controls
./target/release/transubstrate run-world-migration --help # primordial -> network
./target/release/transubstrate replay <events.jsonl>      # metrics without re-running cognition
./target/release/transubstrate verify <events.jsonl>      # log integrity
```

Each run writes `events.jsonl` plus a `manifest.json` (experiment kind,
seed, config, world implementation) into its output directory. Replays and
verification work from those two files alone.
