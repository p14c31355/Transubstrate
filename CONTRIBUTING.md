# Contributing to Transubstrate

## Workflow

We use a branch → PR → review → CI → merge workflow. `main` is the canonical
stable state: always buildable, always testable.

1. Create a descriptive branch (`feat/...`, `fix/...`, `research/...`, `docs/...`, `bench/...`)
2. Implement with tests
3. `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`
4. Push, open a PR, fill out the PR template honestly
5. Merge after CI + review

## PR quality bar

Every substantive PR describes: what changed, why, how it was tested, known
limitations, and follow-up work. Experimental PRs additionally state
HYPOTHESIS / CONFIGURATION / SEEDS / METRICS / OBSERVED RESULT / INTERPRETATION
/ CAVEATS — and never dress hypotheses up as observed results.

## Scientific integrity rules

- Do not claim consciousness, sentience, life, identity persistence, evolution,
  or emergence without an operational definition and evidence.
- Label claims: OBSERVED / COMPUTED / INFERRED / HYPOTHESIZED / SPECULATIVE.
- Keep negative results. Do not cherry-pick runs.
- Do not document planned functionality as if it exists.

## Engineering rules

- VRAM must not scale linearly with organism count (shared cognition model).
- The system must remain fully testable with no LLM installed.
- Determinism first: seeded RNG, event logs, replayable experiments.
- No secrets, no model weights, no large run outputs in git.

## Commit style

Short imperative subject lines; body explains motivation. PRs are squash-
merged when small and coherent, merge-committed when history has research
value.
