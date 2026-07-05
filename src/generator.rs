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

    /// Generates a solvable board with a specific number of tiles.
    ///
    /// `tile_count` must be a multiple of 4 (each face needs 4 tiles to form 2 matchable pairs).
    /// The board uses the full layout but only places tiles at `tile_count` positions.
    ///
    /// Returns `Err(GenerationError::MaxAttemptsExceeded)` if all attempts fail.
    pub fn generate_with_tile_count(
        &mut self,
        layout: &'static Layout,
        tile_count: usize,
        max_attempts: u32,
    ) -> Result<Board, GenerationError> {
        assert!(tile_count % 4 == 0, "tile_count must be a multiple of 4");
        assert!(tile_count <= layout.positions.len(), "tile_count exceeds layout capacity");
        for _ in 0..max_attempts {
            match self.try_generate_with_solution_tiles(layout, tile_count) {
                Ok((board, _)) => return Ok(board),
                Err(GenerationError::NoFreePairs) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(GenerationError::MaxAttemptsExceeded)
    }

    /// Generates a solvable board with a specific number of tiles and a custom face pool.
    ///
    /// `tile_count` must be a multiple of 4. `face_pool` provides the set of face IDs
    /// to choose from (e.g., a mix of penguin and dog face IDs for later levels).
    ///
    /// Returns `Err(GenerationError::MaxAttemptsExceeded)` if all attempts fail.
    pub fn generate_with_faces(
        &mut self,
        layout: &'static Layout,
        tile_count: usize,
        face_pool: &[u8],
        max_attempts: u32,
    ) -> Result<Board, GenerationError> {
        assert!(tile_count % 4 == 0, "tile_count must be a multiple of 4");
        assert!(tile_count <= layout.positions.len(), "tile_count exceeds layout capacity");
        let num_faces_needed = tile_count / 4;
        assert!(face_pool.len() >= num_faces_needed, "face_pool too small for tile_count");
        for _ in 0..max_attempts {
            match self.try_generate_with_faces(layout, tile_count, face_pool) {
                Ok(board) => return Ok(board),
                Err(GenerationError::NoFreePairs) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(GenerationError::MaxAttemptsExceeded)
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
        self.try_generate_with_solution_tiles(layout, layout.positions.len())
    }

    /// Attempts a single board generation with a specified number of tiles.
    ///
    /// `tile_count` must be even (pairs). The algorithm starts with all positions
    /// filled, removes pairs until only `tile_count` positions remain, then assigns
    /// face IDs to those remaining positions.
    fn try_generate_with_solution_tiles(
        &mut self,
        layout: &'static Layout,
        tile_count: usize,
    ) -> Result<(Board, Vec<(usize, usize)>), GenerationError> {
        let num_positions = layout.positions.len();
        let num_pairs = tile_count / 2;
        let pairs_to_remove = (num_positions - tile_count) / 2;

        // Step 1: Start with all positions filled (dummy tiles)
        let mut filled = vec![true; num_positions];

        // Step 1b: Remove pairs to reduce to the target tile count
        // These removed positions will stay empty in the final board.
        for _ in 0..pairs_to_remove {
            let free_positions = self.find_free_positions(&filled, layout);
            if free_positions.len() < 2 {
                return Err(GenerationError::NoFreePairs);
            }
            let mut candidates = free_positions;
            candidates.shuffle(&mut self.rng);
            filled[candidates[0]] = false;
            filled[candidates[1]] = false;
        }

        // Step 2: From the reduced board, repeatedly find free pairs and remove them
        // to establish the solution/placement order.
        let mut removal_order: Vec<(usize, usize)> = Vec::with_capacity(num_pairs);

        for _ in 0..num_pairs {
            let free_positions = self.find_free_positions(&filled, layout);

            if free_positions.len() < 2 {
                return Err(GenerationError::NoFreePairs);
            }

            let mut candidates = free_positions;
            candidates.shuffle(&mut self.rng);

            let pos_a = candidates[0];
            let pos_b = candidates[1];

            filled[pos_a] = false;
            filled[pos_b] = false;

            removal_order.push((pos_a, pos_b));
        }

        // Step 3: Assign face IDs to the pairs
        // Each face appears exactly 4 times (2 pairs), so we need num_pairs/2 distinct faces.
        let num_faces = num_pairs / 2;
        let available_face_count = 50u8;
        let mut available_faces: Vec<u8> = (0..available_face_count).collect();
        available_faces.shuffle(&mut self.rng);
        let selected_faces: Vec<u8> = available_faces[..num_faces.min(available_face_count as usize)].to_vec();

        let mut face_assignments: Vec<u8> = Vec::with_capacity(num_pairs);
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

        let solution = removal_order;

        Ok((board, solution))
    }

    /// Attempts a single board generation with a custom face pool.
    ///
    /// Similar to `try_generate_with_solution_tiles` but draws face IDs from the
    /// provided `face_pool` rather than the default 0..50 range.
    fn try_generate_with_faces(
        &mut self,
        layout: &'static Layout,
        tile_count: usize,
        face_pool: &[u8],
    ) -> Result<Board, GenerationError> {
        let num_positions = layout.positions.len();
        let num_pairs = tile_count / 2;
        let pairs_to_remove = (num_positions - tile_count) / 2;

        let mut filled = vec![true; num_positions];

        // Remove pairs to reduce to the target tile count
        for _ in 0..pairs_to_remove {
            let free_positions = self.find_free_positions(&filled, layout);
            if free_positions.len() < 2 {
                return Err(GenerationError::NoFreePairs);
            }
            let mut candidates = free_positions;
            candidates.shuffle(&mut self.rng);
            filled[candidates[0]] = false;
            filled[candidates[1]] = false;
        }

        // Find free pairs and remove them to establish solution order
        let mut removal_order: Vec<(usize, usize)> = Vec::with_capacity(num_pairs);

        for _ in 0..num_pairs {
            let free_positions = self.find_free_positions(&filled, layout);
            if free_positions.len() < 2 {
                return Err(GenerationError::NoFreePairs);
            }
            let mut candidates = free_positions;
            candidates.shuffle(&mut self.rng);
            let pos_a = candidates[0];
            let pos_b = candidates[1];
            filled[pos_a] = false;
            filled[pos_b] = false;
            removal_order.push((pos_a, pos_b));
        }

        // Assign face IDs from the custom face pool
        let num_faces = num_pairs / 2;
        let mut pool: Vec<u8> = face_pool.to_vec();
        pool.shuffle(&mut self.rng);
        let selected_faces: Vec<u8> = pool[..num_faces].to_vec();

        let mut face_assignments: Vec<u8> = Vec::with_capacity(num_pairs);
        for &face_id in &selected_faces {
            face_assignments.push(face_id);
            face_assignments.push(face_id);
        }
        face_assignments.shuffle(&mut self.rng);

        // Build the final board
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

        Ok(board)
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
