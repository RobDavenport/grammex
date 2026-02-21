//! Edge data trait for user-defined edge types.

/// Trait for user-defined graph edge data.
/// Implementations define what data edges carry and how they are matched in patterns.
pub trait EdgeData: Clone + Eq {
    /// The "kind" used for pattern matching in rules.
    type Kind: Copy + Eq + core::fmt::Debug;

    /// Get this edge's kind for pattern matching.
    fn kind(&self) -> Self::Kind;
}
