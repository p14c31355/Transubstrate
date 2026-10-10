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

## Status

Honest status lives in [docs/status.md](docs/status.md); the continuation queue in
[docs/next.md](docs/next.md). The project is in early bootstrap: core type system,
WABIX v0 contract, resource ledger, and event log exist as an executable skeleton with
tests. No world simulation yet — see the roadmap below.

## Roadmap

| Milestone | Scope | State |
|---|---|---|
| M0 | Project foundation: workspace, CI, hygiene | ✅ merged |
| M1 | Prior-art survey, research docs, WABIX v0 spec | 🚧 |
| M2 | Minimal executable vertical slice (subject→cognition→world→log) | planned |
| M3 | Primordial ALife world v0 (energy, sensing, signaling, death) | planned |
| M4 | Reproduction / lineage v0 | planned |
| M5 | Continuity measurement & migration v0 | planned |
| M6 | Replay from event logs | planned |
| M7 | Local LLM cognition backend (≤8 GiB VRAM, quantized, shared) | planned |
| M8 | First real model-migration experiment | planned |
| M9 | Second world + world migration | planned |
| M10 | Reference-8GB benchmark | planned |

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

## License

Apache-2.0. See [LICENSE](LICENSE).
