# Changelog

All notable changes to Transubstrate are documented here.

## [Unreleased]

### Added (feat/project-foundation)
- Cargo workspace: `ts-core` (core library) + `transubstrate` (CLI binary).
- Typed identity system: SubjectId, IncarnationId, CognitionBackendId,
  MachineId, BodyId, WorldId, LineageId, MigrationEventId, ExperimentId —
  administrative identity explicitly separated from measured continuity.
- Resource ledger: append-only, bounded memory, exact totals across
  compaction; kinds for energy, cognition events, tokens, CPU time, storage.
- WABIX v0 skeleton: capability manifests, typed observations/actions,
  shared validation, WABIX errors.
- Cognition backend ABI: bounded decision context, typed decisions,
  schema-violation-as-typed-error, cognition cost charging.
- Append-only event log (JSONL sink + bounded memory) with seq numbers and
  schema version.
- Deterministic SplitMix64 RNG.
- Subject runtime skeleton: cadence-based cognition, tick loop, staging of
  validated actions.
- CI: fmt, clippy, test, release build on GitHub Actions.
- README, CONTRIBUTING, SECURITY, .gitignore.

### Added (feat/vertical-slice, unmerged)
- Cognition backends: EchoBackend (deterministic baseline), RandomBackend
  (seeded), EnergyRuleBackend (positive control), MalformedBackend (safe
  failure testing).
- TestWorld: deterministic two-action world exercising the full WABIX loop.
- Experiment harness: config → run → summary + event log, with maintenance
  costs, death by energy exhaustion, cadence-based cognition.
- CLI: `transubstrate run-test` (real experiment execution) and
  `transubstrate inspect` (event log digest).
- 26 unit tests passing.
