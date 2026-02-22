//! WASM bindings for the grammex interactive demo.

use wasm_bindgen::prelude::*;
use grammex::*;
use rand::SeedableRng;

// --- Demo node/edge types ---

#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(dead_code)]
enum DNode {
    Start,
    Room,
    Corridor,
    Key,
    Lock,
    Boss,
    Exit,
    Treasure,
    // Quest types
    Objective,
    SubObjective,
    Reward,
    Prerequisite,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DKind {
    Start,
    Room,
    Corridor,
    Key,
    Lock,
    Boss,
    Exit,
    Treasure,
    Objective,
    SubObjective,
    Reward,
    Prerequisite,
}

impl NodeData for DNode {
    type Kind = DKind;
    fn kind(&self) -> DKind {
        match self {
            DNode::Start => DKind::Start,
            DNode::Room => DKind::Room,
            DNode::Corridor => DKind::Corridor,
            DNode::Key => DKind::Key,
            DNode::Lock => DKind::Lock,
            DNode::Boss => DKind::Boss,
            DNode::Exit => DKind::Exit,
            DNode::Treasure => DKind::Treasure,
            DNode::Objective => DKind::Objective,
            DNode::SubObjective => DKind::SubObjective,
            DNode::Reward => DKind::Reward,
            DNode::Prerequisite => DKind::Prerequisite,
        }
    }
}

impl DKind {
    fn label(self) -> &'static str {
        match self {
            DKind::Start => "start",
            DKind::Room => "room",
            DKind::Corridor => "corridor",
            DKind::Key => "key",
            DKind::Lock => "lock",
            DKind::Boss => "boss",
            DKind::Exit => "exit",
            DKind::Treasure => "treasure",
            DKind::Objective => "objective",
            DKind::SubObjective => "task",
            DKind::Reward => "reward",
            DKind::Prerequisite => "prereq",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct DEdge;

impl EdgeData for DEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { 0 }
}

// --- Dungeon rules ---

fn dungeon_rules() -> Vec<Rule<DNode, DEdge>> {
    vec![
        // Rule 1: Start -> Start + Room
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Start);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Room) },
                    ],
                    edges: vec![(pattern::LocalId(0), pattern::LocalId(1), DEdge)],
                    reconnections: vec![],
                },
                weight: 3,
                name: "start_expand",
            }
        },
        // Rule 2: Room -> Room + Room (branch)
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Room);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Room) },
                    ],
                    edges: vec![(pattern::LocalId(0), pattern::LocalId(1), DEdge)],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 5,
                name: "expand_room",
            }
        },
        // Rule 3: Room -> Room + Key + Lock + Room (lock-and-key)
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Room);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Key) },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Lock) },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Room) },
                    ],
                    edges: vec![
                        (pattern::LocalId(0), pattern::LocalId(1), DEdge),
                        (pattern::LocalId(1), pattern::LocalId(2), DEdge),
                        (pattern::LocalId(2), pattern::LocalId(3), DEdge),
                    ],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 1,
                name: "add_lock_key",
            }
        },
        // Rule 4: Room -> Boss
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Room);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Replace(DNode::Boss), data: None },
                    ],
                    edges: vec![],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 1,
                name: "room_to_boss",
            }
        },
        // Rule 5: Boss -> Boss + Exit
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Boss);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Exit) },
                    ],
                    edges: vec![(pattern::LocalId(0), pattern::LocalId(1), DEdge)],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 2,
                name: "boss_to_exit",
            }
        },
    ]
}

fn quest_rules() -> Vec<Rule<DNode, DEdge>> {
    vec![
        // Objective -> Objective + SubObjective + Reward
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::Objective);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::SubObjective) },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Reward) },
                    ],
                    edges: vec![
                        (pattern::LocalId(0), pattern::LocalId(1), DEdge),
                        (pattern::LocalId(1), pattern::LocalId(2), DEdge),
                    ],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 3,
                name: "expand_objective",
            }
        },
        // SubObjective -> SubObjective + Prerequisite
        {
            let mut lhs = Pattern::new();
            lhs.add_node(DKind::SubObjective);
            Rule {
                lhs,
                rhs: rule::Replacement {
                    nodes: vec![
                        rule::ReplacementNode { from_lhs: Some(pattern::LocalId(0)), action: rule::ReplacementAction::Keep, data: None },
                        rule::ReplacementNode { from_lhs: None, action: rule::ReplacementAction::Keep, data: Some(DNode::Prerequisite) },
                    ],
                    edges: vec![
                        (pattern::LocalId(1), pattern::LocalId(0), DEdge),
                    ],
                    reconnections: vec![
                        rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    ],
                },
                weight: 2,
                name: "add_prerequisite",
            }
        },
    ]
}

// --- Simple force-directed layout ---

fn force_layout(graph: &Graph<DNode, DEdge>, width: f64, height: f64) -> Vec<(f64, f64)> {
    let node_ids: Vec<NodeId> = graph.node_ids().collect();
    let n = node_ids.len();
    if n == 0 {
        return vec![];
    }

    // Initialize positions in a circle
    let cx = width / 2.0;
    let cy = height / 2.0;
    let radius = (width.min(height) * 0.35).max(50.0);

    let mut pos: Vec<(f64, f64)> = node_ids.iter().enumerate().map(|(i, _)| {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64) - std::f64::consts::FRAC_PI_2;
        (cx + radius * angle.cos(), cy + radius * angle.sin())
    }).collect();

    // Build index map: NodeId -> index
    let id_to_idx: std::collections::HashMap<usize, usize> = node_ids.iter().enumerate()
        .map(|(i, nid)| (nid.0, i))
        .collect();

    // Run force-directed iterations
    let repulsion = 5000.0;
    let attraction = 0.01;
    let damping = 0.9;
    let iterations = 100;

    let mut vel: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

    for _ in 0..iterations {
        let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

        // Repulsion between all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let dist_sq = dx * dx + dy * dy + 1.0;
                let force = repulsion / dist_sq;
                let dist = dist_sq.sqrt();
                let fx = force * dx / dist;
                let fy = force * dy / dist;
                forces[i].0 += fx;
                forces[i].1 += fy;
                forces[j].0 -= fx;
                forces[j].1 -= fy;
            }
        }

        // Attraction along edges
        for eid in graph.edge_ids() {
            if let Some((src, tgt)) = graph.edge_endpoints(eid) {
                if let (Some(&si), Some(&ti)) = (id_to_idx.get(&src.0), id_to_idx.get(&tgt.0)) {
                    let dx = pos[si].0 - pos[ti].0;
                    let dy = pos[si].1 - pos[ti].1;
                    let fx = -attraction * dx;
                    let fy = -attraction * dy;
                    forces[si].0 += fx;
                    forces[si].1 += fy;
                    forces[ti].0 -= fx;
                    forces[ti].1 -= fy;
                }
            }
        }

        // Center gravity
        for i in 0..n {
            let dx = cx - pos[i].0;
            let dy = cy - pos[i].1;
            forces[i].0 += dx * 0.001;
            forces[i].1 += dy * 0.001;
        }

        // Apply forces
        for i in 0..n {
            vel[i].0 = (vel[i].0 + forces[i].0) * damping;
            vel[i].1 = (vel[i].1 + forces[i].1) * damping;
            pos[i].0 += vel[i].0;
            pos[i].1 += vel[i].1;

            // Clamp to bounds
            pos[i].0 = pos[i].0.clamp(30.0, width - 30.0);
            pos[i].1 = pos[i].1.clamp(30.0, height - 30.0);
        }
    }

    pos
}

// --- JSON helpers ---

fn graph_to_json(graph: &Graph<DNode, DEdge>, width: f64, height: f64) -> String {
    let node_ids: Vec<NodeId> = graph.node_ids().collect();
    let positions = force_layout(graph, width, height);

    let mut json = String::from("{\"nodes\":[");
    for (i, &nid) in node_ids.iter().enumerate() {
        if i > 0 { json.push(','); }
        let kind = graph.node(nid).unwrap().kind();
        let label = kind.label();
        let (x, y) = if i < positions.len() { positions[i] } else { (400.0, 300.0) };
        json.push_str(&format!(
            "{{\"id\":{},\"kind\":\"{}\",\"label\":\"{}\",\"x\":{:.1},\"y\":{:.1}}}",
            nid.0, label, label, x, y
        ));
    }
    json.push_str("],\"edges\":[");

    let mut first = true;
    for eid in graph.edge_ids() {
        if let Some((src, tgt)) = graph.edge_endpoints(eid) {
            if !first { json.push(','); }
            first = false;
            json.push_str(&format!(
                "{{\"source\":{},\"target\":{},\"kind\":\"connection\"}}",
                // Map NodeIds to index in node_ids array for JS rendering
                node_ids.iter().position(|&n| n == src).unwrap_or(0),
                node_ids.iter().position(|&n| n == tgt).unwrap_or(0)
            ));
        }
    }
    json.push_str("]}");
    json
}

// --- Config parsing ---

struct DemoConfig {
    seed: u64,
    mode: String,
    max_steps: usize,
    strategy: String,
    lock_key: bool,
    reachability: bool,
    acyclic: bool,
}

fn parse_config(json: &str) -> DemoConfig {
    // Simple manual JSON parsing (no serde dependency)
    let get_str = |key: &str| -> String {
        if let Some(pos) = json.find(&format!("\"{}\"", key)) {
            let rest = &json[pos + key.len() + 3..]; // skip "key":
            if let Some(start) = rest.find('"') {
                let inner = &rest[start + 1..];
                if let Some(end) = inner.find('"') {
                    return inner[..end].to_string();
                }
            }
        }
        String::new()
    };

    let get_num = |key: &str| -> u64 {
        if let Some(pos) = json.find(&format!("\"{}\"", key)) {
            let rest = &json[pos + key.len() + 3..];
            let num_str: String = rest.chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            num_str.parse().unwrap_or(0)
        } else {
            0
        }
    };

    let get_bool = |key: &str| -> bool {
        if let Some(pos) = json.find(&format!("\"{}\"", key)) {
            let rest = &json[pos + key.len() + 3..];
            rest.trim_start().starts_with("true")
        } else {
            true // default to true for constraints
        }
    };

    DemoConfig {
        seed: get_num("seed"),
        mode: get_str("mode"),
        max_steps: get_num("max_steps") as usize,
        strategy: get_str("strategy"),
        lock_key: get_bool("lock_key"),
        reachability: get_bool("reachability"),
        acyclic: get_bool("acyclic"),
    }
}

fn selection_from_str(s: &str) -> SelectionStrategy {
    match s {
        "first" => SelectionStrategy::FirstMatch,
        "parallel" => SelectionStrategy::Parallel,
        _ => SelectionStrategy::WeightedRandom,
    }
}

// --- WASM exports ---

/// Generate a dungeon graph with the given seed and max_steps. Returns graph JSON.
/// Convenience wrapper matching the spec signature.
#[wasm_bindgen]
pub fn generate_dungeon(seed: u32, max_steps: u32) -> String {
    let config = format!(
        "{{\"seed\":{},\"mode\":\"dungeon\",\"max_steps\":{},\"strategy\":\"weighted\",\"lock_key\":true,\"reachability\":true,\"acyclic\":false}}",
        seed, max_steps
    );
    generate_demo(&config)
}

/// Generate a complete dungeon/quest graph and return JSON.
#[wasm_bindgen]
pub fn generate_demo(config_json: &str) -> String {
    let cfg = parse_config(config_json);
    let max_steps = if cfg.max_steps > 0 { cfg.max_steps } else { 100 };

    let mut g = Graph::new();
    let start_node = if cfg.mode == "quest" {
        g.add_node(DNode::Objective)
    } else {
        g.add_node(DNode::Start)
    };

    let config = RewriterConfig::new()
        .max_steps(max_steps)
        .selection(selection_from_str(&cfg.strategy));

    let mut rewriter = rewriter::Rewriter::new(g, config);

    let rules = if cfg.mode == "quest" {
        quest_rules()
    } else {
        dungeon_rules()
    };
    for rule in rules {
        rewriter.add_rule(rule);
    }

    // Add constraints
    if cfg.mode != "quest" {
        if cfg.reachability {
            rewriter.add_constraint(ReachabilityConstraint::new(
                start_node,
                vec![DKind::Exit],
            ));
        }
        if cfg.lock_key {
            rewriter.add_constraint(LockKeyConstraint::new(
                start_node,
                vec![(DKind::Key, DKind::Lock)],
            ));
        }
    }
    if cfg.acyclic {
        rewriter.add_constraint(AcyclicConstraint);
    }

    let mut rng = rand::rngs::SmallRng::seed_from_u64(cfg.seed);
    match rewriter.rewrite(&mut rng) {
        Ok(result) => graph_to_json(&result.graph, 800.0, 600.0),
        Err(_) => r#"{"nodes":[],"edges":[]}"#.to_string(),
    }
}

/// Step-by-step rewriter for interactive demo.
#[wasm_bindgen]
pub struct StepRewriter {
    inner: rewriter::Rewriter<DNode, DEdge>,
    rng: rand::rngs::SmallRng,
    steps: usize,
    rules_applied: usize,
    constraint_violations: usize,
    done: bool,
}

#[wasm_bindgen]
impl StepRewriter {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Self {
        let cfg = parse_config(config_json);

        let mut g = Graph::new();
        let start_node = if cfg.mode == "quest" {
            g.add_node(DNode::Objective)
        } else {
            g.add_node(DNode::Start)
        };

        let max_steps = if cfg.max_steps > 0 { cfg.max_steps } else { 100 };
        let config = RewriterConfig::new()
            .max_steps(max_steps)
            .selection(selection_from_str(&cfg.strategy));

        let mut inner = rewriter::Rewriter::new(g, config);

        let rules = if cfg.mode == "quest" {
            quest_rules()
        } else {
            dungeon_rules()
        };
        for rule in rules {
            inner.add_rule(rule);
        }

        if cfg.mode != "quest" {
            if cfg.reachability {
                inner.add_constraint(ReachabilityConstraint::new(
                    start_node,
                    vec![DKind::Start],
                ));
            }
            if cfg.lock_key {
                inner.add_constraint(LockKeyConstraint::new(
                    start_node,
                    vec![(DKind::Key, DKind::Lock)],
                ));
            }
        }
        if cfg.acyclic {
            inner.add_constraint(AcyclicConstraint);
        }

        let seed = if cfg.seed > 0 { cfg.seed } else { 42 };
        let rng = rand::rngs::SmallRng::seed_from_u64(seed);

        Self {
            inner,
            rng,
            steps: 0,
            rules_applied: 0,
            constraint_violations: 0,
            done: false,
        }
    }

    /// Perform one rewrite step. Returns JSON event.
    pub fn step(&mut self) -> String {
        if self.done {
            let g = self.inner.graph();
            return format!(
                "{{\"type\":\"complete\",\"nodes\":{},\"edges\":{},\"steps\":{}}}",
                g.node_count(), g.edge_count(), self.steps
            );
        }

        // Capture node/edge IDs before step
        let nodes_before: std::collections::HashSet<usize> =
            self.inner.graph().node_ids().map(|n| n.0).collect();
        let edges_before: std::collections::HashSet<usize> =
            self.inner.graph().edge_ids().map(|e| e.0).collect();

        self.steps += 1;

        // Use observer to track what happens
        let mut obs = StepObserver::default();
        let result = self.inner.step_observed(&mut self.rng, &mut obs);

        match result {
            StepResult::Applied { rule_name, .. } => {
                if obs.constraint_violated {
                    self.constraint_violations += 1;
                    format!(
                        "{{\"type\":\"constraint_violated\",\"rule\":\"{}\"}}",
                        rule_name
                    )
                } else {
                    self.rules_applied += 1;
                    let g = self.inner.graph();

                    // Compute added node/edge IDs
                    let nodes_added: Vec<usize> = g.node_ids()
                        .map(|n| n.0)
                        .filter(|id| !nodes_before.contains(id))
                        .collect();
                    let edges_added: Vec<usize> = g.edge_ids()
                        .map(|e| e.0)
                        .filter(|id| !edges_before.contains(id))
                        .collect();

                    // Format arrays as JSON
                    let nodes_json: String = format!("[{}]",
                        nodes_added.iter().map(|id| format!("{}", id)).collect::<Vec<_>>().join(","));
                    let edges_json: String = format!("[{}]",
                        edges_added.iter().map(|id| format!("{}", id)).collect::<Vec<_>>().join(","));

                    format!(
                        "{{\"type\":\"applied\",\"rule\":\"{}\",\"nodes_added\":{},\"edges_added\":{}}}",
                        rule_name, nodes_json, edges_json
                    )
                }
            }
            StepResult::NoMatch => {
                self.done = true;
                let g = self.inner.graph();
                format!(
                    "{{\"type\":\"no_match\",\"nodes\":{},\"edges\":{},\"steps\":{}}}",
                    g.node_count(), g.edge_count(), self.steps
                )
            }
        }
    }

    /// Get the current graph as JSON (nodes with x/y + edges).
    pub fn graph_json(&self) -> String {
        graph_to_json(self.inner.graph(), 800.0, 600.0)
    }

    /// Get statistics as JSON.
    pub fn stats_json(&self) -> String {
        let g = self.inner.graph();
        format!(
            "{{\"nodes\":{},\"edges\":{},\"steps\":{},\"rules_applied\":{},\"constraint_violations\":{}}}",
            g.node_count(), g.edge_count(), self.steps, self.rules_applied, self.constraint_violations
        )
    }
}

// Simple observer for step tracking
#[derive(Default)]
struct StepObserver {
    constraint_violated: bool,
}

impl observer::RewriteObserver<DNode, DEdge> for StepObserver {
    fn on_constraint_violated(&mut self, _rule_index: usize) {
        self.constraint_violated = true;
    }
}
