//! Core graph rewriting engine.

use alloc::vec::Vec;
use crate::graph::Graph;
use crate::node::NodeData;
use crate::edge::EdgeData;
use crate::rule::Rule;
use crate::constraint::StructuralConstraint;
use crate::config::RewriterConfig;
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
    pub fn step(&mut self, _rng: &mut impl RngCore) -> StepResult {
        todo!()
    }

    /// Perform one rewrite step with observer callbacks.
    pub fn step_observed(&mut self, _rng: &mut impl RngCore, _observer: &mut impl RewriteObserver<N, E>) -> StepResult {
        todo!()
    }

    /// Run rewriting to completion (until no rules match or max_steps reached).
    pub fn rewrite(self, _rng: &mut impl RngCore) -> Result<RewriteResult<N, E>, RewriteError> {
        todo!()
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
