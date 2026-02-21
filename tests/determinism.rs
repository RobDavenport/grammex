//! Integration test: Deterministic output — same seed produces identical graphs.

use grammex::*;
use rand::SeedableRng as _;

#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(dead_code)]
enum DNode {
    Start,
    Room,
    Key,
    Lock,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DKind {
    Start,
    Room,
    Key,
    Lock,
}

impl NodeData for DNode {
    type Kind = DKind;
    fn kind(&self) -> DKind {
        match self {
            DNode::Start => DKind::Start,
            DNode::Room => DKind::Room,
            DNode::Key => DKind::Key,
            DNode::Lock => DKind::Lock,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct DEdge;

impl EdgeData for DEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { 0 }
}

fn expand_rule() -> Rule<DNode, DEdge> {
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Room);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Room) },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 3, name: "expand" }
}

fn start_rule() -> Rule<DNode, DEdge> {
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Start);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Room) },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
        ],
        reconnections: vec![],
    };

    Rule { lhs, rhs, weight: 2, name: "start_expand" }
}

/// Snapshot graph state as sorted vectors for comparison
fn snapshot(g: &Graph<DNode, DEdge>) -> (Vec<DKind>, Vec<(usize, usize)>) {
    let mut node_kinds: Vec<DKind> = g.node_ids()
        .map(|id| g.node(id).unwrap().kind())
        .collect();
    node_kinds.sort_by_key(|k| match k {
        DKind::Start => 0,
        DKind::Room => 1,
        DKind::Key => 2,
        DKind::Lock => 3,
    });

    let mut edges: Vec<(usize, usize)> = g.edge_ids()
        .filter_map(|eid| g.edge_endpoints(eid))
        .map(|(s, t)| (s.0, t.0))
        .collect();
    edges.sort();

    (node_kinds, edges)
}

fn run_grammar(seed: u64) -> (Vec<DKind>, Vec<(usize, usize)>) {
    let mut g = Graph::new();
    g.add_node(DNode::Start);

    let config = RewriterConfig::new()
        .max_steps(20)
        .selection(SelectionStrategy::WeightedRandom);

    let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
    rewriter.add_rule(start_rule());
    rewriter.add_rule(expand_rule());

    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
    let result = rewriter.rewrite(&mut rng).unwrap();
    snapshot(&result.graph)
}

#[test]
fn determinism_same_seed_same_result() {
    for seed in 0..10 {
        let result1 = run_grammar(seed);
        let result2 = run_grammar(seed);
        assert_eq!(
            result1, result2,
            "Seed {seed}: same seed must produce identical results"
        );
    }
}

#[test]
fn determinism_100_iterations() {
    let reference = run_grammar(12345);
    for _ in 0..100 {
        let result = run_grammar(12345);
        assert_eq!(reference, result, "Every run with seed 12345 must match");
    }
}
