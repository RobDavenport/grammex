//! Rewriter configuration.

/// How to select among matching rules when multiple rules match.
#[derive(Debug, Clone, Copy)]
pub enum SelectionStrategy {
    /// Apply the first matching rule (ordered by rule index).
    FirstMatch,
    /// Weighted random selection among all matching rules.
    WeightedRandom,
    /// Apply all non-overlapping matches simultaneously (Markov Junior style).
    Parallel,
}

impl Default for SelectionStrategy {
    fn default() -> Self {
        Self::WeightedRandom
    }
}

/// How to handle multiple matches of the same rule.
#[derive(Debug, Clone, Copy)]
pub enum ApplicationMode {
    /// Pick one match (using SelectionStrategy).
    Single,
    /// Apply to all non-overlapping matches.
    AllNonOverlapping,
}

impl Default for ApplicationMode {
    fn default() -> Self {
        Self::Single
    }
}

/// Configuration for the rewriter.
#[derive(Debug, Clone)]
pub struct RewriterConfig {
    /// Maximum rule applications before stopping.
    pub max_steps: usize,
    /// Rule selection strategy.
    pub selection: SelectionStrategy,
    /// Application mode.
    pub application: ApplicationMode,
}

impl Default for RewriterConfig {
    fn default() -> Self {
        Self {
            max_steps: 10_000,
            selection: SelectionStrategy::default(),
            application: ApplicationMode::default(),
        }
    }
}

impl RewriterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_steps(mut self, n: usize) -> Self {
        self.max_steps = n;
        self
    }

    pub fn selection(mut self, s: SelectionStrategy) -> Self {
        self.selection = s;
        self
    }

    pub fn application(mut self, a: ApplicationMode) -> Self {
        self.application = a;
        self
    }
}
