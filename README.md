# Transubstrate

Experimental substrate for **persistent artificial subjects** whose continuity is not tied
to any single language model, process, machine, body, or world.

## The research question

> Can an artificial subject preserve **measurable continuity** while its cognitive model,
> computational substrate, embodiment, and environment are independently replaced?

Central design principle: **SUBJECT ≠ MODEL**. A subject — identity, history, memory,
drives, relationships, lineage — is a persistent entity; cognition is a replaceable organ.
Continuity is not asserted by a UUID; it is *measured* across controlled migrations.

A secondary question: can the same subject cross between simulation and the physical world
through one stable world boundary — **WABIX**, a world ABI?

## What this is NOT

- Not a chatbot or agent framework
- Not a memory backup system
- Not "an LLM roleplaying life"
- Not an LLM wrapper — the system is fully functional with **no LLM installed**

## What is a persistent artificial subject?

A subject = identity + history + memory + drives + relationships + lineage + embodiment
state + resource state + cognition backend. The backend is a replaceable organ: subjects
survive model swaps, world swaps, and body swaps — and Transubstrate *measures* what
remains continuous across those swaps.

## What is WABIX?

The World ABI: the stable boundary between a subject and whatever world/body it inhabits.
Subjects discover capabilities explicitly (`observe` channels + `actions` with arity and
parameter ranges), act only through typed validated actions, and the world answers with
typed events. Two structurally different worlds (grid-based `primordial-v0`, ring-topology
`network-v0`) already implement it. See [docs/prior-art.md](docs/prior-art.md) for how
WABIX positions itself against W3C WoT, MCP, ROS2, and Viam.

## What is the primordial world?

The first-class ALife reference environment: a 2D grid with an energy field (diffusion +
regeneration), a semantics-free chemical field, a decaying signal field, per-organism
metabolism, energy costs per action, death by exhaustion, and DIVIDE reproduction with
heritable genomes. No GPU and no LLM required.

## Status

Honest status lives in [docs/status.md](docs/status.md); the continuation queue in
[docs/next.md](docs/next.md).

| Milestone | Scope | State |
|---|---|---|
| M0 | Project foundation: workspace, CI, hygiene | ✅ merged |
| M1 | Prior-art survey, research docs, WABIX v0 spec | ✅ merged |
| M2 | Minimal executable vertical slice (subject→cognition→world→log) | ✅ merged |
| M3 | Primordial ALife world v0 (energy, sensing, signaling, death) | ✅ merged |
| M4 | Reproduction / lineage v0 | ✅ merged |
| M5 | Continuity measurement & migration v0 | ✅ merged |
| M6 | Replay from event logs | ✅ merged |
| M7 | Local LLM cognition backend (≤8 GiB VRAM, quantized, shared) | ✅ merged |
| M8 | First real model-migration experiment | ✅ merged |
| M9 | Second world (network-v0) + world migration harness | ✅ merged |
| M10 | Reference-8GB benchmark | ✅ merged |

Milestones M0–M10 are implemented. Ongoing work: research experiments
(scarcity/selection, signaling conventions, world migration) and LLM
per-subject memory.

## Hardware contract

Reference development machine: consumer desktop, 8-core CPU, 32 GiB RAM,
**RTX 2070 SUPER (8 GiB VRAM)**. Everything must run on this class of hardware:

- one **shared** quantized cognition model (≤3B class), never one model per organism
- CPU-first simulation
- no paid cloud dependency for any reference experiment

## Build & test (no LLM required)

```sh
cargo build
cargo test
```

## Run the smallest experiment (no LLM required)

```sh
# Deterministic test world: subject + mock cognition + event log
cargo run -- run-test --ticks 50 --backend random --seed 123

# Primordial ALife world: 8 organisms, energy field, signaling, death
cargo run -- run-primordial --ticks 2000 --population 8 --backend energy-rule

# Inspect the event log of the last run
./target/release/transubstrate inspect runs/<experiment-id>/events.jsonl --tail 20
```

Every run writes `runs/<experiment-id>/events.jsonl` plus a `manifest.json`
describing the exact configuration.

## Run with a local LLM (optional)

```sh
# Requires a local Ollama or llama.cpp server on localhost
cargo run -- run-primordial --ticks 400 --population 2 \
  --backend llm:granite4.1:3b-q4_K_M
```

The model loads once and is shared by all organisms. Models are downloaded by
the user and never committed to this repository.

## Reproduce an experiment / inspect logs

```sh
# Re-execute with the same config (mock backends are bit-reproducible)
cargo run -- run-primordial --ticks 2000 --population 8 --backend energy-rule --seed 42

# Regenerate metrics from an existing log without re-running cognition
cargo run -- replay runs/<experiment-id>/events.jsonl

# Verify log integrity (sequential seq numbers, schema version)
cargo run -- verify runs/<experiment-id>/events.jsonl

# Human-readable digest of a log
cargo run -- inspect runs/<experiment-id>/events.jsonl --tail 20
```

## Benchmark (reference-8gb)

```sh
cargo run --release -- benchmark --out bench-report.json
```

Measures world tick throughput (1/8/32 organisms), mock cognition throughput,
peak host RAM, and event log size. See [docs/status.md](docs/status.md) for
measured numbers.

For exploration-grade runs (outcome-visible memory + sampling), pass
`--llm-temperature 0.7` to `run-world-migration`; see
[docs/experiments/outcome-exploration.md](docs/experiments/outcome-exploration.md)
for the observed experience-driven SIGNAL learning.

Multi-seed sweeps emit a machine-readable mean ± sd summary:

```sh
cargo run --release -- sweep-world-migration --seeds 1,7,42,99,123 \
    --backend energy-rule --summary summary.json
```

Memory is part of the subject, not the backend (SUBJECT != MODEL). The
sweep also runs a per-seed ablation arm — the same migration with the
memory buffer wiped at the boundary — and reports the carry − wipe
`ablation_effect` per dimension: a positive value means the carried
memory causally helped continuity on that seed. `--no-memory` runs the
pure ablation condition only.

```sh
cargo run --release -- sweep-world-migration --seeds 42,7,99,123,2026 \
    --backend llm:granite4.1:3b-q4_K_M --llm-temperature 0.3 \
    --summary summary.json
```

Optionally, set `TS_BENCH_LLM_MODEL=<model>` to also measure local LLM
cognition throughput (decisions/sec, mean latency). The section is omitted
when no endpoint is reachable, so the benchmark stays CI-safe.

## License

Apache-2.0. See [LICENSE](LICENSE).
