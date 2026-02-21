//! Error types for graph rewriting.

use core::fmt;

/// Errors that can occur during rewriting.
#[derive(Debug, Clone)]
pub enum RewriteError {
    /// Maximum steps exceeded without all rules being exhausted.
    MaxStepsExceeded { steps: usize },
    /// A constraint was violated and could not be resolved.
    ConstraintViolation { rule_index: usize },
    /// The graph is in an invalid state.
    InvalidGraph { message: &'static str },
}

impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxStepsExceeded { steps } => write!(f, "max steps exceeded after {steps} steps"),
            Self::ConstraintViolation { rule_index } => write!(f, "constraint violated by rule {rule_index}"),
            Self::InvalidGraph { message } => write!(f, "invalid graph: {message}"),
        }
    }
}
