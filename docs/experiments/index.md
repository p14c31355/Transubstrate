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
| 16 | Temperature sweep (outcome learning vs temp) | docs/experiments/temperature-sweep.md | OBSERVED (n=5/temp: temp 0.3 dominates 0.7 — 3/5 learning, 0 deaths; bimodal behavioral 0.65±0.314) |

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
