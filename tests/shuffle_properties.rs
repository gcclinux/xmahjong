//! Property-based tests for shuffle state management.

use proptest::prelude::*;

use lmahjong::board::turtle_layout;
use lmahjong::game_state::{GameState, GameStatus, ScoreTracker};
use lmahjong::generator::BoardGenerator;
use lmahjong::logic::{select_tile, shuffle, SelectionResult, ShuffleError};
use lmahjong::timer::GameTimer;

/// Helper to create a GameState from a Board in Playing status.
fn make_state(board: lmahjong::board::Board) -> GameState {
    GameState {
        board,
        timer: GameTimer::new(),
        score: ScoreTracker::new(),
        status: GameStatus::Playing,
        selection: None,
        hint: None,
        undo_stack: Vec::new(),
        shuffles_remaining: 3,
        level: 1,
        animations: Vec::new(),
    }
}

// Feature: lmahjong, Property 14: Shuffle State Management
//
// **Validates: Requirements 6.4, 6.5**
//
// For any game session, the shuffle count SHALL start at 3, decrease by 1 on
// each shuffle, reject shuffles when the count reaches 0, and each shuffle
// SHALL clear the undo history completely.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_14_shuffle_state_management(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let (board, solution) = generator.generate_with_solution(layout, 5).unwrap();

        // Need at least 4 pairs from solution to build undo history multiple times
        prop_assume!(solution.len() >= 4);

        let mut state = make_state(board);

        // Assert initial shuffle count is 3
        prop_assert_eq!(state.shuffles_remaining, 3,
            "Shuffles remaining should start at 3");

        // --- Phase 1: Perform some matches to build undo history ---
        let (pos_a, pos_b) = solution[0];
        let r1 = select_tile(&mut state, pos_a);
        prop_assert_eq!(r1, SelectionResult::Selected);
        let r2 = select_tile(&mut state, pos_b);
        prop_assert_eq!(r2, SelectionResult::Matched(pos_a, pos_b));

        // Undo stack should be non-empty
        prop_assert!(!state.undo_stack.is_empty(),
            "Undo stack should have entries after a match");

        // --- Shuffle 1: succeeds, shuffles_remaining == 2, undo cleared ---
        let shuffle_result = shuffle(&mut state);
        prop_assert!(shuffle_result.is_ok(),
            "First shuffle should succeed, got {:?}", shuffle_result);
        prop_assert_eq!(state.shuffles_remaining, 2,
            "After first shuffle, shuffles_remaining should be 2");
        prop_assert!(state.undo_stack.is_empty(),
            "Undo stack should be cleared after first shuffle");

        // --- Phase 2: Build undo history again using valid pairs ---
        // After shuffle, board is rearranged. Find new valid pairs.
        let pairs_after_shuffle1 = state.board.valid_pairs();
        prop_assume!(!pairs_after_shuffle1.is_empty(),
            "Should have valid pairs after shuffle");

        let (pa, pb) = pairs_after_shuffle1[0];
        let r3 = select_tile(&mut state, pa);
        prop_assert_eq!(r3, SelectionResult::Selected,
            "Selection after shuffle1 should work, got {:?}", r3);
        let r4 = select_tile(&mut state, pb);
        prop_assert_eq!(r4, SelectionResult::Matched(pa, pb),
            "Match after shuffle1 should work, got {:?}", r4);

        prop_assert!(!state.undo_stack.is_empty(),
            "Undo stack should have entries after second match");

        // --- Shuffle 2: succeeds, shuffles_remaining == 1, undo cleared ---
        let shuffle_result2 = shuffle(&mut state);
        prop_assert!(shuffle_result2.is_ok(),
            "Second shuffle should succeed, got {:?}", shuffle_result2);
        prop_assert_eq!(state.shuffles_remaining, 1,
            "After second shuffle, shuffles_remaining should be 1");
        prop_assert!(state.undo_stack.is_empty(),
            "Undo stack should be cleared after second shuffle");

        // --- Shuffle 3: succeeds, shuffles_remaining == 0 ---
        let shuffle_result3 = shuffle(&mut state);
        prop_assert!(shuffle_result3.is_ok(),
            "Third shuffle should succeed, got {:?}", shuffle_result3);
        prop_assert_eq!(state.shuffles_remaining, 0,
            "After third shuffle, shuffles_remaining should be 0");
        prop_assert!(state.undo_stack.is_empty(),
            "Undo stack should be cleared after third shuffle");

        // --- Shuffle 4: rejected with NoShufflesRemaining ---
        let shuffle_result4 = shuffle(&mut state);
        prop_assert_eq!(shuffle_result4, Err(ShuffleError::NoShufflesRemaining));
    }
}


// Feature: lmahjong, Property 13: Shuffle Guarantees Valid Pair Exists
//
// **Validates: Requirements 6.2**
//
// For any successful shuffle operation, the resulting board state SHALL contain
// at least one valid matching pair among the free tiles.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_13_shuffle_guarantees_valid_pair_exists(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        let mut state = make_state(board);

        // Perform shuffle — assert it succeeds
        let result = shuffle(&mut state);
        prop_assert!(result.is_ok(),
            "Shuffle should succeed on a generated board, got {:?}", result);

        // After shuffle, at least one valid matching pair must exist among free tiles
        let valid_pairs = state.board.valid_pairs();
        prop_assert!(!valid_pairs.is_empty(),
            "After a successful shuffle, the board must have at least one valid matching pair, \
             but valid_pairs() returned empty. Free tiles: {:?}", state.board.free_tiles());
    }
}
