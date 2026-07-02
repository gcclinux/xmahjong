//! Game logic module.
//!
//! Handles tile selection, matching rules, undo/redo, shuffle mechanics,
//! hint generation, and win/loss condition detection.

use std::time::Instant;

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::board::Tile;
use crate::game_state::{Animation, GameState};

/// Result of attempting to select a tile on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionResult {
    /// First tile was successfully selected (no previous selection).
    Selected,
    /// Two free tiles with the same face_id were matched and removed.
    Matched(usize, usize),
    /// Two free tiles with different face_ids were selected — mismatch.
    Mismatched(usize, usize),
    /// The already-selected tile was clicked again, deselecting it.
    Deselected,
    /// The click was on a non-free tile or empty position — ignored.
    Ignored,
}

/// Error type for undo operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoError {
    /// No moves to undo (stack is empty).
    EmptyStack,
}

/// An entry in the undo stack, recording a matched pair removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoEntry {
    /// The first tile that was removed.
    pub tile_a: Tile,
    /// The second tile that was removed.
    pub tile_b: Tile,
    /// Position index of tile_a.
    pub position_a: usize,
    /// Position index of tile_b.
    pub position_b: usize,
}

/// Handles a tile selection attempt at the given position index.
///
/// Logic:
/// 1. If position is empty or not free → Ignored
/// 2. If same tile clicked again → Deselected
/// 3. If no prior selection → Selected
/// 4. If prior selection exists:
///    a. Same face_id → Matched (remove pair, push undo)
///    b. Different face_id → Mismatched
///
/// Dismisses any active hint on any selection action.
pub fn select_tile(state: &mut GameState, pos: usize) -> SelectionResult {
    // Empty position — ignore
    if state.board.tiles[pos].is_none() {
        return SelectionResult::Ignored;
    }

    // Not free — ignore
    if !state.board.is_free(pos) {
        return SelectionResult::Ignored;
    }

    // Dismiss active hint on any valid interaction
    state.hint = None;

    // Re-click the same selected tile — deselect
    if state.selection == Some(pos) {
        state.selection = None;
        return SelectionResult::Deselected;
    }

    // No prior selection — select this tile
    if state.selection.is_none() {
        state.selection = Some(pos);
        return SelectionResult::Selected;
    }

    // There is a prior selection — compare face_ids
    let other = state.selection.unwrap();
    let face_a = state.board.tiles[other].unwrap().face_id;
    let face_b = state.board.tiles[pos].unwrap().face_id;

    if face_a == face_b {
        // Match! Save tiles for undo before removing
        let tile_a = state.board.tiles[other].unwrap();
        let tile_b = state.board.tiles[pos].unwrap();

        state.board.remove_pair(other, pos);

        // Push undo entry (cap at 10)
        if state.undo_stack.len() >= 10 {
            state.undo_stack.remove(0);
        }
        state.undo_stack.push(UndoEntry {
            tile_a,
            tile_b,
            position_a: other,
            position_b: pos,
        });

        state.selection = None;
        SelectionResult::Matched(other, pos)
    } else {
        // Mismatch — deselect both
        state.selection = None;
        SelectionResult::Mismatched(other, pos)
    }
}

/// Undoes the last matched pair removal, restoring both tiles to the board.
///
/// - Pops the most recent `UndoEntry` from `state.undo_stack`
/// - Restores both tiles using `board.restore_pair()`
/// - Clears any current selection
/// - Returns `UndoError::EmptyStack` if the undo stack is empty
pub fn undo(state: &mut GameState) -> Result<(), UndoError> {
    let entry = state.undo_stack.pop().ok_or(UndoError::EmptyStack)?;

    state.board.restore_pair(
        entry.position_a,
        entry.position_b,
        entry.tile_a,
        entry.tile_b,
    );

    state.selection = None;

    Ok(())
}

/// Result of requesting a hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintResult {
    /// A valid pair was found and highlighted.
    Found(usize, usize),
    /// No valid matches exist.
    NoMatchesAvailable,
}

/// Reason the game has ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverReason {
    /// All tiles cleared — player wins!
    Won,
    /// No valid moves remain — player loses.
    Lost,
}

/// Error type for shuffle operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShuffleError {
    /// No shuffles remaining (max 3 per game).
    NoShufflesRemaining,
    /// Could not produce a valid arrangement after 10 retries.
    NoValidArrangement,
}

/// Shuffles the face IDs among all occupied tile positions on the board.
///
/// - Checks that shuffles_remaining > 0
/// - Collects face_ids from occupied positions, shuffles them, reassigns
/// - Ensures at least one valid pair exists among free tiles (retries up to 10 times)
/// - Decrements shuffles_remaining, clears undo stack and selection
/// - Adds a Shuffle animation to state.animations
/// - Increments state.score.shuffles_used
pub fn shuffle(state: &mut GameState) -> Result<(), ShuffleError> {
    // 1. Check shuffles_remaining > 0
    if state.shuffles_remaining == 0 {
        return Err(ShuffleError::NoShufflesRemaining);
    }

    // 2. Collect occupied position indices and their face_ids
    let occupied_positions: Vec<usize> = (0..state.board.tiles.len())
        .filter(|&i| state.board.tiles[i].is_some())
        .collect();

    let mut face_ids: Vec<u8> = occupied_positions
        .iter()
        .map(|&i| state.board.tiles[i].unwrap().face_id)
        .collect();

    // 3-6. Shuffle and retry up to 10 times until a valid pair exists
    let mut rng = thread_rng();
    let mut found_valid = false;

    for _ in 0..10 {
        face_ids.shuffle(&mut rng);

        // Temporarily assign shuffled face_ids to check validity
        for (idx, &pos) in occupied_positions.iter().enumerate() {
            if let Some(ref mut tile) = state.board.tiles[pos] {
                tile.face_id = face_ids[idx];
            }
        }

        // Check if at least one valid pair exists among free tiles
        if !state.board.valid_pairs().is_empty() {
            found_valid = true;
            break;
        }
    }

    // 7. If all retries fail, return error
    if !found_valid {
        return Err(ShuffleError::NoValidArrangement);
    }

    // 8. Decrement shuffles_remaining
    state.shuffles_remaining -= 1;

    // 9. Clear undo_stack
    state.undo_stack.clear();

    // 10. Clear selection
    state.selection = None;

    // 11. Add Shuffle animation
    state.animations.push(Animation::Shuffle {
        start_time: Instant::now(),
        duration_ms: 500,
    });

    // 12. Increment shuffles_used
    state.score.shuffles_used += 1;

    Ok(())
}

/// Requests a hint by finding one valid pair among free tiles and highlighting it.
///
/// - Finds valid pairs via `board.valid_pairs()`
/// - If a pair exists, sets `state.hint` with the first pair found and increments `hints_used`
/// - If no pairs exist, returns `NoMatchesAvailable`
/// - Replaces any existing hint (requirement 4.5)
pub fn request_hint(state: &mut GameState) -> HintResult {
    let pairs = state.board.valid_pairs();
    if pairs.is_empty() {
        return HintResult::NoMatchesAvailable;
    }
    // Pick the first valid pair
    let (a, b) = pairs[0];
    state.hint = Some(crate::game_state::HintState {
        position_a: a,
        position_b: b,
        activated_at: Instant::now(),
    });
    state.score.hints_used += 1;
    HintResult::Found(a, b)
}

/// Checks whether the game is over (won or lost).
///
/// - Returns `Some(Won)` if the board is completely empty (all tiles removed)
/// - Returns `Some(Lost)` if no valid pairs exist among remaining free tiles
/// - Returns `None` if the game is still in progress (valid moves exist)
pub fn check_game_over(state: &GameState) -> Option<GameOverReason> {
    if state.board.remaining_count() == 0 {
        return Some(GameOverReason::Won);
    }
    if state.board.valid_pairs().is_empty() {
        return Some(GameOverReason::Lost);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{turtle_layout, Board, Tile};
    use crate::game_state::{
        GameState, GameStatus, HintState, ScoreTracker,
    };
    use crate::timer::GameTimer;
    use std::time::Instant;

    /// Helper to create a minimal GameState with a given board.
    fn make_state(board: Board) -> GameState {
        GameState {
            board,
            timer: GameTimer::new(),
            score: ScoreTracker::new(),
            status: GameStatus::Playing,
            selection: None,
            hint: None,
            undo_stack: Vec::new(),
            shuffles_remaining: 3,
            animations: Vec::new(),
        }
    }

    #[test]
    fn select_empty_position_returns_ignored() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        let mut state = make_state(board);

        // Position 0 has no tile — should be ignored
        assert_eq!(select_tile(&mut state, 0), SelectionResult::Ignored);
        assert_eq!(state.selection, None);
    }

    #[test]
    fn select_non_free_tile_returns_ignored() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place a tile on layer 3 and block it from above with a layer 4 tile
        let layer_3_idx = layout
            .positions
            .iter()
            .enumerate()
            .find(|(_, p)| p.layer == 3)
            .map(|(i, _)| i)
            .unwrap();

        board.tiles[layer_3_idx] = Some(Tile {
            face_id: 1,
            position: layout.positions[layer_3_idx],
        });

        // Find a layer 4 blocker
        let blocker = layout.blocking[layer_3_idx]
            .blocked_by
            .iter()
            .find(|&&b| layout.positions[b].layer == 4)
            .copied();

        if let Some(blocker_idx) = blocker {
            board.tiles[blocker_idx] = Some(Tile {
                face_id: 2,
                position: layout.positions[blocker_idx],
            });

            let mut state = make_state(board);
            // layer_3_idx is blocked from above
            assert_eq!(
                select_tile(&mut state, layer_3_idx),
                SelectionResult::Ignored
            );
        }
    }

    #[test]
    fn select_free_tile_first_selection() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place only the two top-layer tiles
        board.tiles[142] = Some(Tile {
            face_id: 5,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 5,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        assert_eq!(select_tile(&mut state, 142), SelectionResult::Selected);
        assert_eq!(state.selection, Some(142));
    }

    #[test]
    fn select_same_tile_again_deselects() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 5,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 5,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        select_tile(&mut state, 142);
        assert_eq!(select_tile(&mut state, 142), SelectionResult::Deselected);
        assert_eq!(state.selection, None);
    }

    #[test]
    fn select_matching_pair_removes_tiles() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        select_tile(&mut state, 142);
        let result = select_tile(&mut state, 143);

        assert_eq!(result, SelectionResult::Matched(142, 143));
        assert!(state.board.tiles[142].is_none());
        assert!(state.board.tiles[143].is_none());
        assert_eq!(state.selection, None);
    }

    #[test]
    fn select_mismatched_pair_deselects_both() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 20,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        select_tile(&mut state, 142);
        let result = select_tile(&mut state, 143);

        assert_eq!(result, SelectionResult::Mismatched(142, 143));
        // Tiles still present
        assert!(state.board.tiles[142].is_some());
        assert!(state.board.tiles[143].is_some());
        assert_eq!(state.selection, None);
    }

    #[test]
    fn match_pushes_undo_entry() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 7,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 7,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        select_tile(&mut state, 142);
        select_tile(&mut state, 143);

        assert_eq!(state.undo_stack.len(), 1);
        let entry = &state.undo_stack[0];
        assert_eq!(entry.position_a, 142);
        assert_eq!(entry.position_b, 143);
        assert_eq!(entry.tile_a.face_id, 7);
        assert_eq!(entry.tile_b.face_id, 7);
    }

    #[test]
    fn undo_stack_capped_at_10() {
        let layout = turtle_layout();
        let board = Board::new(layout);

        // We need 11 pairs of free tiles. Use top-layer tiles plus layer 3
        // Just place multiple pairs at top-layer by doing repeated matches
        // Actually, for simplicity, we'll manipulate the undo stack directly
        // and verify the cap works through select_tile.

        // Place a pair at top layer, match, repeat. But after removal we need new free tiles.
        // Simpler: pre-fill undo_stack with 9 entries, then do one match.
        let mut state = make_state(board);

        // Pre-fill with 9 dummy entries
        for i in 0..9 {
            state.undo_stack.push(UndoEntry {
                tile_a: Tile {
                    face_id: i as u8,
                    position: layout.positions[0],
                },
                tile_b: Tile {
                    face_id: i as u8,
                    position: layout.positions[1],
                },
                position_a: 0,
                position_b: 1,
            });
        }

        // Place a matchable pair
        state.board.tiles[142] = Some(Tile {
            face_id: 30,
            position: layout.positions[142],
        });
        state.board.tiles[143] = Some(Tile {
            face_id: 30,
            position: layout.positions[143],
        });

        select_tile(&mut state, 142);
        select_tile(&mut state, 143);

        assert_eq!(state.undo_stack.len(), 10);
        // The last entry should be our match
        assert_eq!(state.undo_stack[9].tile_a.face_id, 30);

        // Now add one more — stack should still be 10, oldest removed
        state.board.tiles[140] = Some(Tile {
            face_id: 25,
            position: layout.positions[140],
        });
        state.board.tiles[141] = Some(Tile {
            face_id: 25,
            position: layout.positions[141],
        });

        // Verify these positions are free (layer 3, top positions before layer 4)
        // Actually positions 140-141 are layer 3 and layer 4 tiles have been removed,
        // so they should be free now.
        select_tile(&mut state, 140);
        select_tile(&mut state, 141);

        assert_eq!(state.undo_stack.len(), 10);
        assert_eq!(state.undo_stack[9].tile_a.face_id, 25);
    }

    #[test]
    fn match_dismisses_active_hint() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 5,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 5,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        state.hint = Some(HintState {
            position_a: 142,
            position_b: 143,
            activated_at: Instant::now(),
        });

        select_tile(&mut state, 142);
        // Hint should be dismissed on first valid selection
        assert!(state.hint.is_none());
    }

    #[test]
    fn free_tiles_recalculated_after_match() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Fill all positions
        for i in 0..144 {
            board.tiles[i] = Some(Tile {
                face_id: (i % 50) as u8,
                position: layout.positions[i],
            });
        }

        // Make top-layer tiles have the same face_id for a valid match
        board.tiles[142] = Some(Tile {
            face_id: 35,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 35,
            position: layout.positions[143],
        });

        let mut state = make_state(board);

        // Before match, layer 3 tiles blocked by layer 4 should not be free
        let free_before = state.board.free_tiles();
        // Layer 3 tiles that are blocked by positions 142/143 should not be in free set
        let layer3_blocked: Vec<usize> = (0..144)
            .filter(|&i| {
                layout.positions[i].layer == 3
                    && layout.blocking[i]
                        .blocked_by
                        .iter()
                        .any(|&b| b == 142 || b == 143)
            })
            .collect();

        for &idx in &layer3_blocked {
            assert!(
                !free_before.contains(&idx),
                "Layer 3 tile at {} should not be free before removing layer 4",
                idx
            );
        }

        // Match the top-layer pair
        select_tile(&mut state, 142);
        select_tile(&mut state, 143);

        // After match, layer 3 tiles that were only blocked by 142/143 may now be free
        let free_after = state.board.free_tiles();
        // At least verify the removed tiles are not in free set
        assert!(!free_after.contains(&142));
        assert!(!free_after.contains(&143));
    }

    #[test]
    fn undo_after_match_restores_tiles() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        let tile_a = Tile {
            face_id: 12,
            position: layout.positions[142],
        };
        let tile_b = Tile {
            face_id: 12,
            position: layout.positions[143],
        };

        board.tiles[142] = Some(tile_a);
        board.tiles[143] = Some(tile_b);

        let mut state = make_state(board);

        // Match the pair
        select_tile(&mut state, 142);
        select_tile(&mut state, 143);

        // Tiles should be removed
        assert!(state.board.tiles[142].is_none());
        assert!(state.board.tiles[143].is_none());
        assert_eq!(state.undo_stack.len(), 1);

        // Undo the match
        let result = undo(&mut state);
        assert_eq!(result, Ok(()));

        // Tiles should be restored
        assert_eq!(state.board.tiles[142], Some(tile_a));
        assert_eq!(state.board.tiles[143], Some(tile_b));
        assert_eq!(state.undo_stack.len(), 0);
    }

    #[test]
    fn undo_on_empty_stack_returns_error() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        let mut state = make_state(board);

        assert!(state.undo_stack.is_empty());
        let result = undo(&mut state);
        assert_eq!(result, Err(UndoError::EmptyStack));
    }

    #[test]
    fn undo_clears_current_selection() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 8,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 8,
            position: layout.positions[143],
        });

        let mut state = make_state(board);

        // Match the pair
        select_tile(&mut state, 142);
        select_tile(&mut state, 143);

        // Simulate a selection being active before undo
        // (e.g., user selected another tile, then triggered undo)
        state.board.tiles[140] = Some(Tile {
            face_id: 3,
            position: layout.positions[140],
        });
        state.selection = Some(140);

        // Undo should clear selection
        let result = undo(&mut state);
        assert_eq!(result, Ok(()));
        assert_eq!(state.selection, None);
    }

    // =========================================================================
    // Shuffle tests
    // =========================================================================

    /// Helper to create a board with multiple pairs of free tiles for shuffle testing.
    /// Places 4 tiles at top layer positions (142, 143) and layer 3 positions (140, 141)
    /// all with the same face_id so that any shuffle permutation always guarantees a valid
    /// pair among the free tiles (142, 143).
    fn make_shuffleable_state() -> GameState {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Use the same face_id for all 4 tiles to guarantee that any permutation
        // of face_ids will always produce a valid pair at the free positions (142, 143).
        board.tiles[142] = Some(Tile {
            face_id: 1,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 1,
            position: layout.positions[143],
        });
        board.tiles[140] = Some(Tile {
            face_id: 1,
            position: layout.positions[140],
        });
        board.tiles[141] = Some(Tile {
            face_id: 1,
            position: layout.positions[141],
        });

        make_state(board)
    }

    #[test]
    fn shuffle_rearranges_face_ids() {
        let mut state = make_shuffleable_state();

        // Collect original face_ids (as a multiset)
        let original_faces: Vec<u8> = (0..state.board.tiles.len())
            .filter_map(|i| state.board.tiles[i].map(|t| t.face_id))
            .collect();
        let mut original_sorted = original_faces.clone();
        original_sorted.sort();

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));

        // Collect post-shuffle face_ids
        let shuffled_faces: Vec<u8> = (0..state.board.tiles.len())
            .filter_map(|i| state.board.tiles[i].map(|t| t.face_id))
            .collect();
        let mut shuffled_sorted = shuffled_faces.clone();
        shuffled_sorted.sort();

        // Multiset of face_ids should be preserved
        assert_eq!(original_sorted, shuffled_sorted);
    }

    #[test]
    fn shuffle_fails_when_no_shuffles_remaining() {
        let mut state = make_shuffleable_state();
        state.shuffles_remaining = 0;

        let result = shuffle(&mut state);
        assert_eq!(result, Err(ShuffleError::NoShufflesRemaining));
    }

    #[test]
    fn shuffle_clears_undo_stack() {
        let mut state = make_shuffleable_state();

        // Pre-fill undo stack with some entries
        let layout = turtle_layout();
        state.undo_stack.push(UndoEntry {
            tile_a: Tile {
                face_id: 5,
                position: layout.positions[0],
            },
            tile_b: Tile {
                face_id: 5,
                position: layout.positions[1],
            },
            position_a: 0,
            position_b: 1,
        });
        assert!(!state.undo_stack.is_empty());

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));

        // Undo stack should be cleared
        assert!(state.undo_stack.is_empty());
    }

    #[test]
    fn shuffle_clears_selection() {
        let mut state = make_shuffleable_state();

        // Set a selection
        state.selection = Some(142);

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));

        // Selection should be cleared
        assert_eq!(state.selection, None);
    }

    #[test]
    fn shuffle_decrements_shuffles_remaining() {
        let mut state = make_shuffleable_state();
        assert_eq!(state.shuffles_remaining, 3);

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));
        assert_eq!(state.shuffles_remaining, 2);

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));
        assert_eq!(state.shuffles_remaining, 1);

        let result = shuffle(&mut state);
        assert_eq!(result, Ok(()));
        assert_eq!(state.shuffles_remaining, 0);

        // Fourth shuffle should fail
        let result = shuffle(&mut state);
        assert_eq!(result, Err(ShuffleError::NoShufflesRemaining));
    }

    #[test]
    fn shuffle_increments_shuffles_used() {
        let mut state = make_shuffleable_state();
        assert_eq!(state.score.shuffles_used, 0);

        let _ = shuffle(&mut state);
        assert_eq!(state.score.shuffles_used, 1);
    }

    #[test]
    fn shuffle_adds_animation() {
        let mut state = make_shuffleable_state();
        assert!(state.animations.is_empty());

        let _ = shuffle(&mut state);
        assert_eq!(state.animations.len(), 1);

        match state.animations[0] {
            Animation::Shuffle { duration_ms, .. } => {
                assert_eq!(duration_ms, 500);
            }
            _ => panic!("Expected Shuffle animation"),
        }
    }

    // =========================================================================
    // Hint and game-over detection tests
    // =========================================================================

    #[test]
    fn request_hint_valid_pairs_exist_sets_hint_state() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place a matching pair at top layer
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let mut state = make_state(board);
        assert!(state.hint.is_none());
        assert_eq!(state.score.hints_used, 0);

        let result = request_hint(&mut state);

        assert_eq!(result, HintResult::Found(142, 143));
        assert!(state.hint.is_some());
        let hint = state.hint.unwrap();
        assert_eq!(hint.position_a, 142);
        assert_eq!(hint.position_b, 143);
        assert_eq!(state.score.hints_used, 1);
    }

    #[test]
    fn request_hint_no_valid_pairs_returns_no_matches() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place two tiles with different face_ids — no valid pair
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 20,
            position: layout.positions[143],
        });

        let mut state = make_state(board);

        let result = request_hint(&mut state);

        assert_eq!(result, HintResult::NoMatchesAvailable);
        assert!(state.hint.is_none());
        assert_eq!(state.score.hints_used, 0);
    }

    #[test]
    fn request_hint_replaces_existing_hint() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place a matching pair at top layer
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let mut state = make_state(board);

        // Set a previous hint (different positions, simulating a stale hint)
        state.hint = Some(HintState {
            position_a: 0,
            position_b: 1,
            activated_at: Instant::now(),
        });

        let result = request_hint(&mut state);

        // Should replace the old hint with the new valid pair
        assert_eq!(result, HintResult::Found(142, 143));
        let hint = state.hint.unwrap();
        assert_eq!(hint.position_a, 142);
        assert_eq!(hint.position_b, 143);
        // hints_used should be incremented
        assert_eq!(state.score.hints_used, 1);
    }

    #[test]
    fn check_game_over_board_empty_returns_won() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        let state = make_state(board);

        // Empty board — all tiles cleared
        assert_eq!(state.board.remaining_count(), 0);
        let result = check_game_over(&state);
        assert_eq!(result, Some(GameOverReason::Won));
    }

    #[test]
    fn check_game_over_no_valid_pairs_returns_lost() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place tiles with different face_ids so no valid pairs exist
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 20,
            position: layout.positions[143],
        });

        let state = make_state(board);

        assert_eq!(state.board.remaining_count(), 2);
        let result = check_game_over(&state);
        assert_eq!(result, Some(GameOverReason::Lost));
    }

    #[test]
    fn check_game_over_valid_pairs_exist_returns_none() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place a matching pair — game is still in progress
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let state = make_state(board);

        let result = check_game_over(&state);
        assert_eq!(result, None);
    }

    #[test]
    fn request_hint_increments_hints_used_each_time() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let mut state = make_state(board);

        request_hint(&mut state);
        assert_eq!(state.score.hints_used, 1);

        request_hint(&mut state);
        assert_eq!(state.score.hints_used, 2);

        request_hint(&mut state);
        assert_eq!(state.score.hints_used, 3);
    }
}
