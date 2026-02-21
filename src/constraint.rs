//! Structural constraints checked during rewriting.

use crate::graph::{Graph, NodeId};
use crate::node::NodeData;
use crate::edge::EdgeData;

/// A structural constraint checked after each rule application.
/// If `check` returns false, the rule application is rolled back.
pub trait StructuralConstraint<N: NodeData, E: EdgeData> {
    /// Check whether the graph satisfies this constraint.
    fn check(&self, graph: &Graph<N, E>) -> bool;
}

/// Every "lock" node is reachable only via paths that pass through
/// the corresponding "key" node first (from a designated start node).
pub struct LockKeyConstraint<K: Copy + Eq> {
    pub start: NodeId,
    _phantom: core::marker::PhantomData<K>,
}

impl<K: Copy + Eq> LockKeyConstraint<K> {
    pub fn new(start: NodeId) -> Self {
        Self { start, _phantom: core::marker::PhantomData }
    }
}

// Note: implementing StructuralConstraint requires knowing how to extract
// lock/key pairs from nodes. This will need a trait bound or closure.
// For now, this is a marker struct -- implementation TBD.

/// All nodes of specified kinds are reachable from a start node.
pub struct ReachabilityConstraint<NK: Copy + Eq> {
    pub start: NodeId,
    pub required_kinds: alloc::vec::Vec<NK>,
}

impl<NK: Copy + Eq> ReachabilityConstraint<NK> {
    pub fn new(start: NodeId, required_kinds: alloc::vec::Vec<NK>) -> Self {
        Self { start, required_kinds }
    }
}

/// The graph contains exactly N independent cycles.
pub struct CycleConstraint {
    pub expected_cycles: usize,
}

impl CycleConstraint {
    pub fn new(expected_cycles: usize) -> Self {
        Self { expected_cycles }
    }
}

/// The graph must be acyclic (a DAG or tree).
pub struct AcyclicConstraint;
