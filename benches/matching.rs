use criterion::{criterion_group, criterion_main, Criterion};
use grammex::*;
use rand::SeedableRng as _;

#[derive(Clone, PartialEq, Eq, Debug)]
struct BNode(u8);

impl NodeData for BNode {
    type Kind = u8;
    fn kind(&self) -> u8 { self.0 }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct BEdge(u8);

impl EdgeData for BEdge {
    type Kind = u8;
    fn kind(&self) -> u8 { self.0 }
}

fn build_random_graph(node_count: usize, edge_density: f64, seed: u64) -> Graph<BNode, BEdge> {
    use rand::Rng;
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
    let mut g = Graph::new();

    let kinds = [1u8, 2, 3, 4, 5];
    let mut ids = Vec::new();
    for _ in 0..node_count {
        let kind = kinds[rng.gen_range(0..kinds.len())];
        ids.push(g.add_node(BNode(kind)));
    }

    let edge_count = (node_count as f64 * edge_density) as usize;
    let edge_kinds = [10u8, 20, 30];
    for _ in 0..edge_count {
        let s = rng.gen_range(0..node_count);
        let mut t = rng.gen_range(0..node_count);
        if t == s {
            t = (t + 1) % node_count;
        }
        let ek = edge_kinds[rng.gen_range(0..edge_kinds.len())];
        g.add_edge(ids[s], ids[t], BEdge(ek));
    }

    g
}

fn bench_2node_100graph(c: &mut Criterion) {
    let g = build_random_graph(100, 2.0, 42);
    let mut p = Pattern::new();
    let a = p.add_node(1u8);
    let b = p.add_node(2u8);
    p.add_edge(a, b, 10u8);

    c.bench_function("2-node pattern in 100-node graph", |bench| {
        bench.iter(|| {
            pattern::find_matches(&g, &p)
        })
    });
}

fn bench_3node_500graph(c: &mut Criterion) {
    let g = build_random_graph(500, 2.0, 42);
    let mut p = Pattern::new();
    let a = p.add_node(1u8);
    let b = p.add_node(2u8);
    let cc = p.add_node(3u8);
    p.add_edge(a, b, 10u8);
    p.add_edge(b, cc, 10u8);

    c.bench_function("3-node pattern in 500-node graph", |bench| {
        bench.iter(|| {
            pattern::find_matches(&g, &p)
        })
    });
}

fn dungeon_expand_rule() -> Rule<BNode, BEdge> {
    let mut lhs = Pattern::new();
    lhs.add_node(1u8);

    let rhs = rule::Replacement {
        nodes: vec![
            rule::ReplacementNode {
                from_lhs: Some(pattern::LocalId(0)),
                action: rule::ReplacementAction::Keep,
                data: None,
            },
            rule::ReplacementNode {
                from_lhs: None,
                action: rule::ReplacementAction::Keep,
                data: Some(BNode(1)),
            },
        ],
        edges: vec![
            (pattern::LocalId(0), pattern::LocalId(1), BEdge(10)),
        ],
        reconnections: vec![
            rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
        ],
    };

    Rule { lhs, rhs, weight: 1, name: "expand" }
}

fn bench_full_dungeon_50_steps(c: &mut Criterion) {
    c.bench_function("10-rule dungeon grammar, 50 steps", |bench| {
        bench.iter(|| {
            let mut g = Graph::new();
            g.add_node(BNode(1));

            let config = RewriterConfig::new()
                .max_steps(50)
                .selection(SelectionStrategy::WeightedRandom);

            let mut rewriter = rewriter::Rewriter::new(g, config);
            for _ in 0..10 {
                rewriter.add_rule(dungeon_expand_rule());
            }

            let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
            rewriter.rewrite(&mut rng).unwrap()
        })
    });
}

fn bench_parallel_grid(c: &mut Criterion) {
    c.bench_function("parallel rewriting 20x20 grid", |bench| {
        bench.iter(|| {
            let mut g = Graph::new();
            let mut ids = Vec::new();

            for row in 0..20usize {
                for col in 0..20usize {
                    let kind = if (row + col) % 2 == 0 { 1u8 } else { 2 };
                    ids.push(g.add_node(BNode(kind)));
                }
            }
            for row in 0..20 {
                for col in 0..19 {
                    let src = row * 20 + col;
                    let tgt = row * 20 + col + 1;
                    g.add_edge(ids[src], ids[tgt], BEdge(10));
                }
            }

            let mut lhs = Pattern::new();
            let a = lhs.add_node(1u8);
            let b = lhs.add_node(2u8);
            lhs.add_edge(a, b, 10u8);

            let rhs = rule::Replacement {
                nodes: vec![
                    rule::ReplacementNode {
                        from_lhs: Some(pattern::LocalId(0)),
                        action: rule::ReplacementAction::Keep,
                        data: None,
                    },
                    rule::ReplacementNode {
                        from_lhs: Some(pattern::LocalId(1)),
                        action: rule::ReplacementAction::Keep,
                        data: None,
                    },
                    rule::ReplacementNode {
                        from_lhs: None,
                        action: rule::ReplacementAction::Keep,
                        data: Some(BNode(3)),
                    },
                ],
                edges: vec![
                    (pattern::LocalId(0), pattern::LocalId(2), BEdge(10)),
                    (pattern::LocalId(2), pattern::LocalId(1), BEdge(10)),
                ],
                reconnections: vec![
                    rule::Reconnection { from: pattern::LocalId(0), to: pattern::LocalId(0) },
                    rule::Reconnection { from: pattern::LocalId(1), to: pattern::LocalId(1) },
                ],
            };

            let rule = Rule { lhs, rhs, weight: 1, name: "insert" };

            let config = RewriterConfig::new()
                .max_steps(1)
                .selection(SelectionStrategy::Parallel);

            let mut rewriter = rewriter::Rewriter::new(g, config);
            rewriter.add_rule(rule);

            let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
            rewriter.rewrite(&mut rng).unwrap()
        })
    });
}

criterion_group!(benches, bench_2node_100graph, bench_3node_500graph, bench_full_dungeon_50_steps, bench_parallel_grid);
criterion_main!(benches);
