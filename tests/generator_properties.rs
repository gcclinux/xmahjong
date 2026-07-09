//! Property-based tests for the Board Generator module.

use proptest::prelude::*;

use xmahjong::board::turtle_layout;
use xmahjong::generator::BoardGenerator;

// Feature: xmahjong, Property 2: Generated Boards Are Solvable
//
// **Validates: Requirements 1.2**
//
// For any board produced by the generator, there SHALL exist at least one
// complete sequence of valid pair removals that removes all 144 tiles from
// the board. We verify this by replaying the reverse-deal solution: the
// generator records the order in which pairs were placed, and reversing
// that order yields a guaranteed valid removal sequence.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_2_generated_boards_are_solvable(seed in any::<u64>()) {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(seed);
        let (mut board, solution) = generator.generate_with_solution(layout, 5).unwrap();

        // The board starts with 144 tiles (72 pairs)
        prop_assert_eq!(
            board.remaining_count(), 144,
            "Generated board should start with 144 tiles"
        );

        // The solution should contain exactly 72 pair removals
        prop_assert_eq!(
            solution.len(), 72,
            "Solution should contain exactly 72 pair removals, got {}",
            solution.len()
        );

        // Replay the solution: each pair in the solution should be a valid removal
        for (step, &(a, b)) in solution.iter().enumerate() {
            // Both positions must have tiles
            prop_assert!(
                board.tiles[a].is_some(),
                "Step {}: position {} has no tile to remove",
                step, a
            );
            prop_assert!(
                board.tiles[b].is_some(),
                "Step {}: position {} has no tile to remove",
                step, b
            );

            // Both tiles must be free (removable)
            prop_assert!(
                board.is_free(a),
                "Step {}: tile at position {} is not free",
                step, a
            );
            prop_assert!(
                board.is_free(b),
                "Step {}: tile at position {} is not free",
                step, b
            );

            // Both tiles must share the same face_id (valid match)
            let face_a = board.tiles[a].unwrap().face_id;
            let face_b = board.tiles[b].unwrap().face_id;
            prop_assert_eq!(
                face_a, face_b,
                "Step {}: tiles at positions {} and {} have different face IDs ({} vs {})",
                step, a, b, face_a, face_b
            );

            // Remove the pair
            board.remove_pair(a, b);
        }

        // After replaying all 72 removals, the board should be empty
        prop_assert_eq!(
            board.remaining_count(), 0,
            "Board should be empty after replaying solution, {} tiles remain",
            board.remaining_count()
        );
    }
}


// Feature: xmahjong, Property 3: Board Generation Randomness
//
// **Validates: Requirements 1.3**
//
// For any two boards generated with different random seeds, the tile-to-position
// assignment SHALL differ (the boards are not identical).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_3_different_seeds_produce_different_boards(seed1: u64, seed2: u64) {
        prop_assume!(seed1 != seed2);

        let layout = turtle_layout();

        let mut gen1 = BoardGenerator::new(seed1);
        let result1 = gen1.generate(layout, 10);
        // Skip seeds that fail to generate a valid board
        prop_assume!(result1.is_ok());
        let board1 = result1.unwrap();

        let mut gen2 = BoardGenerator::new(seed2);
        let result2 = gen2.generate(layout, 10);
        prop_assume!(result2.is_ok());
        let board2 = result2.unwrap();

        // Collect the face_id assignment vectors from both boards
        let faces1: Vec<u8> = board1.tiles.iter().map(|t| t.unwrap().face_id).collect();
        let faces2: Vec<u8> = board2.tiles.iter().map(|t| t.unwrap().face_id).collect();

        prop_assert_ne!(
            faces1,
            faces2,
            "Boards generated with seeds {} and {} should have different tile-to-position assignments",
            seed1,
            seed2
        );
    }
}
