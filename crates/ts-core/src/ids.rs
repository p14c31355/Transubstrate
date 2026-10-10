//! Strongly-typed identity for Transubstrate.
//!
//! Administrative identity (IDs) is deliberately separate from *measured*
//! continuity. A UUID proves nothing scientifically; it only names things
//! unambiguously. See docs/continuity.md (M1) for the measurement story.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Self {
                // Time-ordered UUIDv7 rendered as its canonical string; the
                // textual form keeps event logs diff-friendly and stable.
                Self(uuid::Uuid::now_v7().to_string())
            }

            pub fn from_static(s: &str) -> Self {
                Self(s.to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(
    /// A persistent artificial subject. NOT the model that powers it.
    SubjectId
);
typed_id!(
    /// One lifetime of a subject on one substrate; changes at migration boundaries.
    IncarnationId
);
typed_id!(
    /// Which cognition backend instance produced decisions (e.g. "mock-rule-v0").
    CognitionBackendId
);
typed_id!(
    /// Which machine/runtime executed the subject.
    MachineId
);
typed_id!(
    /// Which body/sensor-actuator set the subject inhabited.
    BodyId
);
typed_id!(
    /// Which world the subject inhabited.
    WorldId
);
typed_id!(
    /// A lineage (reproductive family), shared by parent and children.
    LineageId
);
typed_id!(
    /// One recorded migration event.
    MigrationEventId
);
typed_id!(
    /// One experiment run.
    ExperimentId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_across_types() {
        let s = SubjectId::generate();
        let w = WorldId::generate();
        assert_ne!(s.as_str(), w.as_str());
    }

    #[test]
    fn static_ids_round_trip() {
        let s = SubjectId::from_static("subject-fixed");
        assert_eq!(s.as_str(), "subject-fixed");
        assert_eq!(s.to_string(), "subject-fixed");
    }
}
