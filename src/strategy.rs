//! Rule selection and application strategies.

use alloc::vec::Vec;
use crate::pattern::Match;
use crate::graph::NodeId;
use rand_core::RngCore;

/// Select a match from a set of candidates using weighted random selection.
pub fn select_match(
    candidates: &[(usize, Match, u32)], // (rule_index, match, weight)
    rng: &mut impl RngCore,
) -> Option<(usize, Match)> {
    if candidates.is_empty() {
        return None;
    }

    let total: u64 = candidates.iter().map(|(_, _, w)| *w as u64).sum();
    if total == 0 {
        return None;
    }

    let mut pick = rng.next_u64() % total;
    for (rule_idx, m, w) in candidates {
        let w64 = *w as u64;
        if pick < w64 {
            return Some((*rule_idx, m.clone()));
        }
        pick -= w64;
    }

    // Fallback (shouldn't reach here)
    let (rule_idx, m, _) = &candidates[candidates.len() - 1];
    Some((*rule_idx, m.clone()))
}

/// Find all non-overlapping matches for parallel application.
/// Greedy: iterate candidates, skip any that share nodes with already-selected matches.
pub fn find_non_overlapping(
    candidates: &[(usize, Match)],
) -> Vec<(usize, Match)> {
    let mut selected = Vec::new();
    let mut used_nodes: Vec<NodeId> = Vec::new();

    for (rule_idx, m) in candidates {
        let overlaps = m.node_map.iter().any(|n| used_nodes.contains(n));
        if !overlaps {
            used_nodes.extend_from_slice(&m.node_map);
            selected.push((*rule_idx, m.clone()));
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::graph::NodeId;
    use crate::pattern::Match;

    struct FakeRng(u64);
    impl RngCore for FakeRng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }
        fn next_u64(&mut self) -> u64 {
            // Simple LCG
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            self.0
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(8) {
                let val = self.next_u64();
                let bytes = val.to_le_bytes();
                let len = chunk.len().min(8);
                chunk[..len].copy_from_slice(&bytes[..len]);
            }
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    #[test]
    fn weighted_selection_distribution() {
        // Weights [1, 1, 8] => third option ~80%
        let m0 = Match { node_map: std::vec![NodeId(0)] };
        let m1 = Match { node_map: std::vec![NodeId(1)] };
        let m2 = Match { node_map: std::vec![NodeId(2)] };
        let candidates = std::vec![
            (0, m0, 1u32),
            (1, m1, 1u32),
            (2, m2, 8u32),
        ];

        let mut rng = FakeRng(12345);
        let mut counts = [0u32; 3];
        let trials = 1000;

        for _ in 0..trials {
            if let Some((idx, _)) = select_match(&candidates, &mut rng) {
                counts[idx] += 1;
            }
        }

        let third_pct = counts[2] as f64 / trials as f64;
        assert!(
            third_pct > 0.70 && third_pct < 0.90,
            "Third option should be ~80%, got {:.1}%",
            third_pct * 100.0
        );
    }

    #[test]
    fn non_overlapping_filters_shared_nodes() {
        // 3 matches all sharing node 0
        let candidates = std::vec![
            (0, Match { node_map: std::vec![NodeId(0), NodeId(1)] }),
            (1, Match { node_map: std::vec![NodeId(0), NodeId(2)] }),
            (2, Match { node_map: std::vec![NodeId(0), NodeId(3)] }),
        ];

        let result = find_non_overlapping(&candidates);
        assert_eq!(result.len(), 1, "Only first match should be selected");
        assert_eq!(result[0].0, 0);
    }

    #[test]
    fn non_overlapping_keeps_independent() {
        // 3 matches with no shared nodes
        let candidates = std::vec![
            (0, Match { node_map: std::vec![NodeId(0), NodeId(1)] }),
            (1, Match { node_map: std::vec![NodeId(2), NodeId(3)] }),
            (2, Match { node_map: std::vec![NodeId(4), NodeId(5)] }),
        ];

        let result = find_non_overlapping(&candidates);
        assert_eq!(result.len(), 3, "All 3 non-overlapping matches should be kept");
    }
}
