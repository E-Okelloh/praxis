//! Basic test-case shrinking — instruction-list truncation.
//! Full proptest integration arrives in Week 5.
use solana_sdk::transaction::Transaction;

/// Shrink a failing transaction sequence to the shortest subsequence that
/// still satisfies `predicate`.
///
/// Strategy: binary-search-style deletion. Try removing each suffix first
/// (cheapest check), then each element in turn. Returns the shortest
/// passing subsequence found, or the original if nothing can be removed.
///
/// `seed` is reserved for future use with randomised shrinking strategies.
pub fn shrink_sequence(
    mut seq: Vec<Transaction>,
    _seed: u64,
    predicate: impl Fn(&[Transaction]) -> bool,
) -> Vec<Transaction> {
    if seq.is_empty() || !predicate(&seq) {
        return seq;
    }

    // Phase 1: try progressively shorter prefixes.
    let mut lo = 1usize;
    let mut hi = seq.len();
    // Binary search for the shortest prefix that still triggers the predicate.
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if predicate(&seq[..mid]) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    seq.truncate(lo);

    // Phase 2: try removing single elements from what remains.
    let mut i = 0;
    while i < seq.len() {
        let candidate: Vec<Transaction> = seq[..i]
            .iter()
            .chain(seq[i + 1..].iter())
            .cloned()
            .collect();
        if !candidate.is_empty() && predicate(&candidate) {
            seq = candidate;
            // Don't advance i — the element at position i is now the old i+1.
        } else {
            i += 1;
        }
    }

    seq
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::transaction::Transaction;

    fn dummy_txs(n: usize) -> Vec<Transaction> {
        (0..n).map(|_| Transaction::default()).collect()
    }

    #[test]
    fn shrinks_to_minimum_prefix() {
        // Predicate: fails when sequence has ≥ 3 elements.
        let txs = dummy_txs(10);
        let shrunk = shrink_sequence(txs, 0, |s| s.len() >= 3);
        assert_eq!(shrunk.len(), 3, "should shrink to exactly 3");
    }

    #[test]
    fn empty_sequence_is_returned_unchanged() {
        let shrunk = shrink_sequence(vec![], 0, |_| true);
        assert!(shrunk.is_empty());
    }

    #[test]
    fn non_triggering_sequence_returned_unchanged() {
        let txs = dummy_txs(5);
        // Predicate never satisfied.
        let shrunk = shrink_sequence(txs.clone(), 0, |_| false);
        assert_eq!(shrunk.len(), txs.len());
    }

    #[test]
    fn single_element_cannot_shrink_further() {
        let txs = dummy_txs(1);
        let shrunk = shrink_sequence(txs, 0, |s| !s.is_empty());
        assert_eq!(shrunk.len(), 1);
    }
}
