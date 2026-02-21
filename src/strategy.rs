//! Rule selection and application strategies.

use alloc::vec::Vec;
use crate::pattern::Match;
use rand_core::RngCore;

/// Select a match from a set of candidates based on the strategy.
pub fn select_match(
    _candidates: &[(usize, Match, u32)], // (rule_index, match, weight)
    _rng: &mut impl RngCore,
) -> Option<(usize, Match)> {
    todo!()
}

/// Find all non-overlapping matches for parallel application.
pub fn find_non_overlapping(
    _candidates: &[(usize, Match)],
) -> Vec<(usize, Match)> {
    todo!()
}
