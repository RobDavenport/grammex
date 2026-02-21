# grammex

A generic graph rewriting engine with game-constraint semantics for procedural generation.

**[Live Demo](https://robdavenport.github.io/grammex/)**

## Features

- **Graph rewriting**: Define pattern->replacement rules for procedural generation
- **Game constraints**: Lock-and-key, reachability, cycle constraints built-in
- **Multiple strategies**: FirstMatch, WeightedRandom, Parallel (Markov Junior-style)
- **Observable**: Monitor rewriting progress via the `RewriteObserver` trait
- **`no_std` compatible**: Works in embedded and WASM environments
- **Deterministic**: Same seed produces same results

## Quick Start

```rust
use grammex::*;

// Define your node and edge types
#[derive(Clone, PartialEq, Eq)]
struct Room(RoomKind);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RoomKind { Start, Empty, Key, Lock, Boss }

impl NodeData for Room {
    type Kind = RoomKind;
    fn kind(&self) -> RoomKind { self.0 }
}

// Build a graph and rewriter
let mut graph = Graph::new();
let start = graph.add_node(Room(RoomKind::Start));

let config = RewriterConfig::new().max_steps(100);
let mut rewriter = Rewriter::new(graph, config);

// Add rules and constraints, then rewrite
let result = rewriter.rewrite(&mut rng)?;
```

## Use Cases

- Dungeon/level generation with guaranteed completability
- Quest/mission structure generation
- Dialogue tree generation with narrative constraints
- Puzzle design (constraint-satisfying key/lock/switch placement)
- Multi-phase pipelines: structure -> rooms -> detail

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
