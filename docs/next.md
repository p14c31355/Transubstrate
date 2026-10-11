# Next — Continuation Queue

Practical continuation queue. Each item is sized for one branch/PR.
Order reflects current value assessment; revisit after each merge.

## 1. Finish M1 documentation (in progress)

- [ ] docs/prior-art.md — write from the verified source list already
      collected (OpenLife arXiv:2606.31046, Lenia, Generative Agents
      arXiv:2304.03442, Shachi arXiv:2509.21862, AlphaEvolve, MemGPT/Letta
      arXiv:2310.08560, W3C WoT TD 1.1, ROS2/ros2_control, Viam, MCP,
      autopoiesis S0303264723001119, Parfit; UNVERIFIED: Exuvia, Enoch,
      OpenRGD). Note: two delegated survey attempts died on API timeouts;
      write it directly from the collected sources.
- [ ] docs/research-questions.md — pin down the three Goal questions with
      operational definitions and falsifiable experiment designs.
- [ ] docs/architecture.md — describe what actually exists now (crates,
      traits, data flow) plus the WABIX v0 spec and its versioning policy.
- [ ] docs/continuity.md — document the continuity vector, dimensions, and
      the control methodology; keep separate from architecture.
- [ ] docs/resource-model.md — ledger semantics, cost model, cognition
      cadence economics.

## 2. M7 — Local LLM cognition backend (next major milestone)

- [ ] Add `llama-cpp` backend behind the CognitionBackend trait:
      shared model instance, per-decision bounded context, typed decision
      extraction (JSON mode or constrained grammar), token cost reporting
      into the resource ledger.
- [ ] 8 GiB VRAM contract: single shared model, ≤3B quantized (Q4_K_M class),
      batch=1, CPU fallback path. Expose a resource profile concept.
- [ ] Cognition cadence economics: measure organism-count vs cognition-event
      cost; document that VRAM does not scale with organism count.
- [ ] Safe failure: malformed model output → logged rejection (already typed
      via CognitionError; test with real model output).
- [ ] Model acquisition docs (user downloads gguf; never committed).

## 3. M8 — Real model migration experiment

- [ ] Run rule-backend → local-LLM-backend migration on the existing
      harness; first migration where continuity measurement is scientifically
      meaningful (the backend carries per-subject memory).
- [ ] Add per-subject memory to the LLM backend path (bounded, summarized);
      measure memory-retention dimension in the continuity vector.
- [ ] Multi-seed statistics: run N seeds, report mean ± spread of migration
      effect; stop reporting single-run anecdotes.

## 4. Primordial world research experiments

- [ ] Carrying capacity: tune regen vs consumption so the field actually
      binds (currently regen dominates); rerun reproduction experiments.
- [ ] Genome evolution under selection: does absorb_affinity drift when
      energy is scarce? Define 'observed adaptation' operationally first.
- [ ] Signaling-convention detection: with signal cost > 0, do organisms
      that signal near resource acquire followers? Define detection metric
      before running.
- [ ] Chemical-field meaning formation (the semantics-free substrate is
      currently unused by controllers).

## 5. Infrastructure

- [ ] Emit ExperimentManifest automatically next to every event log
      (manifest is defined; emission not wired).
- [ ] Regenerate population samples with real field_energy (extend
      SubjectState POPULATION payload).
- [ ] Second world (M9): a structurally different WABIX world (e.g.
      continuous space or network world) for world-migration experiments.
- [ ] Consider `worlds/` + `experiments/` layout split only when crate
      boundaries actually demand it (avoid directory theater).

## Explicitly deferred (do not start without revisiting)

- Physical robot adapters (WABIX safety model needs CapabilityGrant design
  first; simulations are sufficient for current research questions).
- Remote OpenAI-compatible backend (optional; no credentials assumed).
- Multi-machine migration (needs a process boundary story first).
