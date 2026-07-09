//! Property-based tests for the Board Generator module.

use std::collections::HashMap;
use proptest::prelude::*;

use xmahjong::board::turtle_layout;
use xmahjong::generator::BoardGenerator;
use xmahjong::levels;

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


// Feature: space-levels, Property 5: Board face assignment correctness
//
// **Validates: Requirements 4.2, 4.4**
//
// For any level N in 1..=50, the generated board SHALL use exactly tile_count/4
// distinct face IDs, each appearing exactly 4 times, all from the face pool.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn property_5_board_face_assignment(seed in any::<u64>(), level in 1u32..=50) {
        let layout = turtle_layout();
        let tile_count = levels::tiles_for_level(level);
        let face_pool = levels::face_pool_for_level(level);
        let mut generator = BoardGenerator::new(seed);

        let board = if level <= 10 {
            if tile_count < 144 {
                generator.generate_with_tile_count(layout, tile_count, 10)
            } else {
                generator.generate(layout, 5)
            }
        } else {
            generator.generate_with_faces(layout, tile_count, &face_pool, 10)
        };

        prop_assume!(board.is_ok());
        let board = board.unwrap();

        // Count face ID occurrences
        let mut face_counts: HashMap<u8, usize> = HashMap::new();
        for tile in board.tiles.iter().flatten() {
            *face_counts.entry(tile.face_id).or_insert(0) += 1;
        }

        // Verify exactly tile_count/4 distinct face IDs
        let expected_distinct = tile_count / 4;
        prop_assert_eq!(face_counts.len(), expected_distinct,
            "Level {}: expected {} distinct faces, got {}", level, expected_distinct, face_counts.len());

        // Verify each face ID appears exactly 4 times
        for (&face_id, &count) in &face_counts {
            prop_assert_eq!(count, 4,
                "Level {}: face_id {} appears {} times, expected 4", level, face_id, count);
        }

        // Verify all face IDs are from the face pool (for levels 11+)
        if level > 10 {
            let pool_set: std::collections::HashSet<u8> = face_pool.iter().copied().collect();
            for &face_id in face_counts.keys() {
                prop_assert!(pool_set.contains(&face_id),
                    "Level {}: face_id {} is not in the face pool", level, face_id);
            }
        }
    }
}


// Feature: space-levels, Property 6: Board solvability
//
// **Validates: Requirements 4.3, 5.1**
//
// For any generated board at level N in 1..=50, a solution path SHALL exist
// that clears all tiles from the board.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn property_6_board_solvability(seed in any::<u64>(), level in 1u32..=50) {
        let layout = turtle_layout();
        let tile_count = levels::tiles_for_level(level);
        let face_pool = levels::face_pool_for_level(level);
        let mut generator = BoardGenerator::new(seed);

        // Use solution-returning variants to get a proven solution for verification
        let result = if level <= 10 {
            if tile_count < 144 {
                generator.generate_with_tile_count_and_solution(layout, tile_count, 10)
            } else {
                generator.generate_with_solution(layout, 5)
            }
        } else {
            generator.generate_with_faces_and_solution(layout, tile_count, &face_pool, 10)
        };

        prop_assume!(result.is_ok());
        let (mut board, solution) = result.unwrap();

        // Verify board starts with correct tile count
        prop_assert_eq!(board.remaining_count(), tile_count,
            "Level {}: board has {} tiles, expected {}", level, board.remaining_count(), tile_count);

        // Verify solution has correct number of pair removals
        let expected_pairs = tile_count / 2;
        prop_assert_eq!(solution.len(), expected_pairs,
            "Level {}: solution has {} steps, expected {}", level, solution.len(), expected_pairs);

        // Replay the solution: each pair should be a valid removal
        for (step, &(a, b)) in solution.iter().enumerate() {
            prop_assert!(board.tiles[a].is_some(),
                "Level {} step {}: no tile at position {}", level, step, a);
            prop_assert!(board.tiles[b].is_some(),
                "Level {} step {}: no tile at position {}", level, step, b);
            prop_assert!(board.is_free(a),
                "Level {} step {}: tile at {} not free", level, step, a);
            prop_assert!(board.is_free(b),
                "Level {} step {}: tile at {} not free", level, step, b);

            let face_a = board.tiles[a].unwrap().face_id;
            let face_b = board.tiles[b].unwrap().face_id;
            prop_assert_eq!(face_a, face_b,
                "Level {} step {}: face mismatch {} vs {}", level, step, face_a, face_b);

            board.remove_pair(a, b);
        }

        // After replaying all removals, the board should be empty
        prop_assert_eq!(board.remaining_count(), 0,
            "Level {}: board not empty after solution, {} tiles remain", level, board.remaining_count());
    }
}
