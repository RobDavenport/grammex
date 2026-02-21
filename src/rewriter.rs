//! Core graph rewriting engine.

use alloc::vec::Vec;
use crate::graph::{Graph, NodeId};
use crate::node::NodeData;
use crate::edge::EdgeData;
use crate::rule::{Rule, ReplacementAction};
use crate::pattern::{self, Match};
use crate::strategy;
use crate::constraint::StructuralConstraint;
use crate::config::{RewriterConfig, SelectionStrategy};
use crate::observer::RewriteObserver;
use crate::error::RewriteError;
use rand_core::RngCore;

/// Result of a single rewrite step.
#[derive(Debug)]
pub enum StepResult {
    /// A rule was applied.
    Applied {
        rule_index: usize,
        rule_name: &'static str,
    },
    /// No rule matched -- rewriting is complete.
    NoMatch,
}

/// Result of a complete rewrite.
pub struct RewriteResult<N: NodeData, E: EdgeData> {
    pub graph: Graph<N, E>,
    pub steps: usize,
    pub rules_applied: Vec<usize>,
}

/// The graph rewriting engine.
pub struct Rewriter<N: NodeData, E: EdgeData> {
    graph: Graph<N, E>,
    rules: Vec<Rule<N, E>>,
    constraints: Vec<alloc::boxed::Box<dyn StructuralConstraint<N, E>>>,
    config: RewriterConfig,
    steps: usize,
}

impl<N: NodeData, E: EdgeData> Rewriter<N, E> {
    /// Create a new rewriter with the given initial graph and configuration.
    pub fn new(graph: Graph<N, E>, config: RewriterConfig) -> Self {
        Self {
            graph,
            rules: Vec::new(),
            constraints: Vec::new(),
            config,
            steps: 0,
        }
    }

    /// Add a rewriting rule.
    pub fn add_rule(&mut self, rule: Rule<N, E>) {
        self.rules.push(rule);
    }

    /// Add a structural constraint checked after each rule application.
    pub fn add_constraint(&mut self, constraint: impl StructuralConstraint<N, E> + 'static) {
        self.constraints.push(alloc::boxed::Box::new(constraint));
    }

    /// Perform one rewrite step: match -> select -> apply -> verify constraints.
    pub fn step(&mut self, rng: &mut impl RngCore) -> StepResult {
        // 1. Find all matches for all rules
        let mut candidates: Vec<(usize, Match, u32)> = Vec::new();
        for (rule_idx, rule) in self.rules.iter().enumerate() {
            let matches = pattern::find_matches(&self.graph, &rule.lhs);
            for m in matches {
                candidates.push((rule_idx, m, rule.weight));
            }
        }

        if candidates.is_empty() {
            return StepResult::NoMatch;
        }

        // 2. Select based on strategy
        match self.config.selection {
            SelectionStrategy::FirstMatch => {
                let (rule_idx, m, _) = candidates.into_iter().next().unwrap();
                self.try_apply_rule(rule_idx, m)
            }
            SelectionStrategy::WeightedRandom => {
                if let Some((rule_idx, m)) = strategy::select_match(&candidates, rng) {
                    self.try_apply_rule(rule_idx, m)
                } else {
                    StepResult::NoMatch
                }
            }
            SelectionStrategy::Parallel => {
                let pairs: Vec<(usize, Match)> = candidates.into_iter()
                    .map(|(idx, m, _)| (idx, m))
                    .collect();
                let non_overlapping = strategy::find_non_overlapping(&pairs);
                if non_overlapping.is_empty() {
                    return StepResult::NoMatch;
                }
                // Snapshot for rollback
                let snapshot = self.graph.clone();
                let mut last_rule_idx = 0;
                for (rule_idx, m) in non_overlapping {
                    last_rule_idx = rule_idx;
                    apply_rule(&mut self.graph, &self.rules[rule_idx], &m);
                }
                // Check constraints
                for constraint in &self.constraints {
                    if !constraint.check(&self.graph) {
                        self.graph = snapshot;
                        return StepResult::Applied {
                            rule_index: last_rule_idx,
                            rule_name: self.rules[last_rule_idx].name,
                        };
                    }
                }
                self.steps += 1;
                StepResult::Applied {
                    rule_index: last_rule_idx,
                    rule_name: self.rules[last_rule_idx].name,
                }
            }
        }
    }

    fn try_apply_rule(&mut self, rule_idx: usize, m: Match) -> StepResult {
        // 3. Snapshot graph (for constraint rollback)
        let snapshot = self.graph.clone();

        // 4. Apply rule
        apply_rule(&mut self.graph, &self.rules[rule_idx], &m);

        // 5. Check constraints
        for constraint in &self.constraints {
            if !constraint.check(&self.graph) {
                self.graph = snapshot; // rollback
                return StepResult::Applied {
                    rule_index: rule_idx,
                    rule_name: self.rules[rule_idx].name,
                };
            }
        }

        self.steps += 1;
        StepResult::Applied {
            rule_index: rule_idx,
            rule_name: self.rules[rule_idx].name,
        }
    }

    /// Perform one rewrite step with observer callbacks.
    pub fn step_observed(&mut self, rng: &mut impl RngCore, observer: &mut impl RewriteObserver<N, E>) -> StepResult {
        // 1. Find all matches for all rules
        let mut candidates: Vec<(usize, Match, u32)> = Vec::new();
        for (rule_idx, rule) in self.rules.iter().enumerate() {
            let matches = pattern::find_matches(&self.graph, &rule.lhs);
            for m in matches {
                candidates.push((rule_idx, m, rule.weight));
            }
        }

        if candidates.is_empty() {
            observer.on_no_match();
            return StepResult::NoMatch;
        }

        // 2. Select
        let selected = match self.config.selection {
            SelectionStrategy::FirstMatch => {
                let (rule_idx, m, _) = candidates.into_iter().next().unwrap();
                Some((rule_idx, m))
            }
            SelectionStrategy::WeightedRandom => {
                strategy::select_match(&candidates, rng)
            }
            SelectionStrategy::Parallel => {
                // For observed mode, we just pick the first non-overlapping
                let pairs: Vec<(usize, Match)> = candidates.into_iter()
                    .map(|(idx, m, _)| (idx, m))
                    .collect();
                let non_overlapping = strategy::find_non_overlapping(&pairs);
                non_overlapping.into_iter().next()
            }
        };

        let (rule_idx, m) = match selected {
            Some(s) => s,
            None => {
                observer.on_no_match();
                return StepResult::NoMatch;
            }
        };

        // Notify observer of match
        observer.on_match(rule_idx, &m.node_map);

        // 3. Snapshot
        let snapshot = self.graph.clone();

        // 4. Apply
        apply_rule(&mut self.graph, &self.rules[rule_idx], &m);
        observer.on_rule_applied(rule_idx, self.rules[rule_idx].name);

        // 5. Check constraints
        for constraint in &self.constraints {
            if !constraint.check(&self.graph) {
                self.graph = snapshot;
                observer.on_constraint_violated(rule_idx);
                return StepResult::Applied {
                    rule_index: rule_idx,
                    rule_name: self.rules[rule_idx].name,
                };
            }
        }

        self.steps += 1;
        observer.on_step_complete(self.steps, &self.graph);
        StepResult::Applied {
            rule_index: rule_idx,
            rule_name: self.rules[rule_idx].name,
        }
    }

    /// Run rewriting to completion (until no rules match or max_steps reached).
    pub fn rewrite(mut self, rng: &mut impl RngCore) -> Result<RewriteResult<N, E>, RewriteError> {
        let max_steps = self.config.max_steps;
        let mut rules_applied = Vec::new();

        loop {
            if self.steps >= max_steps {
                return Ok(RewriteResult {
                    graph: self.graph,
                    steps: self.steps,
                    rules_applied,
                });
            }

            match self.step(rng) {
                StepResult::Applied { rule_index, .. } => {
                    rules_applied.push(rule_index);
                }
                StepResult::NoMatch => {
                    return Ok(RewriteResult {
                        graph: self.graph,
                        steps: self.steps,
                        rules_applied,
                    });
                }
            }
        }
    }

    /// Get a reference to the current graph state.
    pub fn graph(&self) -> &Graph<N, E> {
        &self.graph
    }

    /// Get the number of steps performed so far.
    pub fn steps(&self) -> usize {
        self.steps
    }
}

/// Apply a single rule to the graph at the given match.
fn apply_rule<N: NodeData, E: EdgeData>(
    graph: &mut Graph<N, E>,
    rule: &Rule<N, E>,
    m: &Match,
) {
    let matched_nodes = &m.node_map;

    // 1. Create replacement node map: rhs_local_id -> graph NodeId
    let mut new_node_ids: Vec<Option<NodeId>> = Vec::new();
    let mut nodes_to_remove: Vec<NodeId> = Vec::new();

    for rhs_node in &rule.rhs.nodes {
        if let Some(lhs_local) = rhs_node.from_lhs {
            let graph_id = matched_nodes[lhs_local.0];
            match &rhs_node.action {
                ReplacementAction::Keep => {
                    new_node_ids.push(Some(graph_id));
                }
                ReplacementAction::Replace(new_data) => {
                    if let Some(node) = graph.node_mut(graph_id) {
                        *node = new_data.clone();
                    }
                    new_node_ids.push(Some(graph_id));
                }
                ReplacementAction::Remove => {
                    nodes_to_remove.push(graph_id);
                    new_node_ids.push(None);
                }
            }
        } else {
            // New node
            let data = rhs_node.data.clone().expect("New RHS node must have data");
            let id = graph.add_node(data);
            new_node_ids.push(Some(id));
        }
    }

    // 2. Add replacement edges
    for (src_local, tgt_local, edge_data) in &rule.rhs.edges {
        if let (Some(src_id), Some(tgt_id)) = (new_node_ids[src_local.0], new_node_ids[tgt_local.0]) {
            graph.add_edge(src_id, tgt_id, edge_data.clone());
        }
    }

    // 3. Handle reconnections: redirect external edges
    for reconnection in &rule.rhs.reconnections {
        let old_node = matched_nodes[reconnection.from.0];
        if let Some(new_node) = new_node_ids[reconnection.to.0] {
            redirect_edges(graph, old_node, new_node, matched_nodes);
        }
    }

    // 4. Remove old matched nodes that aren't kept in the replacement
    let kept_nodes: Vec<NodeId> = new_node_ids.iter().filter_map(|n| *n).collect();
    for &matched_id in matched_nodes {
        if !kept_nodes.contains(&matched_id) {
            graph.remove_node(matched_id);
        }
    }

    // Remove explicitly marked nodes
    for node_id in nodes_to_remove {
        if !kept_nodes.contains(&node_id) {
            graph.remove_node(node_id);
        }
    }
}

/// Redirect external edges from old_node to new_node.
/// Only redirects edges that connect to nodes OUTSIDE the matched set.
fn redirect_edges<N: NodeData, E: EdgeData>(
    graph: &mut Graph<N, E>,
    old_node: NodeId,
    new_node: NodeId,
    matched_nodes: &[NodeId],
) {
    if old_node == new_node {
        return;
    }

    // Collect external edges to redirect (incoming to old_node from outside match)
    let incoming: Vec<_> = graph.incoming(old_node).to_vec();
    for eid in incoming {
        if let Some((source, _)) = graph.edge_endpoints(eid) {
            if !matched_nodes.contains(&source) {
                if let Some(edata) = graph.edge(eid) {
                    let edata = edata.clone();
                    graph.remove_edge(eid);
                    graph.add_edge(source, new_node, edata);
                }
            }
        }
    }

    // Collect external edges to redirect (outgoing from old_node to outside match)
    let outgoing: Vec<_> = graph.outgoing(old_node).to_vec();
    for eid in outgoing {
        if let Some((_, target)) = graph.edge_endpoints(eid) {
            if !matched_nodes.contains(&target) {
                if let Some(edata) = graph.edge(eid) {
                    let edata = edata.clone();
                    graph.remove_edge(eid);
                    graph.add_edge(new_node, target, edata);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::graph::Graph;
    use crate::pattern::{Pattern, LocalId};
    use crate::rule::{Rule, Replacement, ReplacementNode, ReplacementAction, Reconnection};
    use crate::config::{RewriterConfig, SelectionStrategy};
    use crate::observer::RewriteObserver;

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

    struct FakeRng(u64);
    impl RngCore for FakeRng {
        fn next_u32(&mut self) -> u32 { self.next_u64() as u32 }
        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(8) {
                let val = self.next_u64();
                let bytes = val.to_le_bytes();
                let len = chunk.len().min(8);
                chunk[..len].copy_from_slice(&bytes[..len]);
            }
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    /// Helper: build a rule that expands a single "Start"(1) node into
    /// Start -[0]-> Room(2) -[0]-> Room(2)
    fn expansion_rule() -> Rule<TNode, TEdge> {
        let mut lhs = Pattern::new();
        lhs.add_node(1u8); // match Start

        let rhs = Replacement {
            nodes: std::vec![
                // RHS node 0: keep the Start
                ReplacementNode { from_lhs: Some(LocalId(0)), action: ReplacementAction::Keep, data: None },
                // RHS node 1: new Room
                ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(TNode(2)) },
                // RHS node 2: new Room
                ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(TNode(2)) },
            ],
            edges: std::vec![
                (LocalId(0), LocalId(1), TEdge(0)), // Start -> Room1
                (LocalId(1), LocalId(2), TEdge(0)), // Room1 -> Room2
            ],
            reconnections: std::vec![
                Reconnection { from: LocalId(0), to: LocalId(0) },
            ],
        };

        Rule { lhs, rhs, weight: 1, name: "expand_start" }
    }

    #[test]
    fn simple_expansion() {
        // Start with single Start node, apply expansion rule
        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1)); // Start

        let rule = expansion_rule();
        let matches = pattern::find_matches(&g, &rule.lhs);
        assert_eq!(matches.len(), 1);

        apply_rule(&mut g, &rule, &matches[0]);

        // Should now have: Start(1), Room(2), Room(2) = 3 nodes
        assert_eq!(g.node_count(), 3);
        // 2 edges: Start->Room1, Room1->Room2
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn node_replacement() {
        // Replace "Empty"(5) with "Key"(6), preserving edges
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let empty = g.add_node(TNode(5));
        let c = g.add_node(TNode(3));
        g.add_edge(a, empty, TEdge(0));
        g.add_edge(empty, c, TEdge(0));

        let mut lhs = Pattern::new();
        lhs.add_node(5u8); // match Empty

        let rhs = Replacement {
            nodes: std::vec![
                ReplacementNode {
                    from_lhs: Some(LocalId(0)),
                    action: ReplacementAction::Replace(TNode(6)),
                    data: None,
                },
            ],
            edges: std::vec![],
            reconnections: std::vec![
                Reconnection { from: LocalId(0), to: LocalId(0) },
            ],
        };

        let rule = Rule { lhs, rhs, weight: 1, name: "replace_empty" };
        let matches = pattern::find_matches(&g, &rule.lhs);
        assert_eq!(matches.len(), 1);

        apply_rule(&mut g, &rule, &matches[0]);

        // Same number of nodes, but empty is now Key(6)
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.node(empty).unwrap().kind(), 6);
        // Edges preserved
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn reconnection() {
        // A(1) -> B(2) -> C(3), replace B with D(4)
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(2));
        let c = g.add_node(TNode(3));
        g.add_edge(a, b, TEdge(0));
        g.add_edge(b, c, TEdge(0));

        let mut lhs = Pattern::new();
        lhs.add_node(2u8); // match B

        let rhs = Replacement {
            nodes: std::vec![
                // New node D replaces B
                ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(TNode(4)) },
            ],
            edges: std::vec![],
            reconnections: std::vec![
                Reconnection { from: LocalId(0), to: LocalId(0) }, // redirect B's edges to D
            ],
        };

        let rule = Rule { lhs, rhs, weight: 1, name: "replace_b" };
        let matches = pattern::find_matches(&g, &rule.lhs);
        assert_eq!(matches.len(), 1);

        apply_rule(&mut g, &rule, &matches[0]);

        // B removed, D added = 3 nodes (A, C, D)
        assert_eq!(g.node_count(), 3);
        assert!(g.node(b).is_none()); // B removed

        // Find node D (should be kind 4)
        let d_id: Vec<_> = g.node_ids().filter(|&id| g.node(id).unwrap().kind() == 4).collect();
        assert_eq!(d_id.len(), 1);
        let d = d_id[0];

        // A -> D and D -> C edges should exist
        let a_out: Vec<_> = g.outgoing(a).to_vec();
        assert_eq!(a_out.len(), 1);
        assert_eq!(g.edge_endpoints(a_out[0]), Some((a, d)));

        let d_out: Vec<_> = g.outgoing(d).to_vec();
        assert_eq!(d_out.len(), 1);
        assert_eq!(g.edge_endpoints(d_out[0]), Some((d, c)));
    }

    #[test]
    fn step_no_match() {
        // No rules → NoMatch
        let g = Graph::<TNode, TEdge>::new();
        let config = RewriterConfig::new().selection(SelectionStrategy::FirstMatch);
        let mut rewriter = Rewriter::new(g, config);
        let mut rng = FakeRng(42);

        match rewriter.step(&mut rng) {
            StepResult::NoMatch => {}
            _ => panic!("Expected NoMatch"),
        }
    }

    #[test]
    fn step_applies_rule() {
        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1)); // Start

        let config = RewriterConfig::new().selection(SelectionStrategy::FirstMatch);
        let mut rewriter = Rewriter::new(g, config);
        rewriter.add_rule(expansion_rule());

        let mut rng = FakeRng(42);
        match rewriter.step(&mut rng) {
            StepResult::Applied { rule_name, .. } => {
                assert_eq!(rule_name, "expand_start");
            }
            StepResult::NoMatch => panic!("Expected Applied"),
        }

        assert_eq!(rewriter.graph().node_count(), 3);
    }

    #[test]
    fn step_constraint_rollback() {
        // Rule adds a cycle, AcyclicConstraint should cause rollback
        let mut g = Graph::<TNode, TEdge>::new();
        let a = g.add_node(TNode(1));
        let b = g.add_node(TNode(2));
        g.add_edge(a, b, TEdge(0));

        // Rule: match node(2), add edge back to create cycle
        let mut lhs = Pattern::new();
        lhs.add_node(2u8);

        let rhs = Replacement {
            nodes: std::vec![
                ReplacementNode { from_lhs: Some(LocalId(0)), action: ReplacementAction::Keep, data: None },
                ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(TNode(3)) },
            ],
            edges: std::vec![
                (LocalId(0), LocalId(1), TEdge(0)),
                (LocalId(1), LocalId(0), TEdge(0)), // creates cycle potential
            ],
            reconnections: std::vec![],
        };

        let rule = Rule { lhs, rhs, weight: 1, name: "cycle_maker" };

        let config = RewriterConfig::new().selection(SelectionStrategy::FirstMatch);
        let mut rewriter = Rewriter::new(g, config);
        rewriter.add_rule(rule);
        rewriter.add_constraint(crate::constraint::AcyclicConstraint);

        let mut rng = FakeRng(42);
        // Step should apply but rollback due to constraint
        match rewriter.step(&mut rng) {
            StepResult::Applied { .. } => {}
            StepResult::NoMatch => panic!("Expected Applied (with rollback)"),
        }

        // Graph should be unchanged (rolled back)
        assert_eq!(rewriter.graph().node_count(), 2);
        assert_eq!(rewriter.graph().edge_count(), 1);
    }

    #[test]
    fn rewrite_runs_to_completion() {
        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1)); // Start

        let config = RewriterConfig::new()
            .max_steps(5)
            .selection(SelectionStrategy::FirstMatch);
        let mut rewriter = Rewriter::new(g, config);
        rewriter.add_rule(expansion_rule());

        let mut rng = FakeRng(42);
        let result = rewriter.rewrite(&mut rng).unwrap();

        // After first step: Start no longer matches kind 1 for expansion
        // Actually, Start is kept so it still matches → will keep expanding
        // But max_steps=5 should limit it
        assert!(result.steps <= 5);
        assert!(result.graph.node_count() > 1);
    }

    #[test]
    fn step_observed_calls_observer() {
        struct TestObserver {
            match_called: bool,
            applied_called: bool,
            step_complete_called: bool,
        }
        impl RewriteObserver<TNode, TEdge> for TestObserver {
            fn on_match(&mut self, _rule_index: usize, _matched_nodes: &[NodeId]) {
                self.match_called = true;
            }
            fn on_rule_applied(&mut self, _rule_index: usize, _rule_name: &str) {
                self.applied_called = true;
            }
            fn on_step_complete(&mut self, _step: usize, _graph: &Graph<TNode, TEdge>) {
                self.step_complete_called = true;
            }
        }

        let mut g = Graph::<TNode, TEdge>::new();
        g.add_node(TNode(1));

        let config = RewriterConfig::new().selection(SelectionStrategy::FirstMatch);
        let mut rewriter = Rewriter::new(g, config);
        rewriter.add_rule(expansion_rule());

        let mut rng = FakeRng(42);
        let mut observer = TestObserver {
            match_called: false,
            applied_called: false,
            step_complete_called: false,
        };

        rewriter.step_observed(&mut rng, &mut observer);

        assert!(observer.match_called, "on_match should be called");
        assert!(observer.applied_called, "on_rule_applied should be called");
        assert!(observer.step_complete_called, "on_step_complete should be called");
    }
}
