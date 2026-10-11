//! M5: Migration events and measured continuity.
//!
//! Core principle: administrative identity (SubjectId) persists across
//! substrate changes, but continuity is a MEASURED quantity, not an
//! assertion. A migration records what changed (dimension, from, to); a
//! continuity experiment compares pre/post behavior against a control that
//! experiences no migration.
//!
//! This module provides:
//! - MigrationEvent: typed record of one substrate replacement
//! - ContinuityVector: interpretable per-dimension continuity metrics
//!   (NOT collapsed into a single score without documentation)
//! - Behavioral fingerprinting: deterministic comparison of action
//!   distributions pre/post migration
//! - Experiment harness: A/B design with control runs

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One substrate replacement event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MigrationEvent {
    pub migration_id: String,
    pub subject_id: String,
    /// Which dimension changed: "cognition_backend" | "machine" | "body" |
    /// "world" | combinations joined by "+".
    pub dimension: String,
    /// Identity of the old substrate (e.g. backend implementation id).
    pub from: String,
    /// Identity of the new substrate.
    pub to: String,
    /// World tick at which the migration happened.
    pub tick: u64,
    /// Incarnation id changes with every migration boundary.
    pub old_incarnation: String,
    pub new_incarnation: String,
}

impl MigrationEvent {
    pub fn new(
        subject_id: &str,
        dimension: &str,
        from: &str,
        to: &str,
        tick: u64,
        old_incarnation: String,
        new_incarnation: String,
    ) -> Self {
        // Deterministic id from content (migration ids must be comparable
        // across replays; time-based UUIDs would break that).
        let content = format!("{subject_id}|{dimension}|{from}|{to}|{tick}");
        Self {
            migration_id: format!("mig-{:016x}", fnv1a64(content.as_bytes())),
            subject_id: subject_id.into(),
            dimension: dimension.into(),
            from: from.into(),
            to: to.into(),
            tick,
            old_incarnation,
            new_incarnation,
        }
    }
}

/// FNV-1a 64-bit hash — small, deterministic, stable across platforms.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Action distribution: action name -> count. Deterministic fingerprint
/// basis for behavioral continuity comparison.
pub type ActionDistribution = BTreeMap<String, u64>;

/// Build an action distribution from (action_name) samples.
pub fn action_distribution<S: AsRef<str>>(actions: &[S]) -> ActionDistribution {
    let mut dist = ActionDistribution::new();
    for a in actions {
        *dist.entry(a.as_ref().to_string()).or_insert(0) += 1;
    }
    dist
}

/// Behavioral fingerprint: normalized action distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehaviorFingerprint {
    /// action name -> fraction in [0,1] of all actions.
    pub fractions: BTreeMap<String, f32>,
    pub total_actions: u64,
}

impl BehaviorFingerprint {
    pub fn from_distribution(dist: &ActionDistribution) -> Self {
        let total: u64 = dist.values().sum();
        let fractions = dist
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    if total > 0 {
                        *v as f32 / total as f32
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        Self {
            fractions,
            total_actions: total,
        }
    }

    /// L1 distance between two fingerprints: sum |f_a - f_b| over the union
    /// of action names. Range [0, 2]; 0 = identical distributions.
    pub fn l1_distance(&self, other: &BehaviorFingerprint) -> f32 {
        let mut keys: Vec<&String> = self.fractions.keys().collect();
        for k in other.fractions.keys() {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        keys.iter()
            .map(|k| {
                let a = self.fractions.get(*k).copied().unwrap_or(0.0);
                let b = other.fractions.get(*k).copied().unwrap_or(0.0);
                (a - b).abs()
            })
            .sum()
    }
}

/// One measured continuity dimension.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuityDimension {
    pub name: String,
    /// Value in [0,1]; 1 = fully continuous, 0 = fully discontinuous.
    /// Each dimension documents its own computation.
    pub value: f32,
    /// How this value was computed (kept alongside the number — no magic).
    pub computation: String,
}

/// Interpretable continuity vector. NOT aggregated into a single score by
/// default; `documented_aggregate()` exists for convenience but preserves
/// all components and documents its arbitrariness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuityVector {
    pub subject_id: String,
    pub migration_id: Option<String>,
    pub dimensions: Vec<ContinuityDimension>,
}

impl ContinuityVector {
    /// Behavioral continuity: 1 - L1/2 (so identical = 1.0, disjoint = 0.0).
    pub fn behavioral(
        pre: &BehaviorFingerprint,
        post: &BehaviorFingerprint,
    ) -> ContinuityDimension {
        let l1 = pre.l1_distance(post);
        ContinuityDimension {
            name: "behavioral_action_distribution".into(),
            value: (1.0 - l1 / 2.0).clamp(0.0, 1.0),
            computation: format!(
                "1 - L1(fingerprint_pre, fingerprint_post)/2; L1={l1:.4}; pre_n={} post_n={}",
                pre.total_actions, post.total_actions
            ),
        }
    }

    /// Memory retention across a migration boundary: fraction of the
    /// pre-migration memory that the subject still carries afterwards.
    /// Measured on the (action name, quantized reward) content of each
    /// remembered decision — ticks are NOT compared, because the memory
    /// buffer legitimately keeps rolling (FIFO) after the swap; what must
    /// survive is what was remembered, not when it was indexed.
    ///
    /// Computation: |pre ∩ post| / max(1, |pre|) over multisets of
    /// "action:reward_bucket" strings, where reward_bucket quantizes the
    /// reward signal into 0.5-wide bins ([−∞..0) = "loss", else floor to
    /// bin). Reward is included so that remembered outcomes, not just
    /// actions, are matched; a purely action-based match would credit a
    /// subject that remembers acting but not what came of it.
    pub fn memory_retention(
        pre: &[crate::cognition::MemoryEntry],
        post: &[crate::cognition::MemoryEntry],
    ) -> ContinuityDimension {
        let key = |m: &crate::cognition::MemoryEntry| {
            let bucket = if m.reward_signal < 0.0 {
                "loss".to_string()
            } else {
                format!("gain{}", (m.reward_signal / 0.5).floor() as i32)
            };
            format!("{}:{bucket}", m.action.name)
        };
        // Multiset intersection: count keys on both sides, sum the minima.
        let mut pre_counts: BTreeMap<String, u64> = BTreeMap::new();
        for m in pre {
            *pre_counts.entry(key(m)).or_insert(0) += 1;
        }
        let mut post_counts: BTreeMap<String, u64> = BTreeMap::new();
        for m in post {
            *post_counts.entry(key(m)).or_insert(0) += 1;
        }
        let mut matched = 0u64;
        for (k, pre_n) in &pre_counts {
            let post_n = post_counts.get(k).copied().unwrap_or(0);
            matched += (*pre_n).min(post_n);
        }
        let pre_total = pre.len() as u64;
        let value = matched as f32 / pre_total.max(1) as f32;
        ContinuityDimension {
            name: "memory_retention".into(),
            value: value.clamp(0.0, 1.0),
            computation: format!(
                "multiset_match(action:reward_bin)/|memory_pre|; matched={matched} \
                 pre_n={pre_total} post_n={}",
                post.len()
            ),
        }
    }

    /// Mean of dimension values — DOCUMENTED AGGREGATE: arbitrary, treats all
    /// dimensions as equally weighted. Components are always preserved; any
    /// scientific use should prefer the vector over this scalar.
    pub fn documented_aggregate(&self) -> f32 {
        if self.dimensions.is_empty() {
            return 0.0;
        }
        self.dimensions.iter().map(|d| d.value).sum::<f32>() / self.dimensions.len() as f32
    }

    /// Boundary-anchored memory usage: of the decisions the subject REMEMBERED
    /// at the migration boundary (the `boundary` snapshot), how many did it
    /// REPEAT in the post-migration window (`post_observations`, given as
    /// action names observed after the swap)?
    ///
    /// This is the "memory USE" dimension the FIFO roll-off problem hid: even
    /// when the buffer has fully rolled over by the end of the run, the
    /// subject can still act on what it remembered AT the boundary. Value in
    /// [0,1]; 0 = none of the boundary memories were re-enacted, 1 = every
    /// boundary memory's action was re-enacted at least once afterwards.
    ///
    /// Computation: |{boundary actions} ∩ {post actions}| /
    /// max(1, |{boundary actions}|) over action-name sets.
    pub fn memory_usage(
        boundary: &[crate::cognition::MemoryEntry],
        post_action_names: &[String],
    ) -> ContinuityDimension {
        let boundary_actions: std::collections::BTreeSet<String> =
            boundary.iter().map(|m| m.action.name.clone()).collect();
        let post_actions: std::collections::BTreeSet<String> =
            post_action_names.iter().cloned().collect();
        let reused = boundary_actions.intersection(&post_actions).count() as f32;
        let boundary_total = boundary_actions.len() as f32;
        let value = (reused / boundary_total.max(1.0)).clamp(0.0, 1.0);
        ContinuityDimension {
            name: "memory_usage".into(),
            value,
            computation: format!(
                "set_match(boundary_actions ∩ post_actions)/|boundary_actions|; \
                 reused={reused} boundary_n={boundary_total} post_n={}",
                post_actions.len()
            ),
        }
    }
}

/// Collect action names from a decision log slice (already filtered).
pub struct ActionCollector {
    actions: Vec<String>,
}

impl ActionCollector {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn record(&mut self, action_name: Option<&str>) {
        self.actions
            .push(action_name.unwrap_or("<none>").to_string());
    }

    /// Distinct action names recorded (excluding the `<none>` placeholder) —
    /// the set used by memory_usage.
    pub fn distinct_action_names(&self) -> Vec<String> {
        let names: std::collections::BTreeSet<String> = self
            .actions
            .iter()
            .filter(|a| a.as_str() != "<none>")
            .cloned()
            .collect();
        names.into_iter().collect()
    }

    pub fn fingerprint(&self) -> BehaviorFingerprint {
        BehaviorFingerprint::from_distribution(&action_distribution(&self.actions))
    }
}

impl Default for ActionCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_id_deterministic() {
        let m1 = MigrationEvent::new(
            "s1",
            "cognition_backend",
            "mock-echo-v0",
            "mock-random-v0",
            100,
            "inc-1".into(),
            "inc-2".into(),
        );
        let m2 = MigrationEvent::new(
            "s1",
            "cognition_backend",
            "mock-echo-v0",
            "mock-random-v0",
            100,
            "inc-1".into(),
            "inc-2".into(),
        );
        assert_eq!(
            m1.migration_id, m2.migration_id,
            "same content => same id (replay-stable)"
        );
        let m3 = MigrationEvent::new(
            "s1",
            "cognition_backend",
            "mock-echo-v0",
            "mock-random-v0",
            101,
            "inc-1".into(),
            "inc-2".into(),
        );
        assert_ne!(m1.migration_id, m3.migration_id);
    }

    #[test]
    fn fingerprint_l1_identical_is_zero() {
        let a = action_distribution(&["WAIT", "WAIT", "ABSORB"]);
        let fa = BehaviorFingerprint::from_distribution(&a);
        let fb = BehaviorFingerprint::from_distribution(&a);
        assert!((fa.l1_distance(&fb)).abs() < 1e-6);
    }

    #[test]
    fn fingerprint_l1_disjoint_is_two() {
        let a = action_distribution(&["WAIT", "WAIT"]);
        let b = action_distribution(&["ABSORB", "ABSORB"]);
        let fa = BehaviorFingerprint::from_distribution(&a);
        let fb = BehaviorFingerprint::from_distribution(&b);
        assert!((fa.l1_distance(&fb) - 2.0).abs() < 1e-6);
        // Behavioral continuity of disjoint behavior = 0.0
        let d = ContinuityVector::behavioral(&fa, &fb);
        assert!((d.value - 0.0).abs() < 1e-6);
    }

    #[test]
    fn behavioral_continuity_identical_is_one() {
        let a = action_distribution(&["WAIT", "ABSORB", "ABSORB"]);
        let fa = BehaviorFingerprint::from_distribution(&a);
        let d = ContinuityVector::behavioral(&fa, &fa);
        assert!((d.value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn aggregate_documents_arbitrariness() {
        let v = ContinuityVector {
            subject_id: "s".into(),
            migration_id: None,
            dimensions: vec![
                ContinuityDimension {
                    name: "a".into(),
                    value: 1.0,
                    computation: "test".into(),
                },
                ContinuityDimension {
                    name: "b".into(),
                    value: 0.0,
                    computation: "test".into(),
                },
            ],
        };
        assert!((v.documented_aggregate() - 0.5).abs() < 1e-6);
        // Components preserved regardless of aggregate.
        assert_eq!(v.dimensions.len(), 2);
    }

    #[test]
    fn fnv1a_known_vector() {
        // FNV-1a 64 of "a" = 0xaf63dc4c8601ec8c (standard test vector)
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    }

    fn entry(action: &str, reward: f32) -> crate::cognition::MemoryEntry {
        crate::cognition::MemoryEntry {
            tick: 0,
            action: crate::wabix::Action {
                name: action.into(),
                params: vec![],
            },
            reward_signal: reward,
            pre_tick_energy: 10.0,
        }
    }

    #[test]
    fn memory_retention_full_when_buffer_unchanged() {
        let mem = vec![entry("ABSORB", 1.2), entry("WAIT", -0.3)];
        let d = ContinuityVector::memory_retention(&mem, &mem);
        assert!((d.value - 1.0).abs() < 1e-6);
        assert_eq!(d.name, "memory_retention");
    }

    #[test]
    fn memory_retention_partial_when_fifo_rolls() {
        // The subject keeps remembering ABSORB+gain but the WAIT+loss entry
        // aged out of the FIFO buffer.
        let pre = vec![entry("ABSORB", 1.2), entry("WAIT", -0.3)];
        let post = vec![entry("ABSORB", 1.2), entry("ABSORB", 0.4)];
        let d = ContinuityVector::memory_retention(&pre, &post);
        assert!((d.value - 0.5).abs() < 1e-6, "got {}", d.value);
    }

    #[test]
    fn memory_retention_reward_bins_matter() {
        // Same action but a different remembered OUTCOME does not match.
        let pre = vec![entry("ABSORB", 1.2)];
        let post = vec![entry("ABSORB", -0.4)];
        let d = ContinuityVector::memory_retention(&pre, &post);
        assert!(d.value.abs() < 1e-6, "got {}", d.value);
    }

    #[test]
    fn memory_retention_empty_pre_is_neutral() {
        // Nothing remembered before the boundary: the dimension cannot be
        // scored against anything; computation documents that, value = 0.
        let d = ContinuityVector::memory_retention(&[], &[entry("WAIT", 0.0)]);
        assert!(d.value.abs() < 1e-6);
        assert!(d.computation.contains("pre_n=0"));
    }

    #[test]
    fn memory_usage_full_when_boundary_actions_reenacted() {
        // The subject remembered ABSORB and SIGNAL at the boundary, and both
        // were re-enacted in the post window — full usage.
        let boundary = vec![entry("ABSORB", 1.2), entry("SIGNAL", 0.6)];
        let post = vec![
            "ABSORB".to_string(),
            "SIGNAL".to_string(),
            "WAIT".to_string(),
        ];
        let d = ContinuityVector::memory_usage(&boundary, &post);
        assert!((d.value - 1.0).abs() < 1e-6, "got {}", d.value);
        assert_eq!(d.name, "memory_usage");
    }

    #[test]
    fn memory_usage_partial_when_some_reenacted() {
        let boundary = vec![entry("ABSORB", 1.2), entry("SIGNAL", 0.6)];
        let post = vec!["ABSORB".to_string(), "WAIT".to_string()];
        let d = ContinuityVector::memory_usage(&boundary, &post);
        assert!((d.value - 0.5).abs() < 1e-6, "got {}", d.value);
    }

    #[test]
    fn memory_usage_zero_when_boundary_actions_dropped() {
        let boundary = vec![entry("SIGNAL", 0.6)];
        let post = vec!["ABSORB".to_string()];
        let d = ContinuityVector::memory_usage(&boundary, &post);
        assert!(d.value.abs() < 1e-6, "got {}", d.value);
    }

    #[test]
    fn memory_usage_empty_boundary_is_neutral() {
        let d = ContinuityVector::memory_usage(&[], &["ABSORB".to_string()]);
        assert!(d.value.abs() < 1e-6);
        assert!(d.computation.contains("boundary_n=0"));
    }
}
