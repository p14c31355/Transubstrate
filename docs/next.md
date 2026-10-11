# Next — Continuation Queue

Practical continuation queue. Each item is sized for one branch/PR.
Order reflects current value assessment; revisit after each merge.

Last updated: 2026-10-11 (after PR#19 — M0-M11, M9, M10, M11 all merged;
scarcity/competition experiments merged)

## 1. M11 continuation — memory retention continuity (highest value)

- [x] Per-subject memory in the primordial population runner: the bounded
      `memories` map is wired (PR#17) — LLM organisms reason over past
      decisions + reward signals.
- [ ] Memory-retention continuity dimension: fingerprint memory state
      before/after backend swap in the migration harness; add
      `memory_retention` dimension to ContinuityVector.
- [x] Migration harness carries the SAME memory buffer across the migration
      boundary (memory belongs to the subject, not the backend).
- [ ] LLM prompt already renders memory entries — verify end-to-end with
      granite4.1:3b-q4_K_M and record decision quality vs memory length.
- [ ] Multi-seed statistics for the LLM migration effect (n>=5, report
      mean ± sd; stop single-run anecdotes).

## 2. Research experiments

- [x] Scarcity/drift: NEGATIVE RESULT at default mutation (drift +0.0006,
      below ±0.02 band) — PR#11. Knobs documented: stronger mutation,
      longer runs, sharper fitness differential.
- [x] Two-phenotype competition: POSITIVE selection measured — shareA 0.759
      mean, 5/5 seeds, mechanism confirmed (PR#12).
- [ ] Signaling-convention detection: with signal cost > 0, do organisms
      that signal near resource acquire followers? Define detection metric
      before running.
- [ ] Chemical-field meaning formation (the semantics-free substrate is
      currently unused by controllers).
- [ ] Carrying capacity: tune regen vs consumption so the field actually
      binds (currently regen dominates at 0.01/cell/tick); rerun
      reproduction experiments under binding scarcity.

## 3. M9 continuation — world migration experiments

- [ ] World-migration experiment with LLM cognition: subject migrates
      primordial → network mid-run; measure adaptation (action distribution
      shift) and continuity against a same-world control.
- [ ] Observation-adaptation metric: how quickly does behavior adapt to the
      new manifest's channel names (node_energy vs local_energy)?

## 4. Infrastructure

- [ ] Emit ExperimentManifest automatically next to every event log for
      the migration and scarcity harnesses (test/primordial already emit;
      migration/scarcity emission verified wired — verify with a fresh run).
- [ ] Regenerate population samples with real field_energy (extend
      SubjectState POPULATION payload).
- [ ] Consider `worlds/` + `experiments/` layout split only when crate
      boundaries actually demand it (avoid directory theater).

## 5. M10 expansion

- [ ] Record endpoint model metadata (VRAM footprint) in the benchmark
      report when the endpoint exposes it.
- [ ] Cognition throughput for the LLM backend (decisions/sec, tokens/sec)
      as a benchmark section.

## Explicitly deferred (do not start without revisiting)

- Physical robot adapters (WABIX safety model needs CapabilityGrant design
  first; simulations are sufficient for current research questions).
- Remote OpenAI-compatible backend (optional; no credentials assumed).
- Multi-machine migration (needs a process boundary story first).
