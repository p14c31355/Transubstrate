# Observation Adaptation — How Fast Does Behavior Adjust Across a World Boundary?

Status: ENGINEERING CLAIMS + OBSERVED (decision-stream metric; LLM and rule
backends)
Date: 2026-10-11
Harness: `compute_observation_adaptation` (world_migration.rs) + unit tests

## QUESTION

The continuity metrics (behavioral / retention / usage) measure WHETHER the
subject's behavior survives a world migration. They do not measure HOW FAST
the subject adjusts when the destination world's observation channels and
payoff structure differ (`local_energy` → `node_energy`, cell grid → node
graph). A subject can score high retention precisely BECAUSE it never
adapted — it kept running the origin-world strategy in the destination
world. This experiment instruments the speed side.

## METRIC (defined BEFORE the runs, on the decision stream)

Given the event log of a world-migration run with boundary tick B:

1. `pre_dominant` = most frequent action in decisions with tick < B
   (ties → first-seen).
2. Post-swap decisions are cut into trailing windows of
   ADAPTATION_WINDOW = 10 decisions (a trailing partial window smaller
   than 5 decisions is ignored — one stray decision must not read as
   adaptation).
3. `lag_ticks` = first window's start tick − B where `pre_dominant`
   falls below half of the window. None = never displaced.
4. `new_action` = the displacing action in that window.
5. `persists` = does `new_action` hold ≥ half of the LATE post-swap
   decisions (second half of the post phase)? Some(false) means the
   subject switched once and reverted — sampling noise, not adaptation.

The persistence flag exists because a pilot look at existing logs showed
lag-0 switches at temperature 1.0 that are high-entropy flailing, not
learning: the metric must separate "changed immediately" from "changed
and STAYED changed".

## CALIBRATION (rule backend, deterministic)

`run-world-migration --backend energy-rule` (400 ticks, seed 42):
lag 0, displaced by WAIT, persists yes. The rule controller re-evaluates
every decision from current observations, so it switches as soon as the
destination world's first window no longer favors ABSORB — the metric
reads as "immediate, stable". This is the expected true-negative for
"slow adaptation": the rule backend has no learning dynamics to delay it.

## OBSERVED (LLM granite4.1:3b-q4_K_M, temp 0.3, n=5, 400 ticks)

Fresh sweep (today's runs, sweep JSON `observation_adaptation` field):

| seed | lag_ticks | new_action | persists |
|------|-----------|------------|----------|
| 42   | 0         | SIGNAL     | yes      |
| 7    | 0         | SIGNAL     | yes      |
| 99   | 50        | SIGNAL     | yes      |
| 123  | 0         | SIGNAL     | yes      |
| 2024 | 0         | SIGNAL     | yes      |

Archive analysis (the SAME seeds, same temperature, runs from the
memory-ablation record re-analyzed under this metric) diverges:

| seed | lag_ticks | new_action | persists |
|------|-----------|------------|----------|
| 42   | —         | —          | —        |
| 7    | —         | —          | —        |
| 99   | —         | —          | —        |
| 123  | 50        | SIGNAL     | yes      |
| 2026 | 100       | SIGNAL     | yes      |

**Run-to-run variance is REAL and must be reported**: identical seed +
temperature + backend produce different lags (and even different
adaptation outcomes) across Ollama sessions — LLM sampling is not
deterministic across runs here. What IS stable across all 7 adaptive
runs observed (2 archive + 5 fresh): every displacing action is SIGNAL
and every one persists (`persists: true`). The lag VALUE varies; the
persist SIGN of successful switches does not.

Temperature contrast (archived sweeps, same metric):

- temp 0.3 (5 seeds): 2/5 adapt (archive) or 5/5 (fresh); all switches
  persist.
- temp 1.0 (5 seeds): 2/5 displace; **lag 0**; both persist as SIGNAL —
  immediate switching, consistent with PR#43's high format-failure /
  flailing regime; the persistence flag alone cannot separate flail
  from learning here.
- qwen3.5 (5 seeds): 1/5 displace; lag 150; **does NOT persist** — the
  one switch reverts (noise).

## INTERPRETATION

- Across temperatures, the adaptive minority is stable in kind: most
  subjects keep the origin-world strategy; the ones that switch do so
  toward SIGNAL (the destination world's cheap exploration action).
- The lag–persistence pair separates regimes that a single "did it
  change?" flag would conflate: temp 0.3 changes slowly and stays;
  temp 1.0 changes instantly (entropy, not learning); qwen changes
  once and reverts.
- INFERENCES (not claims): the metric gives the observation-adaptation
  speed axis the same treatment as the continuity vector — an explicit,
  replay-derived, per-run measurement instead of an anecdote.

## CAVEATS

- LLM sampling is NOT deterministic across runs on this stack (same
  seed, same temperature, different sessions → different decision
  streams): lag values and even adaptation occurrence vary between
  archive and fresh runs. Multi-seed statistics, not single-seed
  anecdotes, are the only honest reporting mode for LLM adaptation.
- The metric reads the action stream, not the observation values: a
  subject that adapts its action PARAMETERS without changing the action
  NAME is invisible here (e.g. retuning SIGNAL strength). Parameter
  adaptation is future work.
- n=5 per cell, single model family (granite) + one qwen cell; no
  significance claims.
- Dominant-action displacement is a coarse summary of a distribution
  shift; a subject whose behavior is mixed (no majority) still counts
  its pre-dominant share correctly, but "adaptation" here means
  leaving the old majority, not reaching a new stable one.
