//! Resource accounting: compute is never free.
//!
//! The ledger records resource consumption as auditable events with explicit
//! kinds. Token counts and wall-clock seconds are engineering proxies for
//! cost, not biological claims. The ledger is append-only and bounded via
//! snapshot/compaction, keeping memory O(bounded) for long runs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Kinds of resource we account for. Extensible by experiment config later.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// Environmental energy in abstract units (world-defined).
    Energy,
    /// Number of cognition events (inference opportunities consumed).
    CognitionEvents,
    /// Weighted token count consumed by a cognition backend (proxy).
    Tokens,
    /// CPU time in microseconds (proxy for compute).
    CpuMicros,
    /// Bytes written to persistent storage.
    StorageBytes,
}

impl ResourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceKind::Energy => "energy",
            ResourceKind::CognitionEvents => "cognition_events",
            ResourceKind::Tokens => "tokens",
            ResourceKind::CpuMicros => "cpu_micros",
            ResourceKind::StorageBytes => "storage_bytes",
        }
    }
}

/// One auditable consumption entry. Quantities are integers in natural units
/// (uJ, tokens, bytes, us) to keep sums exact and replayable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEntry {
    /// World tick (or logical time) at which the consumption happened.
    pub tick: u64,
    pub kind: ResourceKind,
    pub amount: u64,
    /// Who consumed it; empty string = the runtime itself.
    pub consumer: String,
}

/// Append-only resource ledger with bounded in-memory retention.
///
/// `max_entries` bounds RAM; older entries must be flushed to the event log
/// before compaction (the caller owns that policy). Totals are kept exactly
/// across compaction.
#[derive(Debug)]
pub struct ResourceLedger {
    entries: std::collections::VecDeque<ResourceEntry>,
    totals: BTreeMap<ResourceKind, u128>,
    max_entries: usize,
    dropped_entries: usize,
}

impl ResourceLedger {
    pub fn new(max_entries: usize) -> Self {
        assert!(max_entries > 0, "ledger must retain at least one entry");
        Self {
            entries: std::collections::VecDeque::new(),
            totals: BTreeMap::new(),
            max_entries,
            dropped_entries: 0,
        }
    }

    pub fn record(&mut self, entry: ResourceEntry) {
        *self.totals.entry(entry.kind.clone()).or_insert(0) += entry.amount as u128;
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
            self.dropped_entries += 1;
        }
    }

    /// Recent entries still retained in memory (bounded window).
    pub fn recent(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.entries.iter()
    }

    /// Exact cumulative totals, valid across compaction.
    pub fn totals(&self) -> &BTreeMap<ResourceKind, u128> {
        &self.totals
    }

    pub fn total(&self, kind: &ResourceKind) -> u128 {
        self.totals.get(kind).copied().unwrap_or(0)
    }

    pub fn dropped_entries(&self) -> usize {
        self.dropped_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(tick: u64, kind: ResourceKind, amount: u64) -> ResourceEntry {
        ResourceEntry {
            tick,
            kind,
            amount,
            consumer: "test".into(),
        }
    }

    #[test]
    fn totals_survive_compaction() {
        let mut ledger = ResourceLedger::new(4);
        for t in 0..10 {
            ledger.record(entry(t, ResourceKind::Energy, 5));
        }
        assert_eq!(ledger.total(&ResourceKind::Energy), 50);
        assert_eq!(ledger.dropped_entries(), 6);
        assert_eq!(ledger.recent().count(), 4);
        // Oldest retained entry is tick 6.
        assert_eq!(ledger.recent().next().unwrap().tick, 6);
    }

    #[test]
    fn kinds_are_independent() {
        let mut ledger = ResourceLedger::new(10);
        ledger.record(entry(0, ResourceKind::Tokens, 7));
        ledger.record(entry(1, ResourceKind::CognitionEvents, 1));
        assert_eq!(ledger.total(&ResourceKind::Tokens), 7);
        assert_eq!(ledger.total(&ResourceKind::CognitionEvents), 1);
        assert_eq!(ledger.total(&ResourceKind::Energy), 0);
    }
}
