//! Graph rewriting rules.

use alloc::vec::Vec;
use crate::pattern::{Pattern, LocalId};
use crate::node::NodeData;
use crate::edge::EdgeData;

/// What to do with a matched node during replacement.
#[derive(Debug, Clone)]
pub enum ReplacementAction<N: NodeData> {
    /// Keep the node as-is.
    Keep,
    /// Replace the node's data.
    Replace(N),
    /// Remove the node.
    Remove,
}

/// A node in the replacement subgraph.
#[derive(Debug, Clone)]
pub struct ReplacementNode<N: NodeData> {
    /// If Some, maps to a node from the LHS pattern.
    pub from_lhs: Option<LocalId>,
    /// The replacement action.
    pub action: ReplacementAction<N>,
    /// Data for newly created nodes (when from_lhs is None).
    pub data: Option<N>,
}

/// How to reconnect dangling edges after replacement.
#[derive(Debug, Clone)]
pub struct Reconnection {
    /// The old node (LHS pattern LocalId) whose edges should be redirected.
    pub from: LocalId,
    /// The new node (RHS LocalId) to redirect edges to.
    pub to: LocalId,
}

/// The replacement specification (right-hand side of a rule).
#[derive(Debug, Clone)]
pub struct Replacement<N: NodeData, E: EdgeData> {
    /// Nodes in the replacement.
    pub nodes: Vec<ReplacementNode<N>>,
    /// Edges in the replacement.
    pub edges: Vec<(LocalId, LocalId, E)>,
    /// How to reconnect dangling edges.
    pub reconnections: Vec<Reconnection>,
}

/// A graph rewriting rule: pattern -> replacement.
pub struct Rule<N: NodeData, E: EdgeData> {
    /// Pattern to match (left-hand side).
    pub lhs: Pattern<N::Kind, E::Kind>,
    /// Replacement specification (right-hand side).
    pub rhs: Replacement<N, E>,
    /// Priority/weight for selection when multiple rules match.
    pub weight: u32,
    /// Human-readable name for debugging/observation.
    pub name: &'static str,
}
