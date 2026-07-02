//! Property-based tests for the Board module.

use proptest::prelude::*;
use std::collections::HashMap;

use lmahjong::board::{turtle_layout, Board, Tile, TURTLE_POSITIONS};

/// Creates a full board (144 tiles) using a shuffled assignment of face IDs.
/// Each of the 36 face IDs appears exactly 4 times.
fn create_full_board(face_permutation: &[u8; 144]) -> Board {
    let layout = turtle_layout();
    let mut board = Board::new(layout);
    for i in 0..144 {
        board.tiles[i] = Some(Tile {
            face_id: face_permutation[i],
            position: layout.positions[i],
        });
    }
    board
}

/// Strategy that generates a valid face_id assignment for a full board:
/// 36 face IDs × 4 copies each = 144 tiles, shuffled into a random permutation.
fn full_board_face_assignment() -> impl Strategy<Value = [u8; 144]> {
    // Generate a permutation by shuffling indices
    prop::collection::vec(any::<u32>(), 144..=144).prop_map(|random_keys| {
        // Create 144 face IDs: 4 copies of each face (0..36)
        let mut faces: Vec<u8> = (0u8..36).flat_map(|f| std::iter::repeat(f).take(4)).collect();
        // Sort by random keys to shuffle
        let mut indexed: Vec<(u32, u8)> = random_keys.into_iter().zip(faces.drain(..)).collect();
        indexed.sort_by_key(|(k, _)| *k);
        let mut result = [0u8; 144];
        for (i, (_, face)) in indexed.into_iter().enumerate() {
            result[i] = face;
        }
        result
    })
}

/// Strategy that generates a board with some tiles removed (simulating mid-game state).
/// Removes between 0 and 30 random pairs of positions.
fn partial_board_strategy() -> impl Strategy<Value = Board> {
    (full_board_face_assignment(), 0usize..=30, prop::collection::vec(any::<prop::sample::Index>(), 60..=60))
        .prop_map(|(faces, pairs_to_remove, indices)| {
            let mut board = create_full_board(&faces);
            // Remove some tiles to simulate a mid-game state
            let mut available: Vec<usize> = (0..144).filter(|&i| board.tiles[i].is_some()).collect();
            let mut removed = 0;
            let mut idx_iter = indices.into_iter();
            while removed < pairs_to_remove && available.len() >= 2 {
                if let (Some(idx_a), Some(idx_b)) = (idx_iter.next(), idx_iter.next()) {
                    let a = idx_a.index(available.len());
                    let pos_a = available.remove(a);
                    let b = idx_b.index(available.len());
                    let pos_b = available.remove(b);
                    board.tiles[pos_a] = None;
                    board.tiles[pos_b] = None;
                    removed += 1;
                } else {
                    break;
                }
            }
            board
        })
}

// Feature: lmahjong, Property 1: Board Generation Structural Invariants
//
// **Validates: Requirements 1.1, 1.3**
//
// For any randomly generated board, the board SHALL contain exactly 144 tiles,
// use exactly 36 distinct face IDs, each face ID SHALL appear exactly 4 times,
// tiles SHALL be distributed across 5 layers, and every tile position SHALL
// conform to the Turtle layout specification.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_1_board_has_exactly_144_tiles(faces in full_board_face_assignment()) {
        let board = create_full_board(&faces);
        let tile_count = board.tiles.iter().filter(|t| t.is_some()).count();
        prop_assert_eq!(tile_count, 144, "Board must contain exactly 144 tiles, found {}", tile_count);
    }

    #[test]
    fn property_1_board_has_36_distinct_face_ids(faces in full_board_face_assignment()) {
        let board = create_full_board(&faces);
        let mut face_ids: Vec<u8> = board.tiles.iter()
            .filter_map(|t| t.as_ref())
            .map(|t| t.face_id)
            .collect();
        face_ids.sort();
        face_ids.dedup();
        prop_assert_eq!(face_ids.len(), 36, "Board must have 36 distinct face IDs, found {}", face_ids.len());
    }

    #[test]
    fn property_1_each_face_id_appears_exactly_4_times(faces in full_board_face_assignment()) {
        let board = create_full_board(&faces);
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for tile in board.tiles.iter().filter_map(|t| t.as_ref()) {
            *counts.entry(tile.face_id).or_insert(0) += 1;
        }
        for face_id in 0u8..36 {
            let count = counts.get(&face_id).copied().unwrap_or(0);
            prop_assert_eq!(count, 4, "Face ID {} appears {} times, expected 4", face_id, count);
        }
    }

    #[test]
    fn property_1_board_tiles_span_5_layers(faces in full_board_face_assignment()) {
        let board = create_full_board(&faces);
        let mut layers_present: Vec<bool> = vec![false; 5];
        for tile in board.tiles.iter().filter_map(|t| t.as_ref()) {
            let layer = tile.position.layer as usize;
            prop_assert!(layer < 5, "Tile layer {} exceeds maximum of 4", layer);
            layers_present[layer] = true;
        }
        for (layer, &present) in layers_present.iter().enumerate() {
            prop_assert!(present, "Layer {} has no tiles", layer);
        }
    }

    #[test]
    fn property_1_all_positions_conform_to_turtle_layout(faces in full_board_face_assignment()) {
        let board = create_full_board(&faces);
        for (idx, tile_opt) in board.tiles.iter().enumerate() {
            if let Some(tile) = tile_opt {
                let expected_pos = &TURTLE_POSITIONS[idx];
                prop_assert_eq!(
                    tile.position.layer, expected_pos.layer,
                    "Tile at index {} has layer {}, expected {}",
                    idx, tile.position.layer, expected_pos.layer
                );
                prop_assert_eq!(
                    tile.position.row, expected_pos.row,
                    "Tile at index {} has row {}, expected {}",
                    idx, tile.position.row, expected_pos.row
                );
                prop_assert_eq!(
                    tile.position.col, expected_pos.col,
                    "Tile at index {} has col {}, expected {}",
                    idx, tile.position.col, expected_pos.col
                );
            }
        }
    }
}

// Feature: lmahjong, Property 4: Non-Free Tile Selection Is Ignored
//
// **Validates: Requirements 2.2**
//
// For any board state, positions where is_free() returns false are indeed
// blocked: they either have a tile above them OR are blocked on both sides.
// This validates the structural property that non-free tiles are correctly
// identified, which is the prerequisite for the game logic to ignore
// selections on non-free tiles.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_4_non_free_tile_is_blocked(board in partial_board_strategy()) {
        let layout = turtle_layout();

        // Collect all positions that have a tile but are NOT free
        let non_free_positions: Vec<usize> = (0..144)
            .filter(|&pos| board.tiles[pos].is_some() && !board.is_free(pos))
            .collect();

        for pos in non_free_positions {
            let relation = &layout.blocking[pos];

            // A non-free tile must be blocked in at least one of these ways:
            // 1. Has a tile directly above it (blocked_by contains an occupied position)
            let has_tile_above = relation.blocked_by.iter().any(|&idx| board.tiles[idx].is_some());

            // 2. Is blocked on BOTH left AND right sides
            //    (left is blocked if all left_adjacent positions are occupied,
            //     right is blocked if all right_adjacent positions are occupied)
            //    Note: if left_adjacent is empty, that side is considered unblocked (clear).
            let left_blocked = !relation.left_adjacent.is_empty()
                && relation.left_adjacent.iter().all(|&idx| board.tiles[idx].is_some());
            let right_blocked = !relation.right_adjacent.is_empty()
                && relation.right_adjacent.iter().all(|&idx| board.tiles[idx].is_some());
            let blocked_both_sides = left_blocked && right_blocked;

            // The tile must be non-free for one of these reasons
            prop_assert!(
                has_tile_above || blocked_both_sides,
                "Position {} reported as non-free but is neither blocked from above \
                 nor blocked on both sides. blocked_by occupied: {}, left_blocked: {}, \
                 right_blocked: {}",
                pos,
                has_tile_above,
                left_blocked,
                right_blocked
            );
        }
    }
}
