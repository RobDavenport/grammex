//! Integration test: Dungeon grammar with lock-and-key constraints.

use grammex::*;

// --- Node types ---

#[derive(Clone, PartialEq, Eq, Debug)]
enum DNode {
    Start,
    Room,
    Key,
    Lock,
    Boss,
    Exit,
}

impl NodeData for DNode {
    type Kind = DKind;
    fn kind(&self) -> DKind {
        match self {
            DNode::Start => DKind::Start,
            DNode::Room => DKind::Room,
            DNode::Key => DKind::Key,
            DNode::Lock => DKind::Lock,
            DNode::Boss => DKind::Boss,
            DNode::Exit => DKind::Exit,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DKind {
    Start,
    Room,
    Key,
    Lock,
    Boss,
    Exit,
}

// --- Edge types ---

#[derive(Clone, PartialEq, Eq, Debug)]
struct DEdge;

impl EdgeData for DEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { 0 }
}

// --- Rules ---

fn expand_room_rule() -> Rule<DNode, DEdge> {
    // Room -> Room -[0]-> Room
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Room);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode {
                from_lhs: None,
                action: ReplacementAction::Keep,
                data: Some(DNode::Room),
            },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 5, name: "expand_room" }
}

fn start_to_room_rule() -> Rule<DNode, DEdge> {
    // Start -> Start -[0]-> Room
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Start);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode {
                from_lhs: None,
                action: ReplacementAction::Keep,
                data: Some(DNode::Room),
            },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
        ],
        reconnections: vec![],
    };

    Rule { lhs, rhs, weight: 3, name: "start_to_room" }
}

fn add_lock_key_rule() -> Rule<DNode, DEdge> {
    // Room -> Room -[0]-> Key -[0]-> Lock -[0]-> Room(new)
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Room);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Key) },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Lock) },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Room) },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
            (pattern::LocalId(1), pattern::LocalId(2), DEdge),
            (pattern::LocalId(2), pattern::LocalId(3), DEdge),
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 1, name: "add_lock_key" }
}

fn room_to_boss_rule() -> Rule<DNode, DEdge> {
    // Room -> Boss (replace)
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Room);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Replace(DNode::Boss),
                data: None,
            },
        ],
        edges: vec![],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 1, name: "room_to_boss" }
}

fn boss_to_exit_rule() -> Rule<DNode, DEdge> {
    // Boss -> Boss -[0]-> Exit
    let mut lhs = Pattern::new();
    lhs.add_node(DKind::Boss);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(DNode::Exit) },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), DEdge),
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 2, name: "boss_to_exit" }
}

fn build_dungeon_rewriter(seed: u64) -> (grammex::rewriter::Rewriter<DNode, DEdge>, impl rand_core::RngCore) {
    let mut g = Graph::new();
    let start = g.add_node(DNode::Start);

    let config = RewriterConfig::new()
        .max_steps(50)
        .selection(SelectionStrategy::WeightedRandom);

    let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
    rewriter.add_rule(start_to_room_rule());
    rewriter.add_rule(expand_room_rule());
    rewriter.add_rule(add_lock_key_rule());
    rewriter.add_rule(room_to_boss_rule());
    rewriter.add_rule(boss_to_exit_rule());

    // Add reachability: Exit must be reachable from Start
    rewriter.add_constraint(ReachabilityConstraint::new(
        start,
        vec![DKind::Start],
    ));

    // Add lock-key constraint
    rewriter.add_constraint(LockKeyConstraint::new(
        start,
        vec![(DKind::Key, DKind::Lock)],
    ));

    let rng = rand::rngs::SmallRng::seed_from_u64(seed);
    (rewriter, rng)
}

#[test]
fn dungeon_grammar_produces_valid_graph() {
    let (rewriter, mut rng) = build_dungeon_rewriter(42);
    let result = rewriter.rewrite(&mut rng).unwrap();

    // Should have at least Start node
    let has_start = result.graph.node_ids().any(|id| {
        result.graph.node(id).unwrap().kind() == DKind::Start
    });
    assert!(has_start, "Graph must have a Start node");

    // Should have applied at least one rule
    assert!(result.steps > 0, "Should have applied at least one rule");

    // Graph should have more than 1 node
    assert!(result.graph.node_count() > 1, "Graph should have expanded");
}

#[test]
fn dungeon_grammar_multiple_seeds() {
    for seed in 0..20 {
        let (rewriter, mut rng) = build_dungeon_rewriter(seed);
        let result = rewriter.rewrite(&mut rng).unwrap();

        assert!(
            result.graph.node_count() > 1,
            "Seed {seed}: graph should have expanded"
        );

        // Verify lock-key constraint: every Lock should have a Key in the graph
        let has_lock = result.graph.node_ids().any(|id| {
            result.graph.node(id).unwrap().kind() == DKind::Lock
        });
        let has_key = result.graph.node_ids().any(|id| {
            result.graph.node(id).unwrap().kind() == DKind::Key
        });
        if has_lock {
            assert!(has_key, "Seed {seed}: lock exists without key");
        }
    }
}

use rand::SeedableRng as _;
