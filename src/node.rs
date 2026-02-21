//! Node data trait for user-defined node types.

/// Trait for user-defined graph node data.
/// Implementations define what data nodes carry and how they are matched in patterns.
pub trait NodeData: Clone + Eq {
    /// The "kind" used for pattern matching in rules.
    /// Must be cheap to copy and compare.
    type Kind: Copy + Eq + core::fmt::Debug;

    /// Get this node's kind for pattern matching.
    fn kind(&self) -> Self::Kind;
}
