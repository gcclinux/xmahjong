//! Board generator module.
//!
//! Implements solvable board generation using the reverse-deal algorithm,
//! ensuring every generated board has at least one complete solution.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::board::{Board, Layout, Tile};

/// Errors that can occur during board generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationError {
    /// The algorithm reached a state where no free pairs could be found
    /// (degenerate layout configuration).
    NoFreePairs,
    /// All retry attempts were exhausted without producing a valid board.
    MaxAttemptsExceeded,
}

/// Generates solvable Mahjong solitaire boards using the reverse-deal algorithm.
///
/// The reverse-deal approach guarantees solvability by construction:
/// 1. Start with a full board (all 144 positions occupied)
/// 2. Repeatedly find and remove pairs of free tiles
/// 3. The removal order recorded IS a valid solution
/// 4. Assign face IDs to each removed pair
/// 5. Reconstruct the board with those face ID assignments
pub struct BoardGenerator {
    rng: StdRng,
}

impl BoardGenerator {
    /// Creates a new board generator with the given seed for deterministic generation.
    pub fn new(seed: u64) -> Self {
        BoardGenerator {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Generates a solvable board using the reverse-deal algorithm.
    ///
    /// The algorithm:
    /// 1. Fill all positions with dummy tiles
    /// 2. Repeatedly find free tiles, pick 2, remove them (recording the pair)
    /// 3. Once all 72 pairs are removed, randomly select 36 faces from the 50
    ///    available and assign face IDs (36 faces × 2 pairs each)
    /// 4. Build the final board from those assignments
    ///
    /// If the algorithm gets stuck (no free pairs available), it retries up to
    /// `max_attempts` times with the same RNG (which advances, producing different results).
    ///
    /// Returns `Err(GenerationError::MaxAttemptsExceeded)` if all attempts fail.
    pub fn generate(
        &mut self,
        layout: &'static Layout,
        max_attempts: u32,
    ) -> Result<Board, GenerationError> {
        let (board, _) = self.generate_with_solution(layout, max_attempts)?;
        Ok(board)
    }

    /// Generates a solvable board and returns the solution sequence.
    ///
    /// The solution is a vector of 72 position pairs in the order they should
    /// be removed to clear the board. This is the reverse of the generation's
    /// removal order — the last pair placed during generation is the first pair
    /// that can be removed during play.
    ///
    /// Returns `Err(GenerationError::MaxAttemptsExceeded)` if all attempts fail.
    pub fn generate_with_solution(
        &mut self,
        layout: &'static Layout,
        max_attempts: u32,
    ) -> Result<(Board, Vec<(usize, usize)>), GenerationError> {
        for _ in 0..max_attempts {
            match self.try_generate_with_solution(layout) {
                Ok(result) => return Ok(result),
                Err(GenerationError::NoFreePairs) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(GenerationError::MaxAttemptsExceeded)
    }

    /// Attempts a single board generation using the reverse-deal algorithm.
    fn try_generate_with_solution(
        &mut self,
        layout: &'static Layout,
    ) -> Result<(Board, Vec<(usize, usize)>), GenerationError> {
        let num_positions = layout.positions.len();

        // Step 1: Start with all positions filled (dummy tiles)
        let mut filled = vec![true; num_positions];

        // Step 2: Repeatedly find free pairs and remove them
        let mut removal_order: Vec<(usize, usize)> = Vec::with_capacity(72);

        for _ in 0..72 {
            // Find all currently free positions in the filled board
            let free_positions = self.find_free_positions(&filled, layout);

            if free_positions.len() < 2 {
                return Err(GenerationError::NoFreePairs);
            }

            // Shuffle and pick two free positions
            let mut candidates = free_positions;
            candidates.shuffle(&mut self.rng);

            let pos_a = candidates[0];
            let pos_b = candidates[1];

            // Remove them (mark as empty)
            filled[pos_a] = false;
            filled[pos_b] = false;

            // Record this removal pair
            removal_order.push((pos_a, pos_b));
        }

        // Step 3: Assign face IDs to the pairs
        // Select 36 faces randomly from the 50 available, then assign 2 pairs each = 72 pairs.
        // This gives visual variety between games since different faces appear each time.
        let mut available_faces: Vec<u8> = (0u8..50).collect();
        available_faces.shuffle(&mut self.rng);
        let selected_faces: Vec<u8> = available_faces[..36].to_vec();

        let mut face_assignments: Vec<u8> = Vec::with_capacity(72);
        for &face_id in &selected_faces {
            face_assignments.push(face_id);
            face_assignments.push(face_id);
        }
        face_assignments.shuffle(&mut self.rng);

        // Step 4: Build the final board
        let mut board = Board::new(layout);

        for (pair_idx, &(pos_a, pos_b)) in removal_order.iter().enumerate() {
            let face_id = face_assignments[pair_idx];
            board.tiles[pos_a] = Some(Tile {
                face_id,
                position: layout.positions[pos_a],
            });
            board.tiles[pos_b] = Some(Tile {
                face_id,
                position: layout.positions[pos_b],
            });
        }

        // The solution IS the removal_order itself (not reversed):
        // removal_order[0] was removed first from the full dummy board, meaning
        // those positions were free on the full board. Since the generated board
        // has tiles at the same positions, removal_order[0] positions are also
        // free on the generated board. Removing them in order replays the
        // deconstruction sequence.
        let solution = removal_order;

        Ok((board, solution))
    }

    /// Finds all positions that are currently "free" in the filled board.
    ///
    /// A position is free if:
    /// 1. It is currently filled (has a tile)
    /// 2. No position above it is filled (nothing on top)
    /// 3. At least one side (left or right) has all adjacent positions empty
    fn find_free_positions(&self, filled: &[bool], layout: &Layout) -> Vec<usize> {
        let mut free = Vec::new();

        for (i, &is_filled) in filled.iter().enumerate() {
            if !is_filled {
                continue;
            }

            let relation = &layout.blocking[i];

            // Check no tile is blocking from above
            let blocked_above = relation.blocked_by.iter().any(|&idx| filled[idx]);
            if blocked_above {
                continue;
            }

            // Check at least one side is unblocked
            let left_clear = relation.left_adjacent.iter().all(|&idx| !filled[idx]);
            let right_clear = relation.right_adjacent.iter().all(|&idx| !filled[idx]);

            if left_clear || right_clear {
                free.push(i);
            }
        }

        free
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::turtle_layout;
    use std::collections::HashMap;

    #[test]
    fn generates_board_successfully() {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(42);
        let result = generator.generate(layout, 5);
        assert!(result.is_ok(), "Board generation should succeed");
    }

    #[test]
    fn generated_board_has_144_tiles() {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(123);
        let board = generator.generate(layout, 5).unwrap();
        assert_eq!(board.remaining_count(), 144);
    }

    #[test]
    fn each_face_id_appears_exactly_4_times() {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(456);
        let board = generator.generate(layout, 5).unwrap();

        let mut face_counts: HashMap<u8, usize> = HashMap::new();
        for tile in board.tiles.iter().flatten() {
            *face_counts.entry(tile.face_id).or_insert(0) += 1;
        }

        // Should have exactly 36 distinct face IDs (randomly selected from 50 available)
        assert_eq!(face_counts.len(), 36);

        // Each face ID should appear exactly 4 times
        for (&face_id, &count) in &face_counts {
            assert_eq!(
                count, 4,
                "Face ID {} appears {} times, expected 4",
                face_id, count
            );
        }
    }

    #[test]
    fn different_seeds_produce_different_boards() {
        let layout = turtle_layout();

        let mut gen1 = BoardGenerator::new(100);
        let board1 = gen1.generate(layout, 5).unwrap();

        let mut gen2 = BoardGenerator::new(200);
        let board2 = gen2.generate(layout, 5).unwrap();

        // Collect face_ids for comparison
        let faces1: Vec<u8> = board1
            .tiles
            .iter()
            .map(|t| t.unwrap().face_id)
            .collect();
        let faces2: Vec<u8> = board2
            .tiles
            .iter()
            .map(|t| t.unwrap().face_id)
            .collect();

        assert_ne!(faces1, faces2, "Different seeds should produce different boards");
    }

    #[test]
    fn same_seed_produces_same_board() {
        let layout = turtle_layout();

        let mut gen1 = BoardGenerator::new(999);
        let board1 = gen1.generate(layout, 5).unwrap();

        let mut gen2 = BoardGenerator::new(999);
        let board2 = gen2.generate(layout, 5).unwrap();

        let faces1: Vec<u8> = board1
            .tiles
            .iter()
            .map(|t| t.unwrap().face_id)
            .collect();
        let faces2: Vec<u8> = board2
            .tiles
            .iter()
            .map(|t| t.unwrap().face_id)
            .collect();

        assert_eq!(faces1, faces2, "Same seed should produce identical boards");
    }

    #[test]
    fn all_tiles_have_valid_positions() {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(789);
        let board = generator.generate(layout, 5).unwrap();

        for (i, tile) in board.tiles.iter().enumerate() {
            let tile = tile.unwrap();
            assert_eq!(
                tile.position, layout.positions[i],
                "Tile at index {} has wrong position",
                i
            );
        }
    }

    #[test]
    fn face_ids_are_in_valid_range() {
        let layout = turtle_layout();
        let mut generator = BoardGenerator::new(321);
        let board = generator.generate(layout, 5).unwrap();

        for tile in board.tiles.iter().flatten() {
            assert!(
                tile.face_id < 50,
                "Face ID {} is out of range 0..49",
                tile.face_id
            );
        }
    }
}
