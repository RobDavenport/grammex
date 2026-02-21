//! Integration test: Quest grammar with acyclicity constraint.

use grammex::*;
use rand::SeedableRng as _;

#[derive(Clone, PartialEq, Eq, Debug)]
enum QNode {
    Objective,
    SubObjective,
    Reward,
    Prerequisite,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum QKind {
    Objective,
    SubObjective,
    Reward,
    Prerequisite,
}

impl NodeData for QNode {
    type Kind = QKind;
    fn kind(&self) -> QKind {
        match self {
            QNode::Objective => QKind::Objective,
            QNode::SubObjective => QKind::SubObjective,
            QNode::Reward => QKind::Reward,
            QNode::Prerequisite => QKind::Prerequisite,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct QEdge;

impl EdgeData for QEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { 0 }
}

fn expand_objective_rule() -> Rule<QNode, QEdge> {
    // Objective -> Objective -[0]-> SubObjective -[0]-> Reward
    let mut lhs = Pattern::new();
    lhs.add_node(QKind::Objective);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(QNode::SubObjective) },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(QNode::Reward) },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), QEdge),
            (pattern::LocalId(1), pattern::LocalId(2), QEdge),
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 3, name: "expand_objective" }
}

fn add_prerequisite_rule() -> Rule<QNode, QEdge> {
    // SubObjective -> SubObjective + Prerequisite -> SubObjective
    let mut lhs = Pattern::new();
    lhs.add_node(QKind::SubObjective);

    let rhs = Replacement {
        nodes: vec![
            ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: ReplacementAction::Keep,
                data: None,
            },
            ReplacementNode { from_lhs: None, action: ReplacementAction::Keep, data: Some(QNode::Prerequisite) },
        ],
        edges: vec![
            (pattern::LocalId(1), pattern::LocalId(0), QEdge), // Prereq -> SubObj
        ],
        reconnections: vec![
            Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 2, name: "add_prerequisite" }
}

#[test]
fn quest_grammar_acyclic() {
    let mut g = Graph::new();
    g.add_node(QNode::Objective);

    let config = RewriterConfig::new()
        .max_steps(30)
        .selection(SelectionStrategy::WeightedRandom);

    let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
    rewriter.add_rule(expand_objective_rule());
    rewriter.add_rule(add_prerequisite_rule());
    rewriter.add_constraint(AcyclicConstraint);

    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    let result = rewriter.rewrite(&mut rng).unwrap();

    // Verify: graph expanded
    assert!(result.graph.node_count() > 1, "Quest should expand");

    // Verify: still acyclic (constraint should have ensured this)
    assert!(AcyclicConstraint.check(&result.graph), "Quest graph must be acyclic");
}

#[test]
fn quest_grammar_multiple_seeds() {
    for seed in 0..20 {
        let mut g = Graph::new();
        g.add_node(QNode::Objective);

        let config = RewriterConfig::new()
            .max_steps(30)
            .selection(SelectionStrategy::WeightedRandom);

        let mut rewriter = grammex::rewriter::Rewriter::new(g, config);
        rewriter.add_rule(expand_objective_rule());
        rewriter.add_rule(add_prerequisite_rule());
        rewriter.add_constraint(AcyclicConstraint);

        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let result = rewriter.rewrite(&mut rng).unwrap();

        assert!(
            AcyclicConstraint.check(&result.graph),
            "Seed {seed}: quest graph must be acyclic"
        );
    }
}

// Need StructuralConstraint trait access for AcyclicConstraint.check()
use grammex::constraint::StructuralConstraint;
