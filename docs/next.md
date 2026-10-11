# Next — Continuation Queue

Practical continuation queue. Each item is sized for one branch/PR.
Order reflects current value assessment; revisit after each merge.

Last updated: 2026-10-11 (after PR#46 — world-migration A/B, cross-model
stats, signal pull, outcome learning, temperature sweep, memory_usage, and
the batch sweep runner all merged)

## 1. Experience-driven continuity (highest value)

- [x] Outcome-visible memory: LLM prompt renders past decisions WITH their
      observed outcomes (PR#36); leakage test passes.
- [x] Outcome-driven exploration: at temperature 0.7 the LLM discovers and
      repeats SIGNAL from outcome evidence ("Repeat successful SIGNAL from
      past ticks"); 1/5 seeds adapted post-swap (PR#37,
      docs/experiments/outcome-exploration.md).
- [x] Multi-seed outcome-learning statistics: sweep-world-migration (PR#39)
      batch-collected temp 0.3/0.7 at n=5 each. Temp 0.3 dominates: 3/5
      SIGNAL learning with 0 deaths (vs 2/5 with 2 deaths at 0.7);
      behavioral 0.65 ± 0.314 (bimodal) — PR#40,
      docs/experiments/temperature-sweep.md. Temp 1.0 cell collected
      (PR#43): 2/4 valid seeds learned SIGNAL, 1 schema-rejection
      (`ABSORBT` typo) — WABIX fail-safe OBSERVED end-to-end.
- [x] Boundary-anchored memory sampling: `memory_usage` dimension (PR#42,
      #44) — of the actions remembered AT the boundary, what fraction was
      re-enacted post-swap. Calibrated: rule→LLM 1.0, rule→echo 0.0,
      world-migration signal-seeker 0.5 (SIGNAL→ABSORB flip = partial
      usage). Wired into BOTH harnesses; not masked by FIFO roll-off.
- [x] Cross-model outcome learning: qwen3.5 explores SIGNAL+HOP actively in
      the signal-pull world with no channel meaning (4/4 complete runs, one
      energy death per run) — docs/experiments/cross-model-outcome-learning.md.
- [x] Record memory_usage numbers for LLM runs: temp-0.3 5-dimension sweep
      (PR#47 record, n=5) — 4/5 usage 1.0 (swap-robust), 1 seed 0.0 across
      all dimensions (full strategy replacement ABSORB→SIGNAL).
- [x] Memory ablation (CAUSAL): `memory_carry: bool` + boundary wipe in BOTH
      harnesses; `--no-memory` CLI flag; per-seed ablation arm in
      `sweep-world-migration` (carry − wipe effect in the JSON summary).
      Rule calibration 0.0 (true negative); LLM n=5: behavioral +0.50,
      retention +0.55, usage +0.80 — with a DISCLOSED pre-phase confound
      (LLM sampling makes the wipe arm's pre phase diverge); a
      same-trajectory ablation is queued
      (docs/experiments/memory-ablation.md).
- [x] Same-trajectory ablation: the wipe arm REPLAYS the carry arm's
      pre-swap decisions — 5/5 seeds pre-phase exactly matched; ablation
      effect behavioral +0.68 / retention +0.60 / usage +1.00; wipe arm
      jumps to uniform SIGNAL saturation 5/5 while the carry arm continues
      its boundary strategy — memory causally shapes post-swap behavior
      (docs/experiments/memory-ablation.md).

## 2. Research experiments

- [x] Scarcity/drift: NEGATIVE RESULT at default mutation (drift +0.0006,
      below ±0.02 band) — PR#11. Knobs documented: stronger mutation,
      longer runs, sharper fitness differential.
- [x] Two-phenotype competition: POSITIVE selection measured — shareA 0.759
      mean, 5/5 seeds, mechanism confirmed (PR#12).
- [x] Signal-driven energy pull: SIGNAL now has a resource payoff;
      signal-seeker flips SIGNAL→ABSORB across the boundary (PR#30–#33,
      docs/experiments/signal-pull.md).
- [x] Signaling-follower detection (multi-subject): attraction 5/5 seeds and
      ~2.1× intake vs null — the pull mechanism creates the follower-payoff
      substrate; survival economics fail under current parameters (all
      followers die in 600 ticks) — negative result disclosed
      (docs/experiments/signaling-followers.md).
- [x] Survival-tuned follower run: sweep-sized energy budget (40 units)
      lets 5/6 followers survive 600 ticks in 5/5 seeds; intake ≈2.6× null
      (up from 2.1×); attraction narrows to a small margin (4/5) — harvest
      is the robust effect; signal-gradient steering queued
      (docs/experiments/signaling-followers.md).
- [x] Signal-gradient observation: nearby_signal_left/right channels —
      followers converge on the emitter (proximity 0.98-0.997 vs 0.28
      sweeping); emergent carrying capacity: only 2/6 survive because node
      capacity caps co-residency at the beacon; cue (signal diffusion) is
      separable from payoff (pull energy, 5/5 seeds)
      (docs/experiments/signaling-followers.md).
- [ ] Multi-subject LLM signaling run: the rule-level economics now sustain
      followers (survival budget + gradient steering + carrying capacity
      observed).
- [ ] Chemical-field meaning formation (the semantics-free substrate is
      still unused by controllers).
- [ ] Carrying capacity: tune regen vs consumption so the field actually
      binds (currently regen dominates at 0.01/cell/tick); rerun
      reproduction experiments under binding scarcity. NOTE: an emergent
      node-level carrying capacity is ALREADY OBSERVED in the network world
      (gradient-steered followers: 2/6 survive at the emitter —
      docs/experiments/signaling-followers.md); the remaining work is the
      primordial grid's field-level bind.

## 3. M9 continuation — world migration

- [x] Runnable end-to-end with control + effect vector (PR#22, #28).
- [ ] Observation-adaptation metric: how quickly does behavior adapt to the
      new manifest's channel names (node_energy vs local_energy)? Needs the
      active decision surface above.
- [ ] Multi-subject world migration: currently single-subject; migrate a
      population and measure per-subject continuity spread.

## 4. Infrastructure

- [x] Batch/seed-sweep runner: `sweep-world-migration --seeds 1,7,42,...`
      emits a mean ± sd JSON summary per dimension (PR#39). CSV output and a
      run-migration sweep remain open.
- [ ] Emit ExperimentManifest automatically next to every event log for
      the migration and scarcity harnesses (test/primordial already emit;
      migration/scarcity emission verified wired — verify with a fresh run).
- [ ] Regenerate population samples with real field_energy (extend
      SubjectState POPULATION payload).

## 5. M10 expansion

- [x] LLM cognition throughput section in the benchmark (PR#31,
      `TS_BENCH_LLM_MODEL` gate; 3.29 decisions/s on the reference GPU).
- [ ] Record endpoint model metadata (VRAM footprint) in the benchmark
      report when the endpoint exposes it.

## Explicitly deferred (do not start without revisiting)

- Physical robot adapters (WABIX safety model needs CapabilityGrant design
  first; simulations are sufficient for current research questions).
- Remote OpenAI-compatible backend (optional; no credentials assumed).
- Multi-machine migration (needs a process boundary story first).
