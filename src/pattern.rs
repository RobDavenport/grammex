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
    graph: &Graph<N, E>,
    pattern: &Pattern<N::Kind, E::Kind>,
) -> Vec<Match> {
    let mut matches = Vec::new();
    let mut mapping = Vec::new();
    vf2_search(graph, pattern, &mut mapping, &mut matches);
    matches
}

fn vf2_search<N: NodeData, E: EdgeData>(
    graph: &Graph<N, E>,
    pattern: &Pattern<N::Kind, E::Kind>,
    mapping: &mut Vec<NodeId>,
    matches: &mut Vec<Match>,
) {
    if mapping.len() == pattern.nodes.len() {
        if all_edges_match(graph, pattern, mapping) {
            matches.push(Match { node_map: mapping.clone() });
        }
        return;
    }

    let p_node = &pattern.nodes[mapping.len()];

    for g_node in graph.node_ids() {
        if mapping.contains(&g_node) {
            continue;
        }
        if graph.node(g_node).unwrap().kind() != p_node.kind {
            continue;
        }
        if !consistent(graph, pattern, mapping, p_node, g_node) {
            continue;
        }

        mapping.push(g_node);
        vf2_search(graph, pattern, mapping, matches);
        mapping.pop();
    }
}

fn consistent<N: NodeData, E: EdgeData>(
    graph: &Graph<N, E>,
    pattern: &Pattern<N::Kind, E::Kind>,
    mapping: &[NodeId],
    p_node: &PatternNode<N::Kind>,
    g_node: NodeId,
) -> bool {
    for (p_idx, g_mapped) in mapping.iter().enumerate() {
        let p_mapped_id = LocalId(p_idx);

        // Check forward edges: p_mapped -> p_node in pattern => g_mapped -> g_node in graph
        for edge in &pattern.edges {
            if edge.source == p_mapped_id && edge.target == p_node.id {
                if !has_edge_with_kind(graph, *g_mapped, g_node, edge.kind) {
                    return false;
                }
            }
            // Check backward edges: p_node -> p_mapped in pattern => g_node -> g_mapped in graph
            if edge.source == p_node.id && edge.target == p_mapped_id {
                if !has_edge_with_kind(graph, g_node, *g_mapped, edge.kind) {
                    return false;
                }
            }
        }
    }
    true
}

fn has_edge_with_kind<N: NodeData, E: EdgeData>(
    graph: &Graph<N, E>,
    source: NodeId,
    target: NodeId,
    kind: E::Kind,
) -> bool {
    for &eid in graph.outgoing(source) {
        if let Some((_, t)) = graph.edge_endpoints(eid) {
            if t == target {
                if let Some(edata) = graph.edge(eid) {
                    if edata.kind() == kind {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn all_edges_match<N: NodeData, E: EdgeData>(
    graph: &Graph<N, E>,
    pattern: &Pattern<N::Kind, E::Kind>,
    mapping: &[NodeId],
) -> bool {
    for edge in &pattern.edges {
        let g_source = mapping[edge.source.0];
        let g_target = mapping[edge.target.0];
        if !has_edge_with_kind(graph, g_source, g_target, edge.kind) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::graph::Graph;

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TNode(u8);
    impl crate::node::NodeData for TNode {
        type Kind = u8;
        fn kind(&self) -> u8 { self.0 }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TEdge(u8);
    impl crate::edge::EdgeData for TEdge {
        type Kind = u8;
        fn kind(&self) -> u8 { self.0 }
    }

    #[test]
    fn single_node_match() {
        // 5-node graph with kinds [1, 2, 1, 1, 3]
        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1));
        g.add_node(TNode(2));
        g.add_node(TNode(1));
        g.add_node(TNode(1));
        g.add_node(TNode(3));

        // Pattern: single node of kind 1
        let mut p = Pattern::new();
        p.add_node(1u8);

        let matches = find_matches(&g, &p);
        assert_eq!(matches.len(), 3, "Should find 3 nodes of kind 1");
    }

    #[test]
    fn two_connected_nodes_match() {
        // Graph: 0(1) -> 1(2), 2(1) -> 3(2), 4(1) (no outgoing)
        let mut g = Graph::<TNode, TEdge>::new();
        let n0 = g.add_node(TNode(1));
        let n1 = g.add_node(TNode(2));
        let n2 = g.add_node(TNode(1));
        let n3 = g.add_node(TNode(2));
        g.add_node(TNode(1)); // n4, no edges

        g.add_edge(n0, n1, TEdge(10));
        g.add_edge(n2, n3, TEdge(10));

        // Pattern: node(1) -> node(2) with edge kind 10
        let mut p = Pattern::new();
        let pa = p.add_node(1u8);
        let pb = p.add_node(2u8);
        p.add_edge(pa, pb, 10u8);

        let matches = find_matches(&g, &p);
        assert_eq!(matches.len(), 2, "Should find 2 matching edges");
    }

    #[test]
    fn triangle_match() {
        // Graph: triangle 0->1->2->0, all kind 1, edge kind 5
        let mut g = Graph::<TNode, TEdge>::new();
        let n0 = g.add_node(TNode(1));
        let n1 = g.add_node(TNode(1));
        let n2 = g.add_node(TNode(1));
        g.add_edge(n0, n1, TEdge(5));
        g.add_edge(n1, n2, TEdge(5));
        g.add_edge(n2, n0, TEdge(5));

        // Pattern: triangle a->b->c->a
        let mut p = Pattern::new();
        let pa = p.add_node(1u8);
        let pb = p.add_node(1u8);
        let pc = p.add_node(1u8);
        p.add_edge(pa, pb, 5u8);
        p.add_edge(pb, pc, 5u8);
        p.add_edge(pc, pa, 5u8);

        let matches = find_matches(&g, &p);
        // 3 rotations of the triangle
        assert_eq!(matches.len(), 3, "Should find 3 rotations of the triangle");
    }

    #[test]
    fn no_match_returns_empty() {
        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1));
        g.add_node(TNode(2));

        // Pattern: node of kind 99 (doesn't exist)
        let mut p = Pattern::new();
        p.add_node(99u8);

        let matches = find_matches(&g, &p);
        assert!(matches.is_empty(), "Should find no matches");
    }
}
