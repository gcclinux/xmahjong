//! Property-based tests for score calculation.

use proptest::prelude::*;

use xmahjong::game_state::ScoreTracker;

// Feature: xmahjong, Property 16: Score Calculation
//
// Score starts at 0 and increases with each pair matched.
// Formula:
//   base = pairs_matched * 10
//   streak = pairs_matched * 2
//   penalty = hints_used * 5 + shuffles_used * 10
//   subtotal = max(0, base + streak - penalty)
//   time_bonus = max(0, 500 - elapsed_seconds) [only at game end when elapsed_seconds > 0]
//   final_score = subtotal + time_bonus
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_16_score_calculation(
        elapsed_seconds in 0u32..2000,
        hints_used in 0u32..100,
        shuffles_used in 0u32..100,
        pairs_matched in 0u32..73,
    ) {
        let tracker = ScoreTracker {
            hints_used,
            shuffles_used,
            undos_used: 0,
            elapsed_seconds,
            pairs_matched,
            mismatches: 0,
        };

        let score = tracker.calculate_score();

        // Independently compute expected score using the formula
        let base = pairs_matched as u64 * 10;
        let streak = pairs_matched as u64 * 2;
        let penalty = hints_used as u64 * 5 + shuffles_used as u64 * 10;
        let subtotal = (base + streak).saturating_sub(penalty) as u32;

        let time_bonus = if elapsed_seconds > 0 {
            500u32.saturating_sub(elapsed_seconds)
        } else {
            0
        };

        let expected = subtotal + time_bonus;

        prop_assert_eq!(
            score, expected,
            "Score mismatch for pairs_matched={}, elapsed_seconds={}, hints={}, shuffles={}: got {}, expected {}",
            pairs_matched, elapsed_seconds, hints_used, shuffles_used, score, expected
        );

        // Score is never negative (u32 type guarantees this)
        // Score starts at 0 and grows with pairs
        if pairs_matched == 0 && elapsed_seconds == 0 {
            prop_assert_eq!(score, 0, "Score should be 0 with no pairs matched during gameplay");
        }
    }

    #[test]
    fn property_16_live_score_increases_with_pairs(
        pairs_matched in 0u32..73,
        hints_used in 0u32..10,
        shuffles_used in 0u32..5,
    ) {
        let tracker = ScoreTracker {
            hints_used,
            shuffles_used,
            undos_used: 0,
            elapsed_seconds: 0,
            pairs_matched,
            mismatches: 0,
        };

        let live = tracker.live_score();

        // Live score should equal base + streak - penalty (no time bonus)
        let base = pairs_matched as u64 * 10;
        let streak = pairs_matched as u64 * 2;
        let penalty = hints_used as u64 * 5 + shuffles_used as u64 * 10;
        let expected = (base + streak).saturating_sub(penalty) as u32;

        prop_assert_eq!(live, expected);
    }
}
