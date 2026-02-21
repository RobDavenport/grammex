//! Observer trait for monitoring rewriting progress.

use crate::graph::{Graph, NodeId};
use crate::node::NodeData;
use crate::edge::EdgeData;

/// Observer for monitoring graph rewriting progress.
pub trait RewriteObserver<N: NodeData, E: EdgeData> {
    /// Called when a rule is matched (before application).
    fn on_match(&mut self, _rule_index: usize, _matched_nodes: &[NodeId]) {}
    /// Called after a rule is applied.
    fn on_rule_applied(&mut self, _rule_index: usize, _rule_name: &str) {}
    /// Called when a constraint check fails (application rolled back).
    fn on_constraint_violated(&mut self, _rule_index: usize) {}
    /// Called after each step completes.
    fn on_step_complete(&mut self, _step: usize, _graph: &Graph<N, E>) {}
    /// Called when no rules match (rewriting complete).
    fn on_no_match(&mut self) {}
}

/// No-op observer.
pub struct NoOpRewriteObserver;

impl<N: NodeData, E: EdgeData> RewriteObserver<N, E> for NoOpRewriteObserver {}
