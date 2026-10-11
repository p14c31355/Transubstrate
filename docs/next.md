# Next — Continuation Queue

Practical continuation queue. Each item is sized for one branch/PR.
Order reflects current value assessment; revisit after each merge.

Last updated: 2026-10-11 (after PR#37 — M0-M11, M9, M10, M11 all merged;
world-migration A/B, cross-model stats, signal pull, and outcome-driven
exploration recorded)

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
- [ ] Cross-model outcome learning: does qwen3.5 (thinking-style, reasoning
      off) also learn SIGNAL from outcomes at temp>0?
- [ ] Record memory_usage numbers for LLM runs (temp sweep re-run with the
      5-dimension vector; rule/echo calibrations only so far).

## 2. Research experiments

- [x] Scarcity/drift: NEGATIVE RESULT at default mutation (drift +0.0006,
      below ±0.02 band) — PR#11. Knobs documented: stronger mutation,
      longer runs, sharper fitness differential.
- [x] Two-phenotype competition: POSITIVE selection measured — shareA 0.759
      mean, 5/5 seeds, mechanism confirmed (PR#12).
- [x] Signal-driven energy pull: SIGNAL now has a resource payoff;
      signal-seeker flips SIGNAL→ABSORB across the boundary (PR#30–#33,
      docs/experiments/signal-pull.md).
- [ ] Signaling-convention detection (multi-subject): with signal pull,
      do OTHER subjects flock to/harvest at another's signal? Define the
      detection metric before running; single-subject learning is already
      OBSERVED (PR#37).
- [ ] Chemical-field meaning formation (the semantics-free substrate is
      still unused by controllers).
- [ ] Carrying capacity: tune regen vs consumption so the field actually
      binds (currently regen dominates at 0.01/cell/tick); rerun
      reproduction experiments under binding scarcity.

## 3. M9 continuation — world migration

- [x] Runnable end-to-end with control + effect vector (PR#22, #28).
- [ ] Observation-adaptation metric: how quickly does behavior adapt to the
      new manifest's channel names (node_energy vs local_energy)? Needs the
      active decision surface above.
- [ ] Multi-subject world migration: currently single-subject; migrate a
      population and measure per-subject continuity spread.

## 4. Infrastructure

- [ ] Batch/seed-sweep runner: `run-migration --seeds 1,7,42,...` emitting a
      summary CSV/JSON so multi-seed statistics stop being hand-collected.
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
