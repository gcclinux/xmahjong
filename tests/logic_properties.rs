//! Property-based tests for the Game Logic module.

use std::collections::HashSet;

use proptest::prelude::*;

use lmahjong::board::turtle_layout;
use lmahjong::game_state::{GameState, GameStatus, ScoreTracker};
use lmahjong::generator::BoardGenerator;
use lmahjong::logic::{check_game_over, request_hint, select_tile, undo, GameOverReason, HintResult, SelectionResult, UndoError};
use lmahjong::timer::GameTimer;

// Feature: lmahjong, Property 7: Re-Clicking Selected Tile Deselects
//
// **Validates: Requirements 2.6**
//
// For any board state where exactly one tile is selected, clicking that same
// tile again SHALL result in no tiles being selected.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_7_reclicking_selected_tile_deselects(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        // Find any free tile on the board
        let free_tiles = board.free_tiles();
        prop_assume!(!free_tiles.is_empty(), "No free tiles on generated board");

        let target_pos = free_tiles[0];

        // Snapshot the board tiles before any interaction
        let board_snapshot: Vec<_> = board.tiles.iter().cloned().collect();

        // Create a GameState with the generated board
        let mut state = GameState {
            board,
            timer: GameTimer::new(),
            score: ScoreTracker::new(),
            status: GameStatus::Playing,
            selection: None,
            hint: None,
            undo_stack: Vec::new(),
            shuffles_remaining: 3,
            level: 1,
            base_score: 0,
            base_time_ms: 0,
            base_hints: 0,
            base_shuffles: 0,
            base_undos: 0,
            animations: Vec::new(),
        };

        // Select the free tile
        let first_result = select_tile(&mut state, target_pos);
        prop_assert_eq!(first_result, SelectionResult::Selected,
            "First click on free tile should return Selected");
        prop_assert_eq!(state.selection, Some(target_pos),
            "Selection should be Some(target_pos) after first click");

        // Click the same tile again
        let second_result = select_tile(&mut state, target_pos);

        // Assert: returns Deselected
        prop_assert_eq!(second_result, SelectionResult::Deselected,
            "Second click on same tile should return Deselected");

        // Assert: selection is None
        prop_assert_eq!(state.selection, None,
            "Selection should be None after re-clicking selected tile");

        // Assert: no tiles removed — board unchanged
        let board_after: Vec<_> = state.board.tiles.iter().cloned().collect();
        prop_assert_eq!(board_snapshot.len(), board_after.len(),
            "Board size should not change");
        for (i, (before, after)) in board_snapshot.iter().zip(board_after.iter()).enumerate() {
            prop_assert_eq!(before, after,
                "Tile at position {} changed unexpectedly after deselection", i);
        }

        // Assert: undo stack unchanged (no tiles removed)
        prop_assert!(state.undo_stack.is_empty(),
            "Undo stack should remain empty — no match occurred");
    }
}


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
        base_score: 0,
        base_time_ms: 0,
        base_hints: 0,
        base_shuffles: 0,
        base_undos: 0,
        animations: Vec::new(),
    }
}

// Feature: lmahjong, Property 6: Invalid Match Deselects Both Tiles
//
// **Validates: Requirements 2.4**
//
// For any board state with a selected free tile, selecting a second free tile
// with a different face ID SHALL result in no tiles selected and no tiles
// removed from the board.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_6_invalid_match_deselects_both_tiles(seed in 0u64..10000) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        let free = board.free_tiles();

        // Find two free tiles with different face_ids
        let mut pair: Option<(usize, usize)> = None;
        'outer: for i in 0..free.len() {
            for j in (i + 1)..free.len() {
                let face_a = board.tiles[free[i]].unwrap().face_id;
                let face_b = board.tiles[free[j]].unwrap().face_id;
                if face_a != face_b {
                    pair = Some((free[i], free[j]));
                    break 'outer;
                }
            }
        }

        // Skip seeds where no mismatched pair exists among free tiles
        prop_assume!(pair.is_some());

        let (pos_a, pos_b) = pair.unwrap();

        let mut state = make_state(board);
        let remaining_before = state.board.remaining_count();

        // Select the first tile
        let result1 = select_tile(&mut state, pos_a);
        prop_assert_eq!(result1, SelectionResult::Selected,
            "First tile selection should return Selected, got {:?}", result1);

        // Select the second tile (different face_id) — should mismatch
        let result2 = select_tile(&mut state, pos_b);
        prop_assert_eq!(result2, SelectionResult::Mismatched(pos_a, pos_b),
            "Second tile selection should return Mismatched, got {:?}", result2);

        // Assert: both tiles still exist on the board (not removed)
        prop_assert!(state.board.tiles[pos_a].is_some(),
            "Tile at pos_a ({}) should still exist after mismatch", pos_a);
        prop_assert!(state.board.tiles[pos_b].is_some(),
            "Tile at pos_b ({}) should still exist after mismatch", pos_b);

        // Assert: no selection remains
        prop_assert_eq!(state.selection, None,
            "Selection should be None after mismatch, got {:?}", state.selection);

        // Assert: remaining_count unchanged
        let remaining_after = state.board.remaining_count();
        prop_assert_eq!(remaining_before, remaining_after,
            "Remaining count should be unchanged: before={}, after={}", remaining_before, remaining_after);
    }
}


// Feature: lmahjong, Property 5: Valid Match Removes Tiles and Updates Free Set
//
// **Validates: Requirements 2.3, 2.5**
//
// For any board state with a valid match, selecting the first tile and then
// the second tile of the matching pair SHALL remove both tiles from the board,
// and the resulting free tile set SHALL equal the set computed by a full
// recalculation over the updated board (i.e., checking is_free() for each
// remaining tile individually).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_5_valid_match_removes_tiles_and_updates_free_set(seed in 0u64..10_000) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        // Find a valid pair on the generated board
        let valid_pairs = board.valid_pairs();
        // Generated boards are solvable, so there must be at least one valid pair
        prop_assume!(!valid_pairs.is_empty());

        let (pos_a, pos_b) = valid_pairs[0];

        // Set up GameState and perform the match
        let mut state = make_state(board);
        let result_a = select_tile(&mut state, pos_a);
        prop_assert_eq!(result_a, SelectionResult::Selected,
            "First tile at position {} should be Selected, got {:?}", pos_a, result_a);

        let result_b = select_tile(&mut state, pos_b);
        prop_assert_eq!(result_b, SelectionResult::Matched(pos_a, pos_b),
            "Second tile at position {} should be Matched({}, {}), got {:?}", pos_b, pos_a, pos_b, result_b);

        // Assert both tiles are removed
        prop_assert!(state.board.tiles[pos_a].is_none(),
            "Tile at position {} should be removed after match", pos_a);
        prop_assert!(state.board.tiles[pos_b].is_none(),
            "Tile at position {} should be removed after match", pos_b);

        // Compute free tiles via board.free_tiles()
        let free_set: HashSet<usize> = state.board.free_tiles().into_iter().collect();

        // Independently compute what should be free by checking is_free() for each position
        let expected_free_set: HashSet<usize> = (0..state.board.tiles.len())
            .filter(|&pos| state.board.is_free(pos))
            .collect();

        // Assert the two sets match exactly
        prop_assert_eq!(&free_set, &expected_free_set,
            "free_tiles() result does not match individual is_free() checks. \
             In free_tiles() but not is_free(): {:?}. \
             In is_free() but not free_tiles(): {:?}.",
            free_set.difference(&expected_free_set).collect::<Vec<_>>(),
            expected_free_set.difference(&free_set).collect::<Vec<_>>());
    }
}


// Feature: lmahjong, Property 10: Undo Round-Trip Restores Full State
//
// **Validates: Requirements 5.1, 5.4**
//
// For any board state where a valid match has just been made, performing an undo
// SHALL restore the board to its exact previous state — including tile positions,
// free tile set, and the score contribution of that match being excluded.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_10_undo_round_trip_restores_full_state(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        // Find a valid pair on the board
        let valid_pairs = board.valid_pairs();
        prop_assume!(!valid_pairs.is_empty(), "No valid pairs on generated board");

        let (pos_a, pos_b) = valid_pairs[0];

        let mut state = make_state(board);

        // Snapshot the board tiles before the match
        let tiles_before: Vec<Option<lmahjong::board::Tile>> = state.board.tiles.clone();
        let free_before: HashSet<usize> = state.board.free_tiles().into_iter().collect();

        // Perform the match
        let r1 = select_tile(&mut state, pos_a);
        prop_assert_eq!(r1, SelectionResult::Selected);

        let r2 = select_tile(&mut state, pos_b);
        prop_assert_eq!(r2, SelectionResult::Matched(pos_a, pos_b));

        // Verify tiles were actually removed
        prop_assert!(state.board.tiles[pos_a].is_none(),
            "Tile at pos_a should be removed after match");
        prop_assert!(state.board.tiles[pos_b].is_none(),
            "Tile at pos_b should be removed after match");

        // Call undo
        let undo_result = undo(&mut state);
        prop_assert!(undo_result.is_ok(), "Undo should succeed after a match");

        // Assert the board tiles match the snapshot exactly
        let tiles_after: Vec<Option<lmahjong::board::Tile>> = state.board.tiles.clone();
        prop_assert_eq!(tiles_before.len(), tiles_after.len(),
            "Board size should not change after undo");
        for (i, (before, after)) in tiles_before.iter().zip(tiles_after.iter()).enumerate() {
            prop_assert_eq!(before, after,
                "Tile at position {} differs after undo: before={:?}, after={:?}", i, before, after);
        }

        // Assert free_tiles() matches what it was before the match
        let free_after: HashSet<usize> = state.board.free_tiles().into_iter().collect();
        prop_assert_eq!(&free_before, &free_after,
            "Free tiles after undo should match free tiles before the match. \
             In before but not after: {:?}. In after but not before: {:?}.",
            free_before.difference(&free_after).collect::<Vec<_>>(),
            free_after.difference(&free_before).collect::<Vec<_>>());

        // Assert undo stack is now empty (score contribution excluded)
        prop_assert!(state.undo_stack.is_empty(),
            "Undo stack should be empty after undoing the only match");
    }
}


// Feature: lmahjong, Property 11: Undo Capacity Limit
//
// **Validates: Requirements 5.2**
//
// For any sequence of N consecutive matches (where N ≤ 10), undoing all N
// SHALL succeed. Attempting an (N+1)th undo SHALL fail with EmptyStack.
// Additionally, when 11+ matches are performed, the undo stack caps at 10
// entries (oldest evicted), so only 10 undos succeed and the 11th fails.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_11_undo_capacity_limit_n_matches(seed in any::<u64>(), n in 1usize..=10) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let (board, solution) = generator.generate_with_solution(layout, 5).unwrap();

        // We need at least n pairs from the solution
        prop_assume!(solution.len() >= n);

        let mut state = make_state(board);

        // Perform n matches using the solution sequence
        for i in 0..n {
            let (pos_a, pos_b) = solution[i];

            let r1 = select_tile(&mut state, pos_a);
            prop_assert_eq!(r1, SelectionResult::Selected,
                "Match {}: first select at {} should return Selected, got {:?}", i, pos_a, r1);

            let r2 = select_tile(&mut state, pos_b);
            prop_assert_eq!(r2, SelectionResult::Matched(pos_a, pos_b),
                "Match {}: second select at {} should return Matched, got {:?}", i, pos_b, r2);
        }

        // Assert undo stack has exactly n entries
        prop_assert_eq!(state.undo_stack.len(), n,
            "After {} matches, undo stack should have {} entries, got {}",
            n, n, state.undo_stack.len());

        // All n undos should succeed
        for i in 0..n {
            let result = undo(&mut state);
            prop_assert!(result.is_ok(),
                "Undo {} of {} should succeed, got {:?}", i + 1, n, result);
        }

        // The next undo (n+1) should fail with EmptyStack
        let result = undo(&mut state);
        prop_assert_eq!(result, Err(UndoError::EmptyStack),
            "Undo after all {} undos should return EmptyStack", n);
    }

    #[test]
    fn property_11_undo_capacity_limit_overflow(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let (board, solution) = generator.generate_with_solution(layout, 5).unwrap();

        // We need at least 11 pairs from the solution to test overflow
        prop_assume!(solution.len() >= 11);

        let mut state = make_state(board);

        // Perform 11 matches — the undo stack caps at 10, so the first entry is evicted
        for i in 0..11 {
            let (pos_a, pos_b) = solution[i];

            let r1 = select_tile(&mut state, pos_a);
            prop_assert_eq!(r1, SelectionResult::Selected,
                "Match {}: first select at {} should return Selected, got {:?}", i, pos_a, r1);

            let r2 = select_tile(&mut state, pos_b);
            prop_assert_eq!(r2, SelectionResult::Matched(pos_a, pos_b),
                "Match {}: second select at {} should return Matched, got {:?}", i, pos_b, r2);
        }

        // Undo stack should be capped at 10
        prop_assert_eq!(state.undo_stack.len(), 10,
            "After 11 matches, undo stack should be capped at 10, got {}",
            state.undo_stack.len());

        // Exactly 10 undos should succeed
        for i in 0..10 {
            let result = undo(&mut state);
            prop_assert!(result.is_ok(),
                "Undo {} of 10 should succeed, got {:?}", i + 1, result);
        }

        // The 11th undo should fail with EmptyStack
        let result = undo(&mut state);
        prop_assert_eq!(result, Err(UndoError::EmptyStack),
            "11th undo should return EmptyStack");
    }
}


// Feature: lmahjong, Property 8: No-Moves Detection
//
// **Validates: Requirements 3.3**
//
// For any board state where no pair of free tiles shares the same face ID,
// the game-over check SHALL report that no valid moves are available.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_8_no_moves_detection(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        let mut state = make_state(board);

        // Reassign face_ids of all free tiles to be unique so no pairs exist
        let free_positions = state.board.free_tiles();
        prop_assume!(free_positions.len() >= 2, "Need at least 2 free tiles");

        // Assign each free tile a distinct face_id (0, 1, 2, ...)
        // Since face_id is u8 and free tiles are at most ~30-40 on a full board,
        // we have plenty of distinct values available (0..35 gives 36 unique IDs).
        for (i, &pos) in free_positions.iter().enumerate() {
            if let Some(ref mut tile) = state.board.tiles[pos] {
                tile.face_id = i as u8;
            }
        }

        // Verify precondition: no pair of free tiles shares the same face_id
        let free_after = state.board.free_tiles();
        let mut seen_faces: HashSet<u8> = HashSet::new();
        for &pos in &free_after {
            let face = state.board.tiles[pos].unwrap().face_id;
            // Each face_id should be unique among free tiles
            prop_assert!(seen_faces.insert(face),
                "Precondition violated: duplicate face_id {} among free tiles", face);
        }

        // Assert: check_game_over reports Lost (no valid moves)
        let result = check_game_over(&state);
        prop_assert_eq!(result, Some(GameOverReason::Lost),
            "Expected GameOverReason::Lost when no free tile pairs share a face_id, got {:?}", result);
    }
}


// Feature: lmahjong, Property 9: Hint Returns a Valid Pair
//
// **Validates: Requirements 4.1**
//
// For any board state that contains at least one valid matching pair among free
// tiles, requesting a hint SHALL return a pair of positions where both tiles are
// free and share the same face ID, and the hint state is set accordingly.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_9_hint_returns_a_valid_pair(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let board = generator.generate(layout, 5).unwrap();

        // Generated boards are solvable, so there must be at least one valid pair
        let valid_pairs = board.valid_pairs();
        prop_assume!(!valid_pairs.is_empty(), "No valid pairs on generated board");

        let mut state = make_state(board);

        // Request a hint
        let result = request_hint(&mut state);

        // Assert hint returns Found with two positions
        match result {
            HintResult::Found(a, b) => {
                // Assert both positions have tiles (not empty)
                prop_assert!(state.board.tiles[a].is_some(),
                    "Hint position a ({}) should have a tile", a);
                prop_assert!(state.board.tiles[b].is_some(),
                    "Hint position b ({}) should have a tile", b);

                // Assert both tiles are free
                prop_assert!(state.board.is_free(a),
                    "Hint position a ({}) should be free", a);
                prop_assert!(state.board.is_free(b),
                    "Hint position b ({}) should be free", b);

                // Assert both tiles share the same face_id
                let face_a = state.board.tiles[a].unwrap().face_id;
                let face_b = state.board.tiles[b].unwrap().face_id;
                prop_assert_eq!(face_a, face_b,
                    "Hinted tiles should share face_id: a({})={}, b({})={}",
                    a, face_a, b, face_b);

                // Assert state.hint is Some with correct positions
                prop_assert!(state.hint.is_some(),
                    "state.hint should be Some after successful hint request");
                let hint = state.hint.unwrap();
                prop_assert_eq!(hint.position_a, a,
                    "state.hint.position_a should match returned position a");
                prop_assert_eq!(hint.position_b, b,
                    "state.hint.position_b should match returned position b");
            }
            HintResult::NoMatchesAvailable => {
                prop_assert!(false,
                    "Hint should return Found for a board with valid pairs, got NoMatchesAvailable");
            }
        }
    }
}
