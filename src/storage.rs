//! Storage module.
//!
//! Handles persistence of leaderboard scores and game settings
//! to JSON files on disk.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Returns the storage directory path.
/// Checks `$SNAP_USER_DATA` first, then uses the platform-appropriate location:
/// - macOS: `~/Library/Application Support/lmahjong/`
/// - Linux: `~/.local/share/lmahjong/`
fn storage_dir() -> PathBuf {
    if let Ok(snap_dir) = std::env::var("SNAP_USER_DATA") {
        PathBuf::from(snap_dir)
    } else {
        dirs_fallback()
    }
}

/// Platform-appropriate storage directory.
/// - macOS: `~/Library/Application Support/lmahjong/`
/// - Linux/other: `~/.local/share/lmahjong/`
fn dirs_fallback() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    if cfg!(target_os = "macos") {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("lmahjong")
    } else {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("lmahjong")
    }
}

/// A single leaderboard entry recording a completed game.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardEntry {
    /// Player name (1-20 characters).
    pub name: String,
    /// Final game score.
    pub score: u32,
    /// Time to complete in seconds.
    pub time_seconds: u32,
    /// Date of completion in ISO 8601 format.
    pub date: String,
}

impl LeaderboardEntry {
    /// Validates the entry's name. Returns true if name is 1-20 characters.
    pub fn is_valid_name(name: &str) -> bool {
        let len = name.chars().count();
        (1..=20).contains(&len)
    }
}

/// Leaderboard holding up to 10 entries sorted descending by score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Leaderboard {
    pub entries: Vec<LeaderboardEntry>,
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl Leaderboard {
    /// Loads the leaderboard from disk.
    /// Returns a default (empty) leaderboard on any read or parse error.
    pub fn load() -> Self {
        let path = storage_dir().join("leaderboard.json");
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Saves the leaderboard to disk.
    /// Creates directories as needed. Logs errors to stderr but does not crash.
    pub fn save(&self) {
        let dir = storage_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("lmahjong: failed to create storage directory {:?}: {}", dir, e);
            return;
        }
        let path = dir.join("leaderboard.json");
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("lmahjong: failed to write leaderboard to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                eprintln!("lmahjong: failed to serialize leaderboard: {}", e);
            }
        }
    }

    /// Returns true if the given score qualifies for the top 10.
    /// Qualifies if fewer than 10 entries exist, or score is higher than the lowest entry.
    pub fn qualifies(&self, score: u32) -> bool {
        if self.entries.len() < 10 {
            return true;
        }
        // entries are sorted descending, so the lowest is last
        self.entries.last().map_or(true, |lowest| score > lowest.score)
    }

    /// Inserts a new entry, maintaining descending sort order and capping at 10 entries.
    pub fn insert(&mut self, entry: LeaderboardEntry) {
        self.entries.push(entry);
        self.entries.sort_by(|a, b| b.score.cmp(&a.score));
        self.entries.truncate(10);
    }
}

/// Game settings that persist across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub muted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { muted: false }
    }
}

impl Settings {
    /// Loads settings from disk.
    /// Returns default settings on any read or parse error.
    pub fn load() -> Self {
        let path = storage_dir().join("settings.json");
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Saves settings to disk.
    /// Creates directories as needed. Logs errors to stderr but does not crash.
    pub fn save(&self) {
        let dir = storage_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("lmahjong: failed to create storage directory {:?}: {}", dir, e);
            return;
        }
        let path = dir.join("settings.json");
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("lmahjong: failed to write settings to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                eprintln!("lmahjong: failed to serialize settings: {}", e);
            }
        }
    }
}

/// Represents a saved game state that can be resumed later.
///
/// Stores only the minimal data needed to reconstruct the game:
/// board tile face IDs (None for removed), undo stack, timer, and score tracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedGame {
    /// For each of the 144 positions: `Some(face_id)` if a tile is present, `None` if removed.
    pub tiles: Vec<Option<u8>>,
    /// Undo stack: each entry records (pos_a, face_a, pos_b, face_b).
    pub undo_stack: Vec<(usize, u8, usize, u8)>,
    /// Elapsed time in milliseconds at the time of save.
    pub elapsed_ms: u64,
    /// Number of hints used.
    pub hints_used: u32,
    /// Number of shuffles used.
    pub shuffles_used: u32,
    /// Number of shuffles remaining.
    pub shuffles_remaining: u8,
    /// Number of pairs matched so far.
    pub pairs_matched: u32,
    /// Current level.
    #[serde(default = "default_level")]
    pub level: u32,
}

/// Default level value for backwards compatibility with old save files.
fn default_level() -> u32 {
    1
}

impl SavedGame {
    /// Loads a saved game from disk. Returns None if no save exists or it's corrupt.
    pub fn load() -> Option<Self> {
        let path = storage_dir().join("savegame.json");
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).ok(),
            Err(_) => None,
        }
    }

    /// Saves the game state to disk.
    pub fn save(&self) {
        let dir = storage_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("lmahjong: failed to create storage directory {:?}: {}", dir, e);
            return;
        }
        let path = dir.join("savegame.json");
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("lmahjong: failed to write savegame to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                eprintln!("lmahjong: failed to serialize savegame: {}", e);
            }
        }
    }

    /// Deletes the saved game file (e.g., after successfully loading it).
    pub fn delete() {
        let path = storage_dir().join("savegame.json");
        let _ = fs::remove_file(&path);
    }

    /// Returns true if a saved game file exists on disk.
    pub fn exists() -> bool {
        let path = storage_dir().join("savegame.json");
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_maintains_sorted_order() {
        let mut lb = Leaderboard::default();
        lb.insert(LeaderboardEntry {
            name: "Alice".to_string(),
            score: 500,
            time_seconds: 300,
            date: "2024-01-01".to_string(),
        });
        lb.insert(LeaderboardEntry {
            name: "Bob".to_string(),
            score: 800,
            time_seconds: 200,
            date: "2024-01-02".to_string(),
        });
        lb.insert(LeaderboardEntry {
            name: "Carol".to_string(),
            score: 650,
            time_seconds: 250,
            date: "2024-01-03".to_string(),
        });

        assert_eq!(lb.entries[0].score, 800);
        assert_eq!(lb.entries[1].score, 650);
        assert_eq!(lb.entries[2].score, 500);
    }

    #[test]
    fn insert_caps_at_10_entries() {
        let mut lb = Leaderboard::default();
        for i in 0..15 {
            lb.insert(LeaderboardEntry {
                name: format!("Player{}", i),
                score: i * 100,
                time_seconds: 100,
                date: "2024-01-01".to_string(),
            });
        }

        assert_eq!(lb.entries.len(), 10);
        // The top 10 should be scores 1400, 1300, ..., 500
        assert_eq!(lb.entries[0].score, 1400);
        assert_eq!(lb.entries[9].score, 500);
    }

    #[test]
    fn qualifies_correctly_checks_threshold() {
        let mut lb = Leaderboard::default();

        // Empty leaderboard: any score qualifies
        assert!(lb.qualifies(0));
        assert!(lb.qualifies(100));

        // Fill with 10 entries, lowest score = 100
        for i in 1..=10 {
            lb.insert(LeaderboardEntry {
                name: format!("P{}", i),
                score: i * 100,
                time_seconds: 100,
                date: "2024-01-01".to_string(),
            });
        }

        assert_eq!(lb.entries.len(), 10);
        // Score higher than lowest (100) qualifies
        assert!(lb.qualifies(150));
        // Score equal to lowest does NOT qualify (must be strictly greater)
        assert!(!lb.qualifies(100));
        // Score lower than lowest does NOT qualify
        assert!(!lb.qualifies(50));
    }

    #[test]
    fn name_validation_1_to_20_chars() {
        // Empty name is invalid
        assert!(!LeaderboardEntry::is_valid_name(""));

        // 1 character is valid
        assert!(LeaderboardEntry::is_valid_name("A"));

        // 20 characters is valid
        assert!(LeaderboardEntry::is_valid_name("12345678901234567890"));

        // 21 characters is invalid
        assert!(!LeaderboardEntry::is_valid_name("123456789012345678901"));
    }

    #[test]
    fn settings_default_mute_is_false() {
        let settings = Settings::default();
        assert!(!settings.muted);
    }

    #[test]
    fn qualifies_with_fewer_than_10_entries() {
        let mut lb = Leaderboard::default();
        lb.insert(LeaderboardEntry {
            name: "One".to_string(),
            score: 900,
            time_seconds: 100,
            date: "2024-01-01".to_string(),
        });

        // Only 1 entry, so any score qualifies
        assert!(lb.qualifies(0));
        assert!(lb.qualifies(1000));
    }

    #[test]
    fn insert_entry_with_same_score_keeps_sorted() {
        let mut lb = Leaderboard::default();
        lb.insert(LeaderboardEntry {
            name: "A".to_string(),
            score: 500,
            time_seconds: 100,
            date: "2024-01-01".to_string(),
        });
        lb.insert(LeaderboardEntry {
            name: "B".to_string(),
            score: 500,
            time_seconds: 200,
            date: "2024-01-02".to_string(),
        });

        assert_eq!(lb.entries.len(), 2);
        // Both should be present with score 500
        assert_eq!(lb.entries[0].score, 500);
        assert_eq!(lb.entries[1].score, 500);
    }
}
