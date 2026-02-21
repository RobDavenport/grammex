//! A generic graph rewriting engine with game-constraint semantics.
//!
//! `grammex` provides graph grammar-based procedural generation with constraints
//! verified at rewrite time. Define graph transformation rules and let the engine
//! generate levels, quests, narratives, and puzzles with structural guarantees.
//!
//! # Features
//!
//! - **Graph rewriting**: Define pattern->replacement rules for procedural generation
//! - **Game constraints**: Lock-and-key, reachability, cycle constraints built-in
//! - **Multiple strategies**: FirstMatch, WeightedRandom, Parallel (Markov Junior-style)
//! - **Observable**: Monitor rewriting progress via the [`RewriteObserver`] trait
//! - **`no_std` compatible**: Works in embedded and WASM environments
//!
//! # Quick Start
//!
//! 1. Define node/edge types implementing [`NodeData`] and [`EdgeData`]
//! 2. Build [`Rule`]s with patterns and replacements
//! 3. Create a [`Rewriter`] with initial graph and config
//! 4. Call [`rewrite`](Rewriter::rewrite) or step through with [`step`](Rewriter::step)

#![no_std]

extern crate alloc;

pub mod graph;
pub mod node;
pub mod edge;
pub mod pattern;
pub mod rule;
pub mod rewriter;
pub mod constraint;
pub mod config;
pub mod observer;
pub mod error;
pub mod strategy;

// Re-export primary API
pub use graph::{Graph, NodeId, EdgeId};
pub use node::NodeData;
pub use edge::EdgeData;
pub use pattern::{Pattern, PatternNode, PatternEdge};
pub use rule::{Rule, Replacement, ReplacementNode, ReplacementAction, Reconnection};
pub use rewriter::{Rewriter, StepResult, RewriteResult};
pub use constraint::{StructuralConstraint, LockKeyConstraint, ReachabilityConstraint, CycleConstraint, AcyclicConstraint};
pub use config::{RewriterConfig, SelectionStrategy, ApplicationMode};
pub use observer::{RewriteObserver, NoOpRewriteObserver};
pub use error::RewriteError;
