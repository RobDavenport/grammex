//! Arena-based directed graph with stable node/edge identifiers.

use alloc::vec::Vec;
use crate::node::NodeData;
use crate::edge::EdgeData;

/// Stable identifier for a node in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Stable identifier for an edge in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub usize);

/// A directed graph with stable node and edge identifiers.
/// Uses arena allocation with a free list for O(1) add/remove.
#[derive(Clone)]
pub struct Graph<N: NodeData, E: EdgeData> {
    nodes: Vec<Option<NodeSlot<N>>>,
    edges: Vec<Option<EdgeSlot<E>>>,
    node_count: usize,
    edge_count: usize,
}

#[derive(Clone)]
struct NodeSlot<N: NodeData> {
    data: N,
    outgoing: Vec<EdgeId>,
    incoming: Vec<EdgeId>,
}

#[derive(Clone)]
struct EdgeSlot<E: EdgeData> {
    data: E,
    source: NodeId,
    target: NodeId,
}

impl<N: NodeData, E: EdgeData> Graph<N, E> {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_count: 0,
            edge_count: 0,
        }
    }

    /// Add a node to the graph. Returns its stable ID.
    pub fn add_node(&mut self, data: N) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Some(NodeSlot {
            data,
            outgoing: Vec::new(),
            incoming: Vec::new(),
        }));
        self.node_count += 1;
        id
    }

    /// Add a directed edge from source to target. Returns its stable ID.
    pub fn add_edge(&mut self, source: NodeId, target: NodeId, data: E) -> EdgeId {
        let id = EdgeId(self.edges.len());
        self.edges.push(Some(EdgeSlot { data, source, target }));
        if let Some(node) = &mut self.nodes[source.0] {
            node.outgoing.push(id);
        }
        if let Some(node) = &mut self.nodes[target.0] {
            node.incoming.push(id);
        }
        self.edge_count += 1;
        id
    }

    /// Remove a node and all its connected edges.
    pub fn remove_node(&mut self, id: NodeId) {
        if let Some(node) = self.nodes[id.0].take() {
            self.node_count -= 1;
            // Remove connected edges
            let edges_to_remove: Vec<_> = node.outgoing.iter()
                .chain(node.incoming.iter())
                .copied()
                .collect();
            for eid in edges_to_remove {
                self.remove_edge(eid);
            }
        }
    }

    /// Remove an edge.
    pub fn remove_edge(&mut self, id: EdgeId) {
        if let Some(edge) = self.edges[id.0].take() {
            self.edge_count -= 1;
            // Remove from source's outgoing
            if let Some(src) = &mut self.nodes[edge.source.0] {
                src.outgoing.retain(|e| *e != id);
            }
            // Remove from target's incoming
            if let Some(tgt) = &mut self.nodes[edge.target.0] {
                tgt.incoming.retain(|e| *e != id);
            }
        }
    }

    /// Get node data by ID.
    pub fn node(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(id.0)?.as_ref().map(|s| &s.data)
    }

    /// Get mutable node data by ID.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut N> {
        self.nodes.get_mut(id.0)?.as_mut().map(|s| &mut s.data)
    }

    /// Get edge data by ID.
    pub fn edge(&self, id: EdgeId) -> Option<&E> {
        self.edges.get(id.0)?.as_ref().map(|s| &s.data)
    }

    /// Get the source and target of an edge.
    pub fn edge_endpoints(&self, id: EdgeId) -> Option<(NodeId, NodeId)> {
        self.edges.get(id.0)?.as_ref().map(|s| (s.source, s.target))
    }

    /// Get outgoing edge IDs for a node.
    pub fn outgoing(&self, id: NodeId) -> &[EdgeId] {
        self.nodes.get(id.0)
            .and_then(|s| s.as_ref())
            .map(|s| s.outgoing.as_slice())
            .unwrap_or(&[])
    }

    /// Get incoming edge IDs for a node.
    pub fn incoming(&self, id: NodeId) -> &[EdgeId] {
        self.nodes.get(id.0)
            .and_then(|s| s.as_ref())
            .map(|s| s.incoming.as_slice())
            .unwrap_or(&[])
    }

    /// Iterate over all live node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|_| NodeId(i)))
    }

    /// Iterate over all live edge IDs.
    pub fn edge_ids(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.iter().enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|_| EdgeId(i)))
    }

    /// Number of live nodes.
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Number of live edges.
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
}

impl<N: NodeData, E: EdgeData> Default for Graph<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    // Simple test node/edge types
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestNode(u8);
    impl NodeData for TestNode {
        type Kind = u8;
        fn kind(&self) -> u8 { self.0 }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestEdge(u8);
    impl EdgeData for TestEdge {
        type Kind = u8;
        fn kind(&self) -> u8 { self.0 }
    }

    #[test]
    fn add_and_query_nodes() {
        let mut g = Graph::<TestNode, TestEdge>::new();
        let a = g.add_node(TestNode(1));
        let b = g.add_node(TestNode(2));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.node(a).unwrap().0, 1);
        assert_eq!(g.node(b).unwrap().0, 2);
    }

    #[test]
    fn add_and_query_edges() {
        let mut g = Graph::<TestNode, TestEdge>::new();
        let a = g.add_node(TestNode(1));
        let b = g.add_node(TestNode(2));
        let e = g.add_edge(a, b, TestEdge(10));
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edge(e).unwrap().0, 10);
        assert_eq!(g.edge_endpoints(e), Some((a, b)));
        assert_eq!(g.outgoing(a).len(), 1);
        assert_eq!(g.incoming(b).len(), 1);
    }

    #[test]
    fn remove_node_removes_edges() {
        let mut g = Graph::<TestNode, TestEdge>::new();
        let a = g.add_node(TestNode(1));
        let b = g.add_node(TestNode(2));
        g.add_edge(a, b, TestEdge(10));
        g.remove_node(a);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_count(), 0);
        assert!(g.node(a).is_none());
    }

    #[test]
    fn node_ids_iteration() {
        let mut g = Graph::<TestNode, TestEdge>::new();
        g.add_node(TestNode(1));
        let b = g.add_node(TestNode(2));
        g.add_node(TestNode(3));
        g.remove_node(b);
        let ids: std::vec::Vec<_> = g.node_ids().collect();
        assert_eq!(ids.len(), 2);
    }
}
