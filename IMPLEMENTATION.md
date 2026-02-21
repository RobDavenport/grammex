# grammex Implementation Plan

## What This Is

A generic graph rewriting engine with game-constraint semantics for procedural generation. All module files are scaffolded with trait signatures and `todo!()` stubs. Your job: fill in the implementations, write tests, build the WASM demo.

The graph data structure (`src/graph.rs`) is **already fully implemented** with tests passing. Start from there.

## Hard Rules

- `#![no_std]` with `extern crate alloc` — no std dependency in core library
- `rand_core` is the ONLY dependency
- All tests: `cargo test --target x86_64-pc-windows-msvc`
- WASM check: `cargo build --target wasm32-unknown-unknown --release`
- Deterministic: same seed must produce same results
- Subgraph isomorphism patterns should be small (2-5 nodes) — optimize for that case

## Reference Implementation

The sibling library `wavfc` (at `../wavfc/`) uses similar patterns. Key files:
- `../wavfc/src/propagator.rs` — Constraint propagation (worklist-based)
- `../wavfc/src/solver.rs` — Step/solve loop with observer callbacks
- `../wavfc/src/constraint.rs` — Constraint trait pattern
- `../wavfc/demo-wasm/src/lib.rs` — WASM FFI bindings
- `../wavfc/demo-wasm/www/main.js` — Demo UI pattern

## Key Concepts

**Graph grammar rewriting** works like find-and-replace on graphs:
1. Define a **pattern** (left-hand side) — a small subgraph to find
2. Define a **replacement** (right-hand side) — what to replace it with
3. The **rewriter** finds matches of the pattern in the host graph and applies the replacement
4. **Constraints** are checked after each application — if violated, the application is rolled back

**VF2 algorithm** for subgraph isomorphism:
- Builds a mapping from pattern nodes to graph nodes incrementally
- At each step, tries to extend the mapping by one pair
- Prunes candidates using: node kind match, edge kind match, neighbor compatibility
- For small patterns (2-5 nodes), this is fast (milliseconds)

**Reconnection**: When a pattern match is replaced, edges connecting to the matched nodes from outside need to be redirected to the replacement nodes. This is handled by `Reconnection` specs.

## Implementation Steps

### Phase 1: Pattern Matching (the hard part)

**Step 1: Implement `find_matches` using VF2-lite** (`src/pattern.rs`)

This is the core algorithm. Implement a simplified VF2:

```
function find_matches(graph, pattern):
    matches = []
    vf2_search(graph, pattern, mapping=[], matches)
    return matches

function vf2_search(graph, pattern, mapping, matches):
    if mapping.len() == pattern.nodes.len():
        // Full match found — verify all pattern edges exist
        if all_edges_match(graph, pattern, mapping):
            matches.push(mapping.clone())
        return

    // Next unmapped pattern node
    p_node = pattern.nodes[mapping.len()]

    // Try each graph node as a candidate
    for g_node in graph.node_ids():
        if g_node already in mapping: continue
        if graph.node(g_node).kind() != p_node.kind: continue
        if not consistent(graph, pattern, mapping, p_node, g_node): continue

        mapping.push(g_node)
        vf2_search(graph, pattern, mapping, matches)
        mapping.pop()

function consistent(graph, pattern, mapping, p_node, g_node):
    // Check that all edges between already-mapped pattern nodes and p_node
    // have corresponding edges in the graph between their mapped counterparts and g_node
    for each mapped (p_idx, g_mapped):
        // Check pattern edges from p_idx to p_node.id
        for edge in pattern.edges where source==p_idx && target==p_node.id:
            if no edge from g_mapped to g_node with matching kind: return false
        // Check pattern edges from p_node.id to p_idx
        for edge in pattern.edges where source==p_node.id && target==p_idx:
            if no edge from g_node to g_mapped with matching kind: return false
    return true
```

Tests:
- Single node pattern in 5-node graph → finds all nodes of matching kind
- Two connected nodes → finds all matching edges
- Triangle pattern → finds triangles
- No match → returns empty

**Step 2: Implement rule application** (`src/rewriter.rs`, `src/rule.rs`)

After finding a match, apply the replacement:

```
function apply_rule(graph, rule, match):
    // 1. Record which graph nodes are in the match
    matched_nodes = match.node_map

    // 2. Create new nodes from replacement spec
    new_node_ids = []
    for r_node in rule.rhs.nodes:
        if r_node.from_lhs is Some(local_id):
            match r_node.action:
                Keep => new_node_ids.push(matched_nodes[local_id])
                Replace(data) => graph.node_mut(...).data = data; push id
                Remove => (mark for removal)
        else:
            // New node
            id = graph.add_node(r_node.data)
            new_node_ids.push(id)

    // 3. Add replacement edges
    for (src_local, tgt_local, edge_data) in rule.rhs.edges:
        graph.add_edge(new_node_ids[src], new_node_ids[tgt], edge_data)

    // 4. Handle reconnections: redirect external edges
    for reconnection in rule.rhs.reconnections:
        old_node = matched_nodes[reconnection.from]
        new_node = new_node_ids[reconnection.to]
        // Move all incoming/outgoing edges from old_node to new_node
        redirect_edges(graph, old_node, new_node)

    // 5. Remove old matched nodes that aren't kept
    for node_id in matched_nodes not in new_node_ids:
        graph.remove_node(node_id)
```

Tests:
- Simple expansion: single "Start" node → two "Room" nodes connected by "Corridor" edge
- Node replacement: "Empty" → "Key" (data change, edges preserved)
- Reconnection: replace middle node in A→B→C chain, verify A and C reconnect

**Step 3: Implement strategy selection** (`src/strategy.rs`)

- `select_match`: weighted random selection from candidates using RNG
  - Sum all weights, pick random number in [0, total), find corresponding candidate
- `find_non_overlapping`: greedy — iterate candidates, skip any that share nodes with already-selected matches

Tests:
- Weighted selection: with weights [1, 1, 8], the third option should be picked ~80% of the time over many trials
- Non-overlapping: 3 overlapping matches → only 1 selected

### Phase 2: Rewriter Engine

**Step 4: Implement `Rewriter.step()`** (`src/rewriter.rs`)

```
function step(rng):
    // 1. Find all matches for all rules
    candidates = []
    for (rule_idx, rule) in rules:
        matches = find_matches(graph, rule.lhs)
        for m in matches:
            candidates.push((rule_idx, m, rule.weight))

    if candidates.is_empty(): return StepResult::NoMatch

    // 2. Select based on strategy
    match config.selection:
        FirstMatch => pick candidates[0]
        WeightedRandom => select_match(candidates, rng)
        Parallel => find_non_overlapping, apply all

    // 3. Snapshot graph (for constraint rollback)
    snapshot = graph.clone()

    // 4. Apply rule(s)
    apply_rule(graph, rule, match)

    // 5. Check constraints
    for constraint in constraints:
        if !constraint.check(graph):
            graph = snapshot  // rollback
            return StepResult::Applied (with note about rollback)

    steps += 1
    return StepResult::Applied { rule_index, rule_name }
```

**Step 5: Implement `Rewriter.step_observed()`**
- Same as step() but call observer methods at each stage

**Step 6: Implement `Rewriter.rewrite()`**
- Loop calling step() until NoMatch or max_steps reached
- Return RewriteResult with final graph and stats

### Phase 3: Constraints

**Step 7: Implement ReachabilityConstraint** (`src/constraint.rs`)
- BFS/DFS from start node
- Check that all nodes of required_kinds are visited
- Implement `StructuralConstraint` for it

**Step 8: Implement CycleConstraint**
- Count independent cycles: `cycles = edges - nodes + connected_components`
- Check against expected_cycles

**Step 9: Implement AcyclicConstraint**
- DFS-based cycle detection (track visited + in-stack)

**Step 10: Implement LockKeyConstraint**
- This needs a way to identify lock/key pairs from node data
- Approach: require NodeData to implement a `lock_key_pair` method or use a closure
- BFS from start: for each lock node encountered, verify its corresponding key was already visited
- This is the most complex constraint — implement it carefully

Tests for all constraints:
- Reachability: graph with disconnected component → fails; fully connected → passes
- Cycle: tree graph → 0 cycles; graph with one loop → 1 cycle
- Acyclic: DAG → passes; graph with cycle → fails
- LockKey: key before lock on all paths → passes; lock without key → fails

### Phase 4: Integration Tests

**tests/dungeon_grammar.rs** — The killer demo test
```rust
// Define node kinds: Start, Room, Corridor, Key, Lock, Boss, Exit
// Define edge kinds: Connection, Contains

// Rule 1: Start → Start -[Connection]→ Room
// Rule 2: Room → Room -[Connection]→ Room (expansion)
// Rule 3: Room → Room -[Connection]→ Key -[Connection]→ Lock -[Connection]→ Room (lock-and-key)
// Rule 4: Room → Boss (terminal — replace a Room with Boss, only once)
// Rule 5: Boss → Boss -[Connection]→ Exit (add exit after boss)

// Add ReachabilityConstraint (Exit reachable from Start)
// Add LockKeyConstraint (keys before locks)
// Run rewriter with max_steps=50
// Verify: Start and Exit exist, all locks have keys, Exit reachable from Start
// Run 100 times with different seeds, all should produce valid dungeons
```

**tests/quest_grammar.rs**
```rust
// Node kinds: Objective, SubObjective, Reward, Prerequisite
// Rules expand objectives into sub-objectives with prerequisites
// Verify acyclicity (no circular prerequisites)
```

**tests/parallel_rewriting.rs**
```rust
// Grid-like graph (nodes connected in 2D grid pattern)
// Rule: replace pattern A-B with pattern A-C-B (insert node)
// Use Parallel strategy
// Verify all non-overlapping matches applied simultaneously
```

**tests/determinism.rs**
```rust
// Same grammar + same seed → identical graph, 100 times
```

### Phase 5: Benchmarks

**benches/matching.rs** — Replace placeholder with:
- Pattern matching: 2-node pattern in 100-node graph
- Pattern matching: 3-node pattern in 500-node graph
- Full rewrite: 10-rule dungeon grammar, 50 steps
- Parallel rewriting: 20x20 grid graph, insertion rule

### Phase 6: WASM Demo

**demo-wasm/src/lib.rs** — Replace stubs with:
- `generate_dungeon(seed: u32, max_steps: u32) -> String` — Run dungeon grammar, return graph JSON
- `StepRewriter::new(grammar: &str, seed: u32)` — Initialize with built-in grammar
- `StepRewriter::step() -> String` — Returns JSON event:
  - `{"type":"applied","rule":"expand_room","nodes_added":[5,6],"edges_added":[3]}` — rule applied
  - `{"type":"constraint_violated","rule":"add_lock"}` — rolled back
  - `{"type":"complete","nodes":25,"edges":30,"steps":15}` — done
  - `{"type":"no_match"}` — no rules match
- `StepRewriter::graph_json() -> String` — Current graph state:
  ```json
  {
    "nodes": [{"id": 0, "kind": "Start", "x": 100, "y": 200}, ...],
    "edges": [{"source": 0, "target": 1, "kind": "Connection"}, ...]
  }
  ```
  Note: x/y positions should be computed via simple force-directed layout in Rust

**demo-wasm/www/main.js** — Replace placeholder with:
- Graph visualization on canvas:
  - Nodes as colored circles with labels
  - Color map: Start=green, Room=blue, Key=yellow, Lock=red, Boss=purple, Exit=white
  - Edges as lines with arrowheads
  - Force-directed layout (spring simulation) animated on canvas
- Controls:
  - "Generate" — full generation, show final graph
  - "Step" — one rewrite step, animate the change (flash matched pattern, then show replacement)
  - "Auto Play" — animate step-by-step with speed slider
  - "Reset" — clear to initial Start node
  - Grammar selector: Dungeon, Quest
  - Seed input
  - Constraint toggles: Lock-Key (checkbox), Reachability (checkbox)
- Stats: nodes, edges, rules applied, cycles detected, constraint violations

**demo-wasm/www/index.html** — Already has the layout, may need minor tweaks

### Phase 7: Verify Everything

```bash
# Core library
cargo test --target x86_64-pc-windows-msvc
cargo build --target wasm32-unknown-unknown --release
cargo bench

# WASM demo
wasm-pack build demo-wasm --target web --release
# Manually test in browser: open demo-wasm/www/index.html via local server
```

Commit and push. GitHub Actions will deploy to Pages automatically.

## VF2 Algorithm Reference

VF2 (Vento-Foggia) subgraph isomorphism algorithm, simplified for small patterns:

```
State: partial mapping M = {(pattern_node, graph_node), ...}

function vf2_match(graph, pattern, M, results):
    if |M| == |pattern.nodes|:
        results.push(M)
        return

    // Determine next pattern node to map (in order)
    p = pattern.nodes[|M|]

    // Generate candidate graph nodes
    for each g in graph.node_ids():
        if g is already mapped in M: skip
        if g.kind != p.kind: skip

        // Feasibility check: all edges between M and (p,g) must exist
        if not feasible(graph, pattern, M, p, g): skip

        M' = M + (p, g)
        vf2_match(graph, pattern, M', results)

function feasible(graph, pattern, M, p, g):
    for each (p_mapped, g_mapped) in M:
        // Forward edges: p_mapped -> p requires g_mapped -> g
        if pattern has edge (p_mapped -> p, kind=k):
            if graph has no edge (g_mapped -> g, kind=k): return false
        // Backward edges: p -> p_mapped requires g -> g_mapped
        if pattern has edge (p -> p_mapped, kind=k):
            if graph has no edge (g -> g_mapped, kind=k): return false
    return true
```

## Reconnection Logic Reference

When replacing matched subgraph with replacement:

```
1. External edges TO matched nodes:
   - For each edge (external → matched_node):
     - Find reconnection for matched_node
     - Redirect edge to reconnection target in replacement

2. External edges FROM matched nodes:
   - For each edge (matched_node → external):
     - Find reconnection for matched_node
     - Redirect edge from reconnection source in replacement

3. Edges BETWEEN matched nodes:
   - Removed (replaced by edges defined in replacement.edges)
```

## Dormans Dungeon Grammar Reference

Based on Joris Dormans' "Cyclic Dungeon Generation" paper:

1. Start with a single "Start" node
2. **Expansion rules**: Split rooms, add corridors, branch paths
3. **Lock-and-key rules**: Place keys and corresponding locks on separate paths
4. **Terminal rules**: Replace abstract rooms with concrete types (treasure, trap, boss)
5. **Constraint**: All locks must have their key reachable from Start without passing through the lock

This creates dungeons where the player must explore to find keys, unlock doors, and reach the boss — with guaranteed completability.
