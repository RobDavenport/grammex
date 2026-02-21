//! Integration test: Parallel rewriting on a grid graph.

use grammex::*;
use rand::SeedableRng as _;

#[derive(Clone, PartialEq, Eq, Debug)]
struct GNode(u8); // 1 = A, 2 = B, 3 = C

impl NodeData for GNode {
    type Kind = u8;
    fn kind(&self) -> u8 { self.0 }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct GEdge(u8);

impl EdgeData for GEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { self.0 }
}

/// Build a grid graph: 4x4 grid of A(1) nodes connected horizontally
fn build_grid() -> Graph<GNode, GEdge> {
    let mut g = Graph::new();
    let mut ids = vec![];

    // Create 4x4 grid of nodes (alternating A and B for pattern matching)
    for row in 0..4 {
        for col in 0..4 {
            let kind = if (row + col) % 2 == 0 { 1 } else { 2 };
            ids.push(g.add_node(GNode(kind)));
        }
    }

    // Connect horizontally: A(1) -> B(2) edges
    for row in 0..4 {
        for col in 0..3 {
            let src = row * 4 + col;
            let tgt = row * 4 + col + 1;
            g.add_edge(ids[src], ids[tgt], GEdge(10));
        }
    }

    g
}

fn insert_node_rule() -> Rule<GNode, GEdge> {
    // Pattern: A(1) -[10]-> B(2)
    // Replace with: A(1) -[10]-> C(3) -[10]-> B(2) (insert node between)
    let mut lhs = Pattern::new();
    let pa = lhs.add_node(1u8);
    let pb = lhs.add_node(2u8);
    lhs.add_edge(pa, pb, 10u8);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(1)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode {
                from_lhs: None,
                action: ReplacementAction::Keep,
                data: Some(GNode(3)),
            },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(2), GEdge(10)), // A -> C
            (pattern::LocalId(2), pattern::LocalId(1), GEdge(10)), // C -> B
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
            Reconnection { from: pattern::LocalId(1), to: pattern::LocalId(1) },
        ],
    };

    Rule { lhs, rhs, weight: 1, name: "insert_c" }
}

#[test]
fn parallel_rewriting_inserts_nodes() {
    let g = build_grid();
    let initial_nodes = g.node_count();
    let _initial_edges = g.edge_count();

    let config = RewriterConfig::new()
        .max_steps(1)
        .selection(SelectionStrategy::Parallel);

    let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
    rewriter.add_rule(insert_node_rule());

    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    let result = rewriter.rewrite(&mut rng).unwrap();

    // Parallel should have inserted multiple C nodes in one step
    let c_count = result.graph.node_ids()
        .filter(|&id| result.graph.node(id).unwrap().kind() == 3)
        .count();

    assert!(c_count > 0, "Should have inserted at least one C node");
    assert!(
        result.graph.node_count() > initial_nodes,
        "Node count should have increased: {} > {}",
        result.graph.node_count(),
        initial_nodes
    );
}

#[test]
fn parallel_non_overlapping_matches() {
    let g = build_grid();

    let config = RewriterConfig::new()
        .max_steps(1)
        .selection(SelectionStrategy::Parallel);

    let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
    rewriter.add_rule(insert_node_rule());

    let mut rng = rand::rngs::SmallRng::seed_from_u64(99);
    let result = rewriter.rewrite(&mut rng).unwrap();

    // All original nodes should still exist (we keep A and B, add C)
    let a_count = result.graph.node_ids()
        .filter(|&id| result.graph.node(id).unwrap().kind() == 1)
        .count();
    let b_count = result.graph.node_ids()
        .filter(|&id| result.graph.node(id).unwrap().kind() == 2)
        .count();

    assert!(a_count > 0, "A nodes should still exist");
    assert!(b_count > 0, "B nodes should still exist");
}
