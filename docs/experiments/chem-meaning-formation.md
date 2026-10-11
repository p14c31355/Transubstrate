# Chemical-Field Meaning Formation — Detection Attempt

Status: ENGINEERING CLAIMS + OBSERVED (negative result, n=5 seeds)
Date: 2026-10-11
Harness: `run-primordial --markers N` + `analyze-chem` + unit tests

## QUESTION

With the chemical substrate live (PR#66: RELEASE channel 1 converts
organism energy into the cell's chemical field, sense-only, no payoff),
can a population coordinate MEANING onto chemical gradients? Nobody
defines "mark = resource" — the world attaches no semantics. If any
subject starts acting differently when it senses a mark, that is a
candidate meaning-formation signal.

## INSTRUMENTATION BUILT (this PR)

1. `WorldEvent::ChemicalMarked` — every channel-1 RELEASE is logged
   (who, where, how much). Marks are now first-class log data.
2. `nearby_chemical_0` observation channel — the strongest chemical
   reading among the cell and its 4 neighbors (mirrors the
   nearby_signal_0 non-local-sensing precedent, PR#55).
3. `EventPayload::DecisionObservation` — the observation snapshot at
   decision time is logged per cognition event, so post-hoc analyzers
   can correlate behavior with what the subject actually SAW.
4. `ChemMarkerBackend` (`--markers N`) — mixed-population runs:
   the first N organisms are deterministic surplus-markers (they WRITE
   the substrate); the rest (e.g. LLMs) observe it.
5. `analyze-chem <events.jsonl>` — the detection metric:

   - on-chem = decisions whose snapshot shows `nearby_chemical_0 > 0`
   - mark_awareness = P(ABSORB | on-chem) − P(ABSORB | all decisions)
   - observer_awareness = same, restricted to subjects that NEVER wrote
     a mark (writer exclusion is required: markers ABSORB LESS on their
     own marks by construction — they release instead — which would
     contaminate the correlation with rule behavior, not meaning).

## CALIBRATION (rule-only, deterministic)

`run-primordial --backend chem-marker` (4 markers): marks 104
(26 each × 4). mark_awareness +0.0121 ≈ 0 — the marker rule does not
read `chemical_0` (it reads `local_energy` only), so the detector
correctly reports "no reaction" where there is none. True negative.

A mixed rule run (4 markers + 2 energy-rule) shows the writer
contamination the observer split exists for: raw awareness −0.13
(markers absorb less on their own marks — rule artifact, not meaning).

## OBSERVED (LLM observers, granite4.1:3b-q4_K_M, temp 0.3, n=5 seeds)

4 ChemMarker + 2 LLM observers per run, 600 ticks, cognition every 10.

| seed | observer_awareness | observer on-chem n |
|------|--------------------|--------------------|
| 42   | +0.0077            | 117                |
| 7    | −0.0003            | 118                |
| 99   | −0.0007            | 118                |
| 123  | −0.0004            | 118                |
| 2024 | −0.0003            | 118                |

**All 5 seeds: |observer_awareness| < 0.01.** The LLM observers sensed
a chemical gradient in ~117 of their 120 decisions (the markers'
diffusion spreads the gradient across the map), and their ABSORB rate
while sensing was indistinguishable from their baseline in every seed.
**The LLMs behaved as if the chemical information did not exist.**

Structural observation (first run, before `nearby_chemical_0` was
added): LLM subjects at distance 8-11 cells from any mark had NO
observable channel carrying the marks (`chemical_0` is the subject's
own cell only) and never moved (MOVE 0 in all LLM runs to date). The
marks were literally invisible to them. The channel addition fixes
observability at the diffusion radius; the negative result above is
AFTER the fix — sensing works (on-chem n≈354/360 across seeds),
reaction does not follow.

## INTERPRETATION

- Chemical marks joined SIGNAL in the "sensed but ignored" category:
  with no world-attached payoff, LLM subjects do not coordinate meaning
  onto either channel. This SUPPORTS the hypothesis documented after
  PR#58 (convention formation needs a received-signal→payoff coupling,
  which would pre-define meaning and narrow the emergence claim).
- HYPOTHESIS (falsifiable): meaning formation on a semantics-free
  channel requires either (a) a payoff coupling (costs emergence
  claims), or (b) a population-scale interaction structure (n > 2
  observers, offspring inheriting the reaction) that a 2-observer run
  cannot provide. Both are testable later.
- The detector itself is calibrated and reusable: rule true-negative,
  writer-contamination known and split out, observation-based (fair to
  stationary subjects).

## CAVEATS

- LLM observer n=2 per run (one shared model); markers dominate the
  mark distribution by construction.
- awareness measures the ABSORB-rate margin only: a subject could
  "react" by adjusting SIGNAL/MOVE parameters without changing its
  action name — parameter-level reaction is invisible to this metric.
- Single model family (granite), temp 0.3, 600 ticks. Longer runs or
  higher temperatures may explore more.
