//! Game state module.
//!
//! Defines the central game state structure and game status enum,
//! holding all data needed to represent the current state of a game session.

use std::time::Instant;

use crate::board::Board;
use crate::logic::UndoEntry;
use crate::timer::GameTimer;

/// Central game state holding all data for the current session.
pub struct GameState {
    /// The board with tile positions and occupancy.
    pub board: Board,
    /// Elapsed time tracker with pause support.
    pub timer: GameTimer,
    /// Score tracking (hints, shuffles, time).
    pub score: ScoreTracker,
    /// Current game phase.
    pub status: GameStatus,
    /// Currently selected tile position index, if any.
    pub selection: Option<usize>,
    /// Active hint highlight state, if any.
    pub hint: Option<HintState>,
    /// Stack of moves available for undo (max 10).
    pub undo_stack: Vec<UndoEntry>,
    /// Number of shuffles remaining (starts at 3).
    pub shuffles_remaining: u8,
    /// Active animations being played.
    pub animations: Vec<Animation>,
}

/// The current phase/status of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    /// Game is actively being played.
    Playing,
    /// Game is paused (timer stopped, input disabled).
    Paused,
    /// Player has cleared all tiles.
    Won,
    /// No valid moves remain.
    Lost,
    /// Main menu is displayed.
    Menu,
    /// Player is entering their name for the leaderboard.
    NameEntry,
}

/// Tracks score-relevant statistics for the current game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreTracker {
    /// Number of hints used this game.
    pub hints_used: u32,
    /// Number of shuffles used this game.
    pub shuffles_used: u32,
    /// Elapsed seconds at game completion (snapshot for scoring).
    pub elapsed_seconds: u32,
}

impl ScoreTracker {
    /// Creates a new score tracker with zero values.
    pub fn new() -> Self {
        Self {
            hints_used: 0,
            shuffles_used: 0,
            elapsed_seconds: 0,
        }
    }

    /// Calculates the final score using the formula:
    /// `max(0, 1000 - elapsed_seconds - hints_used * 50 - shuffles_used * 100)`
    pub fn calculate_score(&self) -> u32 {
        let penalty = self.elapsed_seconds + self.hints_used * 50 + self.shuffles_used * 100;
        1000u32.saturating_sub(penalty)
    }
}

impl Default for ScoreTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// State for the leaderboard name entry flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntryState {
    /// Characters typed so far by the player.
    pub text: String,
    /// The score that qualified for the leaderboard.
    pub score: u32,
    /// The elapsed time in seconds at game completion.
    pub time_seconds: u32,
}

impl NameEntryState {
    /// Creates a new name entry state with the given score and time.
    pub fn new(score: u32, time_seconds: u32) -> Self {
        Self {
            text: String::new(),
            score,
            time_seconds,
        }
    }

    /// Appends a character to the name buffer if it won't exceed 20 characters.
    /// Returns true if the character was added.
    pub fn push_char(&mut self, c: char) -> bool {
        if self.text.chars().count() < 20 {
            self.text.push(c);
            true
        } else {
            false
        }
    }

    /// Removes the last character from the name buffer.
    /// Returns true if a character was removed.
    pub fn pop_char(&mut self) -> bool {
        self.text.pop().is_some()
    }

    /// Returns true if the current text is a valid name (1-20 characters).
    pub fn is_valid(&self) -> bool {
        let len = self.text.chars().count();
        (1..=20).contains(&len)
    }
}

/// State of an active hint highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HintState {
    /// First tile position of the hinted pair.
    pub position_a: usize,
    /// Second tile position of the hinted pair.
    pub position_b: usize,
    /// When the hint was activated (for auto-dismiss after 3 seconds).
    pub activated_at: Instant,
}

/// Animations that can be playing on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Animation {
    /// Fade-out animation when a matched pair is removed.
    TileRemoval {
        positions: (usize, usize),
        start_time: Instant,
        duration_ms: u32,
    },
    /// Red flash animation for mismatched pair.
    TileMismatch {
        positions: (usize, usize),
        start_time: Instant,
        duration_ms: u32,
    },
    /// Pulsing glow on hinted tiles.
    HintPulse {
        positions: (usize, usize),
        start_time: Instant,
    },
    /// Shuffle animation when tiles are rearranged.
    Shuffle {
        start_time: Instant,
        duration_ms: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_perfect_game() {
        let tracker = ScoreTracker {
            hints_used: 0,
            shuffles_used: 0,
            elapsed_seconds: 0,
        };
        assert_eq!(tracker.calculate_score(), 1000);
    }

    #[test]
    fn score_with_penalties() {
        let tracker = ScoreTracker {
            hints_used: 2,
            shuffles_used: 1,
            elapsed_seconds: 120,
        };
        // 1000 - 120 - 100 - 100 = 680
        assert_eq!(tracker.calculate_score(), 680);
    }

    #[test]
    fn score_never_negative() {
        let tracker = ScoreTracker {
            hints_used: 10,
            shuffles_used: 10,
            elapsed_seconds: 9999,
        };
        assert_eq!(tracker.calculate_score(), 0);
    }

    #[test]
    fn score_exact_zero() {
        let tracker = ScoreTracker {
            hints_used: 0,
            shuffles_used: 0,
            elapsed_seconds: 1000,
        };
        assert_eq!(tracker.calculate_score(), 0);
    }

    #[test]
    fn name_entry_new_creates_empty_state() {
        let entry = NameEntryState::new(500, 120);
        assert_eq!(entry.text, "");
        assert_eq!(entry.score, 500);
        assert_eq!(entry.time_seconds, 120);
        assert!(!entry.is_valid()); // Empty name is invalid
    }

    #[test]
    fn name_entry_push_char_adds_characters() {
        let mut entry = NameEntryState::new(500, 120);
        assert!(entry.push_char('A'));
        assert!(entry.push_char('l'));
        assert!(entry.push_char('i'));
        assert_eq!(entry.text, "Ali");
        assert!(entry.is_valid());
    }

    #[test]
    fn name_entry_push_char_rejects_beyond_20() {
        let mut entry = NameEntryState::new(500, 120);
        for c in "12345678901234567890".chars() {
            assert!(entry.push_char(c));
        }
        assert_eq!(entry.text.chars().count(), 20);
        assert!(entry.is_valid());

        // 21st character should be rejected
        assert!(!entry.push_char('X'));
        assert_eq!(entry.text.chars().count(), 20);
    }

    #[test]
    fn name_entry_pop_char_removes_last() {
        let mut entry = NameEntryState::new(500, 120);
        entry.push_char('H');
        entry.push_char('i');
        assert!(entry.pop_char());
        assert_eq!(entry.text, "H");
        assert!(entry.pop_char());
        assert_eq!(entry.text, "");
        // Popping empty string returns false
        assert!(!entry.pop_char());
    }

    #[test]
    fn name_entry_is_valid_checks_1_to_20_chars() {
        let mut entry = NameEntryState::new(500, 120);
        assert!(!entry.is_valid()); // 0 chars: invalid

        entry.push_char('A');
        assert!(entry.is_valid()); // 1 char: valid

        for c in "BCDEFGHIJKLMNOPQRST".chars() {
            entry.push_char(c);
        }
        assert_eq!(entry.text.chars().count(), 20);
        assert!(entry.is_valid()); // 20 chars: valid
    }
}
