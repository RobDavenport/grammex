//! Structural constraints checked during rewriting.

use alloc::vec::Vec;
use alloc::collections::VecDeque;
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
///
/// Uses kind-based pairs: each `(key_kind, lock_kind)` defines a lock-key relationship.
/// BFS from start, collect key kinds as encountered, verify each lock's key was already collected.
pub struct LockKeyConstraint<NK: Copy + Eq> {
    pub start: NodeId,
    pub pairs: Vec<(NK, NK)>, // (key_kind, lock_kind)
}

impl<NK: Copy + Eq> LockKeyConstraint<NK> {
    pub fn new(start: NodeId, pairs: Vec<(NK, NK)>) -> Self {
        Self { start, pairs }
    }

    fn is_key_kind(&self, kind: NK) -> bool {
        self.pairs.iter().any(|(k, _)| *k == kind)
    }

    fn is_lock_kind(&self, kind: NK) -> bool {
        self.pairs.iter().any(|(_, l)| *l == kind)
    }

    fn required_key_for_lock(&self, lock_kind: NK) -> Option<NK> {
        self.pairs.iter().find(|(_, l)| *l == lock_kind).map(|(k, _)| *k)
    }
}

impl<N: NodeData, E: EdgeData> StructuralConstraint<N, E> for LockKeyConstraint<N::Kind> {
    fn check(&self, graph: &Graph<N, E>) -> bool {
        // BFS from start, collecting keys. At each lock, verify key was already collected.
        let mut visited = Vec::new();
        let mut collected_keys: Vec<N::Kind> = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(self.start);
        visited.push(self.start);

        while let Some(node_id) = queue.pop_front() {
            let node_kind = match graph.node(node_id) {
                Some(n) => n.kind(),
                None => continue,
            };

            // If this is a lock, check that its key was already collected
            if self.is_lock_kind(node_kind) {
                if let Some(required_key) = self.required_key_for_lock(node_kind) {
                    if !collected_keys.contains(&required_key) {
                        return false;
                    }
                }
            }

            // If this is a key, collect it
            if self.is_key_kind(node_kind) {
                if !collected_keys.contains(&node_kind) {
                    collected_keys.push(node_kind);
                }
            }

            // Enqueue neighbors (treat as undirected for traversal)
            for &eid in graph.outgoing(node_id) {
                if let Some((_, target)) = graph.edge_endpoints(eid) {
                    if !visited.contains(&target) {
                        visited.push(target);
                        queue.push_back(target);
                    }
                }
            }
            for &eid in graph.incoming(node_id) {
                if let Some((source, _)) = graph.edge_endpoints(eid) {
                    if !visited.contains(&source) {
                        visited.push(source);
                        queue.push_back(source);
                    }
                }
            }
        }

        true
    }
}

/// All nodes of specified kinds are reachable from a start node.
pub struct ReachabilityConstraint<NK: Copy + Eq> {
    pub start: NodeId,
    pub required_kinds: Vec<NK>,
}

impl<NK: Copy + Eq> ReachabilityConstraint<NK> {
    pub fn new(start: NodeId, required_kinds: Vec<NK>) -> Self {
        Self { start, required_kinds }
    }
}

impl<N: NodeData, E: EdgeData> StructuralConstraint<N, E> for ReachabilityConstraint<N::Kind> {
    fn check(&self, graph: &Graph<N, E>) -> bool {
        // BFS from start (treating graph as undirected for reachability)
        let mut visited = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(self.start);
        visited.push(self.start);

        while let Some(node_id) = queue.pop_front() {
            for &eid in graph.outgoing(node_id) {
                if let Some((_, target)) = graph.edge_endpoints(eid) {
                    if !visited.contains(&target) {
                        visited.push(target);
                        queue.push_back(target);
                    }
                }
            }
            for &eid in graph.incoming(node_id) {
                if let Some((source, _)) = graph.edge_endpoints(eid) {
                    if !visited.contains(&source) {
                        visited.push(source);
                        queue.push_back(source);
                    }
                }
            }
        }

        // Check all required kinds are found among visited nodes.
        // If no node of a required kind exists in the graph yet, skip it (vacuous truth).
        for required in &self.required_kinds {
            let exists_in_graph = graph.node_ids().any(|nid| {
                graph.node(nid).map(|n| n.kind() == *required).unwrap_or(false)
            });
            if !exists_in_graph {
                continue;
            }
            let found = visited.iter().any(|&nid| {
                graph.node(nid).map(|n| n.kind() == *required).unwrap_or(false)
            });
            if !found {
                return false;
            }
        }

        true
    }
}

/// The graph contains exactly N independent cycles.
/// Uses formula: cycles = edges - nodes + connected_components
pub struct CycleConstraint {
    pub expected_cycles: usize,
}

impl CycleConstraint {
    pub fn new(expected_cycles: usize) -> Self {
        Self { expected_cycles }
    }
}

impl<N: NodeData, E: EdgeData> StructuralConstraint<N, E> for CycleConstraint {
    fn check(&self, graph: &Graph<N, E>) -> bool {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        // Count connected components (treating as undirected)
        let node_ids: Vec<NodeId> = graph.node_ids().collect();
        let mut visited = Vec::new();
        let mut components = 0usize;

        for &nid in &node_ids {
            if visited.contains(&nid) {
                continue;
            }
            components += 1;
            // BFS
            let mut queue = VecDeque::new();
            queue.push_back(nid);
            visited.push(nid);
            while let Some(current) = queue.pop_front() {
                for &eid in graph.outgoing(current) {
                    if let Some((_, target)) = graph.edge_endpoints(eid) {
                        if !visited.contains(&target) {
                            visited.push(target);
                            queue.push_back(target);
                        }
                    }
                }
                for &eid in graph.incoming(current) {
                    if let Some((source, _)) = graph.edge_endpoints(eid) {
                        if !visited.contains(&source) {
                            visited.push(source);
                            queue.push_back(source);
                        }
                    }
                }
            }
        }

        let cycles = (edge_count + components).saturating_sub(node_count);
        cycles == self.expected_cycles
    }
}

/// The graph must be acyclic (a DAG or tree).
/// Uses DFS with visited + in-stack arrays for directed cycle detection.
pub struct AcyclicConstraint;

impl<N: NodeData, E: EdgeData> StructuralConstraint<N, E> for AcyclicConstraint {
    fn check(&self, graph: &Graph<N, E>) -> bool {
        let node_ids: Vec<NodeId> = graph.node_ids().collect();
        let mut visited = Vec::new();
        let mut in_stack = Vec::new();

        for &nid in &node_ids {
            if visited.contains(&nid) {
                continue;
            }
            if has_cycle_dfs(graph, nid, &mut visited, &mut in_stack) {
                return false;
            }
        }
        true
    }
}

fn has_cycle_dfs<N: NodeData, E: EdgeData>(
    graph: &Graph<N, E>,
    node: NodeId,
    visited: &mut Vec<NodeId>,
    in_stack: &mut Vec<NodeId>,
) -> bool {
    visited.push(node);
    in_stack.push(node);

    for &eid in graph.outgoing(node) {
        if let Some((_, target)) = graph.edge_endpoints(eid) {
            if !visited.contains(&target) {
                if has_cycle_dfs(graph, target, visited, in_stack) {
                    return true;
                }
            } else if in_stack.contains(&target) {
                return true;
            }
        }
    }

    in_stack.retain(|&n| n != node);
    false
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

    // --- Reachability ---

    #[test]
    fn reachability_connected_passes() {
        // A(1) -> B(2) -> C(3), all connected
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(2));
        let c = g.add_node(TNode(3));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(b, c, TEdge(0));

        let constraint = ReachabilityConstraint::new(a, std::vec![1, 2, 3]);
        assert!(constraint.check(&g));
    }

    #[test]
    fn reachability_disconnected_fails() {
        // A(1) -> B(2), C(3) disconnected
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(2));
        g.add_node(TNode(3)); // disconnected
        g.add_edge(a, b, TEdge(0));

        let constraint = ReachabilityConstraint::new(a, std::vec![1, 2, 3]);
        assert!(!constraint.check(&g));
    }

    // --- Cycle ---

    #[test]
    fn cycle_count_tree() {
        // Tree: A -> B, A -> C (0 cycles)
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(1));
        let c = g.add_node(TNode(1));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(a, c, TEdge(0));

        let constraint = CycleConstraint::new(0);
        assert!(constraint.check(&g));
    }

    #[test]
    fn cycle_count_one_loop() {
        // A -> B -> C -> A (1 cycle: 3 edges, 3 nodes, 1 component => 3-3+1=1)
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(1));
        let c = g.add_node(TNode(1));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(b, c, TEdge(0));
        g.add_edge(c, a, TEdge(0));

        let constraint = CycleConstraint::new(1);
        assert!(constraint.check(&g));
    }

    // --- Acyclic ---

    #[test]
    fn acyclic_dag_passes() {
        // DAG: A -> B -> C, A -> C
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(1));
        let c = g.add_node(TNode(1));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(b, c, TEdge(0));
        g.add_edge(a, c, TEdge(0));

        assert!(AcyclicConstraint.check(&g));
    }

    #[test]
    fn acyclic_with_cycle_fails() {
        // Cycle: A -> B -> C -> A
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(1));
        let c = g.add_node(TNode(1));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(b, c, TEdge(0));
        g.add_edge(c, a, TEdge(0));

        assert!(!AcyclicConstraint.check(&g));
    }

    // --- LockKey ---

    #[test]
    fn key_before_lock_passes() {
        // Start(1) -> Key(2) -> Lock(3) -> End(4)
        // Pair: (2=key, 3=lock)
        let mut g = Graph::<TNode, TEdge>::new();
        let start = g.add_node(TNode(1));
        let key = g.add_node(TNode(2));
        let lock = g.add_node(TNode(3));
        let end = g.add_node(TNode(4));
        g.add_edge(start, key, TEdge(0));
        g.add_edge(key, lock, TEdge(0));
        g.add_edge(lock, end, TEdge(0));

        let constraint = LockKeyConstraint::new(start, std::vec![(2u8, 3u8)]);
        assert!(constraint.check(&g));
    }

    #[test]
    fn lock_without_key_fails() {
        // Start(1) -> Lock(3) -> End(4), no key
        let mut g = Graph::<TNode, TEdge>::new();
        let start = g.add_node(TNode(1));
        let lock = g.add_node(TNode(3));
        let end = g.add_node(TNode(4));
        g.add_edge(start, lock, TEdge(0));
        g.add_edge(lock, end, TEdge(0));

        let constraint = LockKeyConstraint::new(start, std::vec![(2u8, 3u8)]);
        assert!(!constraint.check(&g));
    }

    #[test]
    fn multiple_lock_key_pairs() {
        // Start(1) -> Key1(2) -> Lock1(3) -> Key2(4) -> Lock2(5)
        // Pairs: (2=key1, 3=lock1), (4=key2, 5=lock2)
        let mut g = Graph::<TNode, TEdge>::new();
        let start = g.add_node(TNode(1));
        let k1 = g.add_node(TNode(2));
        let l1 = g.add_node(TNode(3));
        let k2 = g.add_node(TNode(4));
        let l2 = g.add_node(TNode(5));
        g.add_edge(start, k1, TEdge(0));
        g.add_edge(k1, l1, TEdge(0));
        g.add_edge(l1, k2, TEdge(0));
        g.add_edge(k2, l2, TEdge(0));

        let constraint = LockKeyConstraint::new(start, std::vec![(2u8, 3u8), (4u8, 5u8)]);
        assert!(constraint.check(&g));
    }
}
