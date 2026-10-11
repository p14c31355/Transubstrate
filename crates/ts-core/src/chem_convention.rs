//! Chemical-convention analysis: does anyone's BEHAVIOR correlate with the
//! chemical marks?
//!
//! The chemical substrate (RELEASE channel 1) is sense-only by design —
//! nothing eats it, so the world pre-defines no meaning for a mark. Meaning
//! formation, if it happens at all, can only show up as a BEHAVIORAL
//! correlation: subjects that SENSE a chemical gradient behave differently
//! from the run baseline. This module computes that correlation from an
//! event log AFTER the run; it never feeds back into decisions.
//!
//! Operational definition (fixed before running any experiment):
//!
//! - A "mark" is a `WorldEvent::ChemicalMarked` record (subject, pos, tick,
//!   amount). Only channel-1 RELEASEs produce these.
//! - A decision is "on-chem" when its `DecisionObservation` snapshot shows
//!   `nearby_chemical_0 > 0` — the subject SENSED a chemical gradient at
//!   decision time (own cell or 4-neighbor reach; observation-based, not
//!   position-based, so stationary subjects are measured fairly).
//! - `on_chem_absorb_rate` = ABSORB fraction among on-chem decisions.
//! - `baseline_absorb_rate` = ABSORB fraction across ALL decisions.
//! - `mark_awareness` = on_chem − baseline. Positive = subjects act
//!   differently while sensing a mark (candidate meaning-formation
//!   signal); ≈0 = marks are ignored; negative = marks deter ABSORB.
//! - The OBSERVER condition repeats the computation restricted to subjects
//!   that NEVER wrote a mark themselves. A marker's own surplus rule makes
//!   it ABSORB LESS on its own marks by construction (it releases instead),
//!   so writer decisions would contaminate the correlation — the observer
//!   split is the actual meaning-formation question: do non-writers react?
//! - Decisions made before any mark existed are never on-chem (the
//!   observation snapshot is recorded at decision time, so temporal
//!   causality holds automatically).

use crate::event_log::{EventPayload, EventRecord, read_log};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of the chemical-convention analysis of one event log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChemConventionSummary {
    /// Total RELEASE-channel-1 marks in the run.
    pub marks_total: u64,
    /// Marks per writing subject (who wrote the substrate at all).
    pub per_subject_marks: std::collections::BTreeMap<String, u64>,
    /// ABSORB rate among decisions made while SENSING a chemical gradient.
    pub on_chem_absorb_rate: Option<f32>,
    /// ABSORB rate across all decisions (the run baseline).
    pub baseline_absorb_rate: f32,
    /// on_chem − baseline. None when no on-chem decisions were observed
    /// (nobody ever sensed a mark, or nothing was ever marked).
    pub mark_awareness: Option<f32>,
    /// Decisions observed in the on-chem condition.
    pub on_chem_n: usize,
    /// Total decisions observed (baseline denominator).
    pub decisions_n: usize,
    /// ABSORB rate among on-chem decisions by subjects that never wrote a
    /// mark themselves (the observer condition).
    pub observer_absorb_rate: Option<f32>,
    /// observer on-chem − observer baseline (non-writer decisions overall).
    /// None when the observer condition has no data.
    pub observer_awareness: Option<f32>,
    /// Decisions observed in the observer on-chem condition.
    pub observer_on_chem_n: usize,
}

/// Analyze one event log for chemical-mark/behavior correlation.
pub fn analyze_chem_convention(records: &[EventRecord]) -> ChemConventionSummary {
    // Marks: who wrote, how much, and how many.
    let mut per_subject_marks: std::collections::BTreeMap<String, u64> =
        std::collections::BTreeMap::new();
    let mut marks_total = 0u64;

    // Observation snapshots at decision time: (subject, tick) -> chemical
    // sensing strength. A later snapshot for the same (subject, tick)
    // cannot happen (one decision per cognition event), so insert wins.
    let mut chem_sensed: std::collections::BTreeMap<(String, u64), f32> =
        std::collections::BTreeMap::new();

    // Decisions in encounter order: (subject, tick, action).
    let mut decisions: Vec<(String, u64, Option<String>)> = Vec::new();

    for rec in records {
        match &rec.payload {
            EventPayload::WorldEvents { events, .. } => {
                for e in events {
                    if let crate::wabix::WorldEvent::ChemicalMarked { subject, .. } = e {
                        *per_subject_marks.entry(subject.clone()).or_insert(0) += 1;
                        marks_total += 1;
                    }
                }
            }
            EventPayload::DecisionObservation {
                subject_id,
                tick,
                observations,
            } => {
                let chem = observations
                    .iter()
                    .find(|(c, _)| c == "nearby_chemical_0")
                    .map(|(_, v)| *v)
                    .unwrap_or(0.0);
                chem_sensed.insert((subject_id.clone(), *tick), chem);
            }
            EventPayload::DecisionMade {
                subject_id,
                tick,
                decision,
            } => {
                let action = decision.action.as_ref().map(|a| a.name.clone());
                decisions.push((subject_id.clone(), *tick, action));
            }
            _ => {}
        }
    }

    // Baseline over all decisions.
    let decisions_n = decisions.len();
    let baseline_absorb = decisions
        .iter()
        .filter(|(_, _, a)| a.as_deref() == Some("ABSORB"))
        .count();

    // On-chem condition: the decision's snapshot sensed a chemical.
    let mut on_chem_absorb = 0usize;
    let mut on_chem_n = 0usize;
    let mut observer_absorb = 0usize;
    let mut observer_on_chem_n = 0usize;
    for (subject, tick, action) in &decisions {
        let sensed = chem_sensed
            .get(&(subject.clone(), *tick))
            .copied()
            .unwrap_or(0.0);
        if sensed <= 0.0 {
            continue;
        }
        let is_absorb = action.as_deref() == Some("ABSORB");
        on_chem_n += 1;
        if is_absorb {
            on_chem_absorb += 1;
        }
        // Observer condition: subjects that never wrote a mark.
        if !per_subject_marks.contains_key(subject) {
            observer_on_chem_n += 1;
            if is_absorb {
                observer_absorb += 1;
            }
        }
    }

    let on_chem_rate = if on_chem_n > 0 {
        Some(on_chem_absorb as f32 / on_chem_n as f32)
    } else {
        None
    };
    let baseline_rate = if decisions_n > 0 {
        Some(baseline_absorb as f32 / decisions_n as f32)
    } else {
        None
    };

    // Observer baseline: ABSORB rate across ALL non-writer decisions.
    let observer_total = decisions
        .iter()
        .filter(|(s, _, _)| !per_subject_marks.contains_key(s))
        .count();
    let observer_baseline_absorb = decisions
        .iter()
        .filter(|(s, _, _)| !per_subject_marks.contains_key(s))
        .filter(|(_, _, a)| a.as_deref() == Some("ABSORB"))
        .count();
    let observer_baseline_rate = if observer_total > 0 {
        Some(observer_baseline_absorb as f32 / observer_total as f32)
    } else {
        None
    };
    let observer_rate = if observer_on_chem_n > 0 {
        Some(observer_absorb as f32 / observer_on_chem_n as f32)
    } else {
        None
    };

    ChemConventionSummary {
        marks_total,
        per_subject_marks,
        on_chem_absorb_rate: on_chem_rate,
        baseline_absorb_rate: baseline_rate.unwrap_or(0.0),
        mark_awareness: match (on_chem_rate, baseline_rate) {
            (Some(a), Some(b)) => Some(a - b),
            _ => None,
        },
        on_chem_n,
        decisions_n,
        observer_absorb_rate: observer_rate,
        observer_awareness: match (observer_rate, observer_baseline_rate) {
            (Some(a), Some(b)) => Some(a - b),
            _ => None,
        },
        observer_on_chem_n,
    }
}

/// Convenience: read a log file and analyze it.
pub fn analyze_chem_convention_file(path: &Path) -> Result<ChemConventionSummary, String> {
    let records = read_log(path).map_err(|e| e.to_string())?;
    Ok(analyze_chem_convention(&records))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::Decision;
    use crate::event_log::EventRecord;
    use crate::wabix::Action;

    fn rec(seq: u64, payload: EventPayload) -> EventRecord {
        EventRecord {
            seq,
            log_version: crate::event_log::EVENT_LOG_VERSION,
            payload,
        }
    }

    fn decision(action: Option<(&str, Vec<f32>)>) -> Decision {
        Decision {
            action: action.map(|(name, params)| Action {
                name: name.into(),
                params,
            }),
            rationale: "test".into(),
        }
    }

    fn obs_snapshot(seq: u64, subject: &str, tick: u64, nearby_chem: f32) -> EventRecord {
        rec(
            seq,
            EventPayload::DecisionObservation {
                subject_id: subject.into(),
                tick,
                observations: vec![
                    ("local_energy".into(), 0.5),
                    ("nearby_chemical_0".into(), nearby_chem),
                ],
            },
        )
    }

    #[test]
    fn no_marks_gives_null_awareness() {
        let records = vec![
            rec(
                0,
                EventPayload::DecisionObservation {
                    subject_id: "a".into(),
                    tick: 1,
                    observations: vec![("nearby_chemical_0".into(), 0.0)],
                },
            ),
            rec(
                1,
                EventPayload::DecisionMade {
                    subject_id: "a".into(),
                    tick: 1,
                    decision: decision(Some(("ABSORB", vec![1.0]))),
                },
            ),
        ];
        let s = analyze_chem_convention(&records);
        assert_eq!(s.marks_total, 0);
        assert_eq!(s.mark_awareness, None);
        assert_eq!(s.on_chem_n, 0);
        assert_eq!(s.decisions_n, 1);
        assert_eq!(s.baseline_absorb_rate, 1.0);
    }

    #[test]
    fn on_chem_absorb_vs_baseline() {
        // Writer "m" marks and then WAITS on its own mark (sensed).
        // Observer "a" senses the gradient and ABSORBS; observer "b" senses
        // nothing and WAITS. Baseline = 1/3; on-chem = 1/2; observer:
        // on-chem 1/1, baseline (a+b decisions) 1/2 → awareness +0.5.
        let records = vec![
            rec(
                0,
                EventPayload::WorldEvents {
                    tick: 0,
                    events: vec![crate::wabix::WorldEvent::ChemicalMarked {
                        subject: "m".into(),
                        pos: (2, 2),
                        amount: 1.0,
                    }],
                },
            ),
            obs_snapshot(1, "m", 1, 0.4),
            obs_snapshot(2, "a", 1, 0.2),
            obs_snapshot(3, "b", 1, 0.0),
            rec(
                4,
                EventPayload::DecisionMade {
                    subject_id: "m".into(),
                    tick: 1,
                    decision: decision(None),
                },
            ),
            rec(
                5,
                EventPayload::DecisionMade {
                    subject_id: "a".into(),
                    tick: 1,
                    decision: decision(Some(("ABSORB", vec![1.0]))),
                },
            ),
            rec(
                6,
                EventPayload::DecisionMade {
                    subject_id: "b".into(),
                    tick: 1,
                    decision: decision(None),
                },
            ),
        ];
        let s = analyze_chem_convention(&records);
        assert_eq!(s.marks_total, 1);
        assert_eq!(s.per_subject_marks.get("m"), Some(&1));
        assert_eq!(s.on_chem_n, 2);
        assert_eq!(s.on_chem_absorb_rate, Some(0.5));
        assert_eq!(s.baseline_absorb_rate, 1.0 / 3.0);
        assert_eq!(s.observer_on_chem_n, 1);
        assert_eq!(s.observer_absorb_rate, Some(1.0));
        assert_eq!(s.observer_awareness, Some(0.5));
    }

    #[test]
    fn writer_excluded_from_observer_condition() {
        // Writer senses its own mark and WAITS — that decision must be in
        // the overall on-chem condition but NOT in the observer condition.
        let records = vec![
            rec(
                0,
                EventPayload::WorldEvents {
                    tick: 0,
                    events: vec![crate::wabix::WorldEvent::ChemicalMarked {
                        subject: "m".into(),
                        pos: (2, 2),
                        amount: 1.0,
                    }],
                },
            ),
            obs_snapshot(1, "m", 1, 0.4),
            rec(
                2,
                EventPayload::DecisionMade {
                    subject_id: "m".into(),
                    tick: 1,
                    decision: decision(None),
                },
            ),
        ];
        let s = analyze_chem_convention(&records);
        assert_eq!(s.on_chem_n, 1);
        assert_eq!(s.observer_on_chem_n, 0);
        assert_eq!(s.observer_awareness, None);
    }
}
