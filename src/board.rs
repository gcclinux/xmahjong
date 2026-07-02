//! Board module.
//!
//! Manages the tile layout, positions, blocking relations, and free-tile detection
//! for the Mahjong solitaire board.

use std::sync::OnceLock;

/// Represents a tile on the board with its face image and position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// Face image ID (0..35), identifies which of the 36 tile faces this tile shows.
    pub face_id: u8,
    /// The position this tile occupies on the board.
    pub position: TilePosition,
}

/// A position in the Turtle layout, defined by layer, row, and column.
/// Coordinates use half-tile grid units (each tile occupies a 2x2 grid space).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePosition {
    /// Layer index (0..4). Layer 0 is the bottom, layer 4 is the top.
    pub layer: u8,
    /// Row coordinate in the half-tile grid.
    pub row: u8,
    /// Column coordinate in the half-tile grid.
    pub col: u8,
}

/// The static layout definition containing all tile positions and blocking relations.
#[derive(Debug)]
pub struct Layout {
    /// All 144 tile slot positions in the Turtle formation.
    pub positions: &'static [TilePosition],
    /// Precomputed blocking/adjacency relations for each position.
    pub blocking: Vec<BlockingRelation>,
}

/// Describes blocking relationships for a single position.
/// A tile is free if no tile is on top AND at least one side (left or right) is unblocked.
#[derive(Debug, Clone)]
pub struct BlockingRelation {
    /// The position index this relation describes.
    pub position: usize,
    /// Indices of positions that block this one from above (tiles stacked on top).
    pub blocked_by: Vec<usize>,
    /// Indices of positions adjacent to the left at the same layer.
    pub left_adjacent: Vec<usize>,
    /// Indices of positions adjacent to the right at the same layer.
    pub right_adjacent: Vec<usize>,
}

/// The board state: holds tiles at each position and a reference to the layout.
#[derive(Debug)]
pub struct Board {
    /// Tile slots indexed by position ID. `None` means the position is empty.
    pub tiles: Vec<Option<Tile>>,
    /// Reference to the static layout used by this board.
    pub layout: &'static Layout,
}

/// Returns the static Turtle layout (positions + blocking relations).
/// Computed once on first access.
pub fn turtle_layout() -> &'static Layout {
    static LAYOUT: OnceLock<Layout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        let positions = TURTLE_POSITIONS;
        let blocking = compute_blocking_relations(positions);
        Layout {
            positions,
            blocking,
        }
    })
}

/// Computes blocking relations for all positions in the layout.
/// A position is blocked from above if another position on a higher layer overlaps it.
/// A position has a left/right adjacent if another position on the same layer is
/// directly to its left/right (col differs by exactly 2, same row range overlaps).
fn compute_blocking_relations(positions: &[TilePosition]) -> Vec<BlockingRelation> {
    let len = positions.len();
    let mut relations = Vec::with_capacity(len);

    for i in 0..len {
        let pos = &positions[i];
        let mut blocked_by = Vec::new();
        let mut left_adjacent = Vec::new();
        let mut right_adjacent = Vec::new();

        for j in 0..len {
            if i == j {
                continue;
            }
            let other = &positions[j];

            // Check if 'other' is directly above 'pos' (blocks from above).
            // A tile on a higher layer blocks if it overlaps horizontally and vertically.
            // Each tile occupies a 2x2 grid area. Overlap means the ranges intersect.
            if other.layer == pos.layer + 1 {
                // Horizontal overlap: pos occupies [col, col+2), other occupies [other.col, other.col+2)
                let h_overlap = pos.col < other.col + 2 && other.col < pos.col + 2;
                // Vertical overlap: pos occupies [row, row+2), other occupies [other.row, other.row+2)
                let v_overlap = pos.row < other.row + 2 && other.row < pos.row + 2;
                if h_overlap && v_overlap {
                    blocked_by.push(j);
                }
            }

            // Check left/right adjacency (same layer, same row range, col differs by 2).
            if other.layer == pos.layer {
                // Vertical overlap check for adjacency
                let v_overlap = pos.row < other.row + 2 && other.row < pos.row + 2;
                if v_overlap {
                    if other.col + 2 == pos.col {
                        // 'other' is directly to the left of 'pos'
                        left_adjacent.push(j);
                    } else if pos.col + 2 == other.col {
                        // 'other' is directly to the right of 'pos'
                        right_adjacent.push(j);
                    }
                }
            }
        }

        relations.push(BlockingRelation {
            position: i,
            blocked_by,
            left_adjacent,
            right_adjacent,
        });
    }

    relations
}


// =============================================================================
// Classic Turtle Layout - 144 tile positions across 5 layers
// =============================================================================
//
// Coordinates use a half-tile grid where each tile occupies a 2-wide × 2-tall cell.
// The layout mirrors the traditional Mahjong solitaire "Turtle" formation.
//
// Layer 0 (bottom): 86 tiles - the main body
// Layer 1: 36 tiles
// Layer 2: 16 tiles
// Layer 3: 4 tiles
// Layer 4 (top): 2 tiles
//
// Total: 144 tiles

/// The static array of all 144 tile positions in the Turtle layout.
pub static TURTLE_POSITIONS: &[TilePosition] = &[
    // =========================================================================
    // Layer 0: 86 tiles - the turtle body shape
    // =========================================================================
    // Row 0: 12 tiles (cols 2..26 in steps of 2)
    TilePosition { layer: 0, row: 0, col: 2 },
    TilePosition { layer: 0, row: 0, col: 4 },
    TilePosition { layer: 0, row: 0, col: 6 },
    TilePosition { layer: 0, row: 0, col: 8 },
    TilePosition { layer: 0, row: 0, col: 10 },
    TilePosition { layer: 0, row: 0, col: 12 },
    TilePosition { layer: 0, row: 0, col: 14 },
    TilePosition { layer: 0, row: 0, col: 16 },
    TilePosition { layer: 0, row: 0, col: 18 },
    TilePosition { layer: 0, row: 0, col: 20 },
    TilePosition { layer: 0, row: 0, col: 22 },
    TilePosition { layer: 0, row: 0, col: 24 },
    // Row 2: 12 tiles
    TilePosition { layer: 0, row: 2, col: 2 },
    TilePosition { layer: 0, row: 2, col: 4 },
    TilePosition { layer: 0, row: 2, col: 6 },
    TilePosition { layer: 0, row: 2, col: 8 },
    TilePosition { layer: 0, row: 2, col: 10 },
    TilePosition { layer: 0, row: 2, col: 12 },
    TilePosition { layer: 0, row: 2, col: 14 },
    TilePosition { layer: 0, row: 2, col: 16 },
    TilePosition { layer: 0, row: 2, col: 18 },
    TilePosition { layer: 0, row: 2, col: 20 },
    TilePosition { layer: 0, row: 2, col: 22 },
    TilePosition { layer: 0, row: 2, col: 24 },
    // Row 4: 14 tiles (wider - turtle extends with head/tail)
    TilePosition { layer: 0, row: 4, col: 0 },
    TilePosition { layer: 0, row: 4, col: 2 },
    TilePosition { layer: 0, row: 4, col: 4 },
    TilePosition { layer: 0, row: 4, col: 6 },
    TilePosition { layer: 0, row: 4, col: 8 },
    TilePosition { layer: 0, row: 4, col: 10 },
    TilePosition { layer: 0, row: 4, col: 12 },
    TilePosition { layer: 0, row: 4, col: 14 },
    TilePosition { layer: 0, row: 4, col: 16 },
    TilePosition { layer: 0, row: 4, col: 18 },
    TilePosition { layer: 0, row: 4, col: 20 },
    TilePosition { layer: 0, row: 4, col: 22 },
    TilePosition { layer: 0, row: 4, col: 24 },
    TilePosition { layer: 0, row: 4, col: 26 },
    // Row 6: 14 tiles
    TilePosition { layer: 0, row: 6, col: 0 },
    TilePosition { layer: 0, row: 6, col: 2 },
    TilePosition { layer: 0, row: 6, col: 4 },
    TilePosition { layer: 0, row: 6, col: 6 },
    TilePosition { layer: 0, row: 6, col: 8 },
    TilePosition { layer: 0, row: 6, col: 10 },
    TilePosition { layer: 0, row: 6, col: 12 },
    TilePosition { layer: 0, row: 6, col: 14 },
    TilePosition { layer: 0, row: 6, col: 16 },
    TilePosition { layer: 0, row: 6, col: 18 },
    TilePosition { layer: 0, row: 6, col: 20 },
    TilePosition { layer: 0, row: 6, col: 22 },
    TilePosition { layer: 0, row: 6, col: 24 },
    TilePosition { layer: 0, row: 6, col: 26 },
    // Row 8: 14 tiles
    TilePosition { layer: 0, row: 8, col: 0 },
    TilePosition { layer: 0, row: 8, col: 2 },
    TilePosition { layer: 0, row: 8, col: 4 },
    TilePosition { layer: 0, row: 8, col: 6 },
    TilePosition { layer: 0, row: 8, col: 8 },
    TilePosition { layer: 0, row: 8, col: 10 },
    TilePosition { layer: 0, row: 8, col: 12 },
    TilePosition { layer: 0, row: 8, col: 14 },
    TilePosition { layer: 0, row: 8, col: 16 },
    TilePosition { layer: 0, row: 8, col: 18 },
    TilePosition { layer: 0, row: 8, col: 20 },
    TilePosition { layer: 0, row: 8, col: 22 },
    TilePosition { layer: 0, row: 8, col: 24 },
    TilePosition { layer: 0, row: 8, col: 26 },
    // Row 10: 12 tiles
    TilePosition { layer: 0, row: 10, col: 2 },
    TilePosition { layer: 0, row: 10, col: 4 },
    TilePosition { layer: 0, row: 10, col: 6 },
    TilePosition { layer: 0, row: 10, col: 8 },
    TilePosition { layer: 0, row: 10, col: 10 },
    TilePosition { layer: 0, row: 10, col: 12 },
    TilePosition { layer: 0, row: 10, col: 14 },
    TilePosition { layer: 0, row: 10, col: 16 },
    TilePosition { layer: 0, row: 10, col: 18 },
    TilePosition { layer: 0, row: 10, col: 20 },
    TilePosition { layer: 0, row: 10, col: 22 },
    TilePosition { layer: 0, row: 10, col: 24 },
    // Row 12: 8 tiles (narrower - turtle legs/tail taper)
    TilePosition { layer: 0, row: 12, col: 6 },
    TilePosition { layer: 0, row: 12, col: 8 },
    TilePosition { layer: 0, row: 12, col: 10 },
    TilePosition { layer: 0, row: 12, col: 12 },
    TilePosition { layer: 0, row: 12, col: 14 },
    TilePosition { layer: 0, row: 12, col: 16 },
    TilePosition { layer: 0, row: 12, col: 18 },
    TilePosition { layer: 0, row: 12, col: 20 },

    // =========================================================================
    // Layer 1: 36 tiles - centered rectangle on top of layer 0
    // =========================================================================
    // Row 2: 10 tiles
    TilePosition { layer: 1, row: 2, col: 4 },
    TilePosition { layer: 1, row: 2, col: 6 },
    TilePosition { layer: 1, row: 2, col: 8 },
    TilePosition { layer: 1, row: 2, col: 10 },
    TilePosition { layer: 1, row: 2, col: 12 },
    TilePosition { layer: 1, row: 2, col: 14 },
    TilePosition { layer: 1, row: 2, col: 16 },
    TilePosition { layer: 1, row: 2, col: 18 },
    TilePosition { layer: 1, row: 2, col: 20 },
    TilePosition { layer: 1, row: 2, col: 22 },
    // Row 4: 10 tiles
    TilePosition { layer: 1, row: 4, col: 4 },
    TilePosition { layer: 1, row: 4, col: 6 },
    TilePosition { layer: 1, row: 4, col: 8 },
    TilePosition { layer: 1, row: 4, col: 10 },
    TilePosition { layer: 1, row: 4, col: 12 },
    TilePosition { layer: 1, row: 4, col: 14 },
    TilePosition { layer: 1, row: 4, col: 16 },
    TilePosition { layer: 1, row: 4, col: 18 },
    TilePosition { layer: 1, row: 4, col: 20 },
    TilePosition { layer: 1, row: 4, col: 22 },
    // Row 6: 8 tiles
    TilePosition { layer: 1, row: 6, col: 6 },
    TilePosition { layer: 1, row: 6, col: 8 },
    TilePosition { layer: 1, row: 6, col: 10 },
    TilePosition { layer: 1, row: 6, col: 12 },
    TilePosition { layer: 1, row: 6, col: 14 },
    TilePosition { layer: 1, row: 6, col: 16 },
    TilePosition { layer: 1, row: 6, col: 18 },
    TilePosition { layer: 1, row: 6, col: 20 },
    // Row 8: 8 tiles
    TilePosition { layer: 1, row: 8, col: 6 },
    TilePosition { layer: 1, row: 8, col: 8 },
    TilePosition { layer: 1, row: 8, col: 10 },
    TilePosition { layer: 1, row: 8, col: 12 },
    TilePosition { layer: 1, row: 8, col: 14 },
    TilePosition { layer: 1, row: 8, col: 16 },
    TilePosition { layer: 1, row: 8, col: 18 },
    TilePosition { layer: 1, row: 8, col: 20 },

    // =========================================================================
    // Layer 2: 16 tiles - smaller centered rectangle
    // =========================================================================
    // Row 4: 6 tiles
    TilePosition { layer: 2, row: 4, col: 8 },
    TilePosition { layer: 2, row: 4, col: 10 },
    TilePosition { layer: 2, row: 4, col: 12 },
    TilePosition { layer: 2, row: 4, col: 14 },
    TilePosition { layer: 2, row: 4, col: 16 },
    TilePosition { layer: 2, row: 4, col: 18 },
    // Row 6: 6 tiles
    TilePosition { layer: 2, row: 6, col: 8 },
    TilePosition { layer: 2, row: 6, col: 10 },
    TilePosition { layer: 2, row: 6, col: 12 },
    TilePosition { layer: 2, row: 6, col: 14 },
    TilePosition { layer: 2, row: 6, col: 16 },
    TilePosition { layer: 2, row: 6, col: 18 },
    // Row 8: 4 tiles (to reach 16 total)
    TilePosition { layer: 2, row: 8, col: 10 },
    TilePosition { layer: 2, row: 8, col: 12 },
    TilePosition { layer: 2, row: 8, col: 14 },
    TilePosition { layer: 2, row: 8, col: 16 },

    // =========================================================================
    // Layer 3: 4 tiles - small square near center
    // =========================================================================
    TilePosition { layer: 3, row: 5, col: 11 },
    TilePosition { layer: 3, row: 5, col: 13 },
    TilePosition { layer: 3, row: 7, col: 11 },
    TilePosition { layer: 3, row: 7, col: 13 },

    // =========================================================================
    // Layer 4 (top): 2 tiles - single pair at the very top
    // =========================================================================
    TilePosition { layer: 4, row: 6, col: 12 },
    TilePosition { layer: 4, row: 6, col: 14 },
];

impl Board {
    /// Creates a new empty board with all positions set to `None`.
    pub fn new(layout: &'static Layout) -> Self {
        let tiles = vec![None; layout.positions.len()];
        Board { tiles, layout }
    }

    /// Returns true if the tile at `pos` is free.
    ///
    /// A tile is free if:
    /// 1. There is a tile present at the position
    /// 2. No tile occupies any position in its `blocked_by` list (nothing on top)
    /// 3. ALL positions in `left_adjacent` are empty OR ALL positions in `right_adjacent` are empty
    pub fn is_free(&self, pos: usize) -> bool {
        // Must have a tile at this position
        if self.tiles[pos].is_none() {
            return false;
        }

        let relation = &self.layout.blocking[pos];

        // Check no tile is blocking from above
        let blocked_above = relation
            .blocked_by
            .iter()
            .any(|&idx| self.tiles[idx].is_some());
        if blocked_above {
            return false;
        }

        // Check at least one side is unblocked
        let left_clear = relation
            .left_adjacent
            .iter()
            .all(|&idx| self.tiles[idx].is_none());
        let right_clear = relation
            .right_adjacent
            .iter()
            .all(|&idx| self.tiles[idx].is_none());

        left_clear || right_clear
    }

    /// Returns all positions that currently have a free tile.
    pub fn free_tiles(&self) -> Vec<usize> {
        (0..self.tiles.len())
            .filter(|&pos| self.is_free(pos))
            .collect()
    }

    /// Returns all valid matching pairs among free tiles.
    /// A valid pair is two free tiles that share the same `face_id`.
    pub fn valid_pairs(&self) -> Vec<(usize, usize)> {
        let free = self.free_tiles();
        let mut pairs = Vec::new();

        for i in 0..free.len() {
            for j in (i + 1)..free.len() {
                let tile_a = self.tiles[free[i]].unwrap();
                let tile_b = self.tiles[free[j]].unwrap();
                if tile_a.face_id == tile_b.face_id {
                    pairs.push((free[i], free[j]));
                }
            }
        }

        pairs
    }

    /// Removes the tiles at positions `a` and `b` from the board.
    pub fn remove_pair(&mut self, a: usize, b: usize) {
        self.tiles[a] = None;
        self.tiles[b] = None;
    }

    /// Restores previously removed tiles at positions `a` and `b`.
    pub fn restore_pair(&mut self, a: usize, b: usize, tile_a: Tile, tile_b: Tile) {
        self.tiles[a] = Some(tile_a);
        self.tiles[b] = Some(tile_b);
    }

    /// Returns the number of tiles remaining on the board.
    pub fn remaining_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_layout_has_144_positions() {
        assert_eq!(TURTLE_POSITIONS.len(), 144);
    }

    #[test]
    fn turtle_layout_has_5_layers() {
        let max_layer = TURTLE_POSITIONS.iter().map(|p| p.layer).max().unwrap();
        assert_eq!(max_layer, 4);
    }

    #[test]
    fn turtle_layout_layer_counts() {
        let layer_0 = TURTLE_POSITIONS.iter().filter(|p| p.layer == 0).count();
        let layer_1 = TURTLE_POSITIONS.iter().filter(|p| p.layer == 1).count();
        let layer_2 = TURTLE_POSITIONS.iter().filter(|p| p.layer == 2).count();
        let layer_3 = TURTLE_POSITIONS.iter().filter(|p| p.layer == 3).count();
        let layer_4 = TURTLE_POSITIONS.iter().filter(|p| p.layer == 4).count();

        assert_eq!(layer_0, 86);
        assert_eq!(layer_1, 36);
        assert_eq!(layer_2, 16);
        assert_eq!(layer_3, 4);
        assert_eq!(layer_4, 2);
        assert_eq!(layer_0 + layer_1 + layer_2 + layer_3 + layer_4, 144);
    }

    #[test]
    fn turtle_layout_no_duplicate_positions() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for pos in TURTLE_POSITIONS.iter() {
            assert!(
                seen.insert((pos.layer, pos.row, pos.col)),
                "Duplicate position: layer={}, row={}, col={}",
                pos.layer,
                pos.row,
                pos.col
            );
        }
    }

    #[test]
    fn blocking_relations_computed_correctly() {
        let layout = turtle_layout();
        assert_eq!(layout.blocking.len(), 144);

        // The top-layer tiles (layer 4) should have no tiles blocking them from above
        for rel in &layout.blocking {
            let pos = &layout.positions[rel.position];
            if pos.layer == 4 {
                assert!(
                    rel.blocked_by.is_empty(),
                    "Top layer tile at ({},{},{}) should not be blocked from above",
                    pos.layer,
                    pos.row,
                    pos.col
                );
            }
        }

        // Verify that layer 4 positions appear in blocked_by of layer 3 positions.
        let layer_4_indices: Vec<usize> = layout
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.layer == 4)
            .map(|(i, _)| i)
            .collect();

        let layer_3_indices: Vec<usize> = layout
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.layer == 3)
            .map(|(i, _)| i)
            .collect();

        // At least some layer 3 positions should be blocked by layer 4 positions
        let any_blocked = layer_3_indices.iter().any(|&i| {
            layout.blocking[i]
                .blocked_by
                .iter()
                .any(|&b| layer_4_indices.contains(&b))
        });
        assert!(
            any_blocked,
            "Some layer 3 tiles should be blocked by layer 4 tiles"
        );
    }

    // =========================================================================
    // Board impl unit tests
    // =========================================================================

    #[test]
    fn board_new_creates_empty_board() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        assert_eq!(board.tiles.len(), 144);
        assert!(board.tiles.iter().all(|t| t.is_none()));
        assert_eq!(board.remaining_count(), 0);
    }

    #[test]
    fn board_is_free_returns_false_for_empty_position() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        // No tile at position 0 — not free
        assert!(!board.is_free(0));
    }

    #[test]
    fn board_is_free_top_layer_tiles_are_free() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place tiles at the two top-layer positions (indices 142 and 143)
        let tile_142 = Tile {
            face_id: 0,
            position: layout.positions[142],
        };
        let tile_143 = Tile {
            face_id: 0,
            position: layout.positions[143],
        };
        board.tiles[142] = Some(tile_142);
        board.tiles[143] = Some(tile_143);

        // Top layer tiles have no tiles above them.
        // They are adjacent to each other (left/right), so one side may be blocked.
        // But at least one side should be clear since there are only 2 tiles total at layer 4.
        // Position 142 has right_adjacent containing 143 (occupied), but left_adjacent should be empty.
        assert!(board.is_free(142));
        assert!(board.is_free(143));
    }

    #[test]
    fn board_is_free_blocked_from_above() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Find a layer 3 position that is blocked by a layer 4 position
        let layer_3_idx = layout
            .positions
            .iter()
            .enumerate()
            .find(|(_, p)| p.layer == 3)
            .map(|(i, _)| i)
            .unwrap();

        // Place a tile at this layer 3 position
        board.tiles[layer_3_idx] = Some(Tile {
            face_id: 1,
            position: layout.positions[layer_3_idx],
        });

        // Find a layer 4 position that blocks it
        let blocker = layout.blocking[layer_3_idx]
            .blocked_by
            .iter()
            .find(|&&b| layout.positions[b].layer == 4)
            .copied();

        if let Some(blocker_idx) = blocker {
            // Place a tile on top
            board.tiles[blocker_idx] = Some(Tile {
                face_id: 2,
                position: layout.positions[blocker_idx],
            });
            // The layer 3 tile should NOT be free (blocked from above)
            assert!(!board.is_free(layer_3_idx));
        }
    }

    #[test]
    fn board_is_free_blocked_on_both_sides() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Find a position that has both left and right adjacents
        let target = layout.blocking.iter().find(|rel| {
            !rel.left_adjacent.is_empty() && !rel.right_adjacent.is_empty()
        });

        if let Some(rel) = target {
            let pos = rel.position;
            // Place the target tile
            board.tiles[pos] = Some(Tile {
                face_id: 5,
                position: layout.positions[pos],
            });
            // Place tiles in ALL left adjacent positions
            for &left in &rel.left_adjacent {
                board.tiles[left] = Some(Tile {
                    face_id: 6,
                    position: layout.positions[left],
                });
            }
            // Place tiles in ALL right adjacent positions
            for &right in &rel.right_adjacent {
                board.tiles[right] = Some(Tile {
                    face_id: 7,
                    position: layout.positions[right],
                });
            }
            // Should NOT be free — blocked on both sides
            // (but only if not blocked from above, which it isn't since we didn't place above)
            assert!(!board.is_free(pos));
        }
    }

    #[test]
    fn board_free_tiles_empty_board() {
        let layout = turtle_layout();
        let board = Board::new(layout);
        assert!(board.free_tiles().is_empty());
    }

    #[test]
    fn board_free_tiles_only_top_layer() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place tiles only at top layer (positions 142 and 143)
        board.tiles[142] = Some(Tile {
            face_id: 0,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 0,
            position: layout.positions[143],
        });

        let free = board.free_tiles();
        assert_eq!(free.len(), 2);
        assert!(free.contains(&142));
        assert!(free.contains(&143));
    }

    #[test]
    fn board_valid_pairs_matching_faces() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place two tiles with the same face_id at the top layer
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 10,
            position: layout.positions[143],
        });

        let pairs = board.valid_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (142, 143));
    }

    #[test]
    fn board_valid_pairs_no_match_different_faces() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Place two tiles with different face_ids at the top layer
        board.tiles[142] = Some(Tile {
            face_id: 10,
            position: layout.positions[142],
        });
        board.tiles[143] = Some(Tile {
            face_id: 20,
            position: layout.positions[143],
        });

        let pairs = board.valid_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn board_remove_pair() {
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

        assert_eq!(board.remaining_count(), 2);
        board.remove_pair(142, 143);
        assert_eq!(board.remaining_count(), 0);
        assert!(board.tiles[142].is_none());
        assert!(board.tiles[143].is_none());
    }

    #[test]
    fn board_restore_pair() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        let tile_a = Tile {
            face_id: 5,
            position: layout.positions[142],
        };
        let tile_b = Tile {
            face_id: 5,
            position: layout.positions[143],
        };

        // Start empty, restore tiles
        board.restore_pair(142, 143, tile_a, tile_b);
        assert_eq!(board.remaining_count(), 2);
        assert_eq!(board.tiles[142], Some(tile_a));
        assert_eq!(board.tiles[143], Some(tile_b));
    }

    #[test]
    fn board_remove_and_restore_round_trip() {
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

        // Remove and verify
        board.remove_pair(142, 143);
        assert_eq!(board.remaining_count(), 0);

        // Restore and verify state is identical
        board.restore_pair(142, 143, tile_a, tile_b);
        assert_eq!(board.remaining_count(), 2);
        assert_eq!(board.tiles[142], Some(tile_a));
        assert_eq!(board.tiles[143], Some(tile_b));
    }

    #[test]
    fn board_remaining_count_full_board() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Fill all positions
        for i in 0..144 {
            board.tiles[i] = Some(Tile {
                face_id: (i % 36) as u8,
                position: layout.positions[i],
            });
        }

        assert_eq!(board.remaining_count(), 144);
    }

    #[test]
    fn board_free_tiles_updates_after_removal() {
        let layout = turtle_layout();
        let mut board = Board::new(layout);

        // Fill all positions to create a full board
        for i in 0..144 {
            board.tiles[i] = Some(Tile {
                face_id: (i % 36) as u8,
                position: layout.positions[i],
            });
        }

        let free_before = board.free_tiles();
        // Top layer tiles should be free on a full board
        assert!(free_before.contains(&142) || free_before.contains(&143));

        // Remove the top two tiles
        board.remove_pair(142, 143);

        let free_after = board.free_tiles();
        // After removing the top tiles, some layer 3 tiles that were blocked
        // by them may now become free
        assert!(!free_after.contains(&142));
        assert!(!free_after.contains(&143));
    }
}
