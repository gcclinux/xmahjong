//! Property-based tests for score calculation.

use proptest::prelude::*;

use lmahjong::game_state::ScoreTracker;

// Feature: lmahjong, Property 16: Score Calculation
//
// **Validates: Requirements 8.2**
//
// For any combination of elapsed_seconds, hints_used, and shuffles_used,
// the score matches the formula max(0, 1000 - elapsed_seconds - hints_used * 50 - shuffles_used * 100)
// and is never negative.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_16_score_calculation(
        elapsed_seconds in 0u32..10000,
        hints_used in 0u32..100,
        shuffles_used in 0u32..100,
    ) {
        let tracker = ScoreTracker {
            hints_used,
            shuffles_used,
            elapsed_seconds,
        };

        let score = tracker.calculate_score();

        // Independently compute expected score using the formula
        let penalty = elapsed_seconds as u64 + hints_used as u64 * 50 + shuffles_used as u64 * 100;
        let expected = if penalty >= 1000 { 0u32 } else { 1000 - penalty as u32 };

        // Assert score matches the formula
        prop_assert_eq!(
            score, expected,
            "Score mismatch for elapsed_seconds={}, hints_used={}, shuffles_used={}: got {}, expected {}",
            elapsed_seconds, hints_used, shuffles_used, score, expected
        );

        // Assert score is never negative (u32 guarantees this at type level,
        // but we verify the implementation doesn't panic or wrap)
        prop_assert!(score <= 1000, "Score {} exceeds maximum of 1000", score);
    }
}
