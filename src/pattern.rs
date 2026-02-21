//! Pattern matching for subgraph isomorphism.

use alloc::vec::Vec;
use crate::graph::{Graph, NodeId};
use crate::node::NodeData;
use crate::edge::EdgeData;

/// A local ID within a pattern (not a global graph NodeId).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

/// A node in a pattern.
#[derive(Debug, Clone)]
pub struct PatternNode<NK> {
    pub id: LocalId,
    pub kind: NK,
}

/// An edge in a pattern.
#[derive(Debug, Clone)]
pub struct PatternEdge<EK> {
    pub source: LocalId,
    pub target: LocalId,
    pub kind: EK,
}

/// A subgraph pattern to match against the host graph (left-hand side of a rule).
#[derive(Debug, Clone)]
pub struct Pattern<NK, EK> {
    pub nodes: Vec<PatternNode<NK>>,
    pub edges: Vec<PatternEdge<EK>>,
}

impl<NK: Copy + Eq, EK: Copy + Eq> Pattern<NK, EK> {
    /// Create a new empty pattern.
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new() }
    }

    /// Add a node to the pattern.
    pub fn add_node(&mut self, kind: NK) -> LocalId {
        let id = LocalId(self.nodes.len());
        self.nodes.push(PatternNode { id, kind });
        id
    }

    /// Add an edge to the pattern.
    pub fn add_edge(&mut self, source: LocalId, target: LocalId, kind: EK) {
        self.edges.push(PatternEdge { source, target, kind });
    }
}

/// A match: mapping from pattern LocalIds to graph NodeIds.
#[derive(Debug, Clone)]
pub struct Match {
    /// node_map[local_id.0] = graph NodeId
    pub node_map: Vec<NodeId>,
}

/// Find all matches of a pattern in a graph using VF2-lite subgraph isomorphism.
pub fn find_matches<N: NodeData, E: EdgeData>(
    _graph: &Graph<N, E>,
    _pattern: &Pattern<N::Kind, E::Kind>,
) -> Vec<Match> {
    todo!()
}
