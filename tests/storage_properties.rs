//! Property-based tests for leaderboard storage.

use proptest::prelude::*;

use xmahjong::storage::{Leaderboard, LeaderboardEntry};

// Feature: xmahjong, Property 17: Leaderboard Invariants
//
// **Validates: Requirements 8.3**
//
// For any sequence of insertions, leaderboard has at most 10 entries,
// sorted descending by score, and contains only the top 10 scores ever inserted.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_17_leaderboard_invariants(
        scores in prop::collection::vec(0u32..100000, 1..50),
    ) {
        let mut leaderboard = Leaderboard::default();

        for (i, &score) in scores.iter().enumerate() {
            leaderboard.insert(LeaderboardEntry {
                name: format!("Player{}", i),
                score,
                time_seconds: 100,
                hints_used: 0,
                shuffles_used: 0,
                undos_used: 0,
                date: "2024-01-01".to_string(),
            });
        }

        let entries = &leaderboard.entries;

        // Invariant 1: At most 10 entries
        prop_assert!(
            entries.len() <= 10,
            "Leaderboard has {} entries, expected at most 10",
            entries.len()
        );

        // Invariant 2: Sorted descending by score
        for i in 0..entries.len().saturating_sub(1) {
            prop_assert!(
                entries[i].score >= entries[i + 1].score,
                "Leaderboard not sorted descending: entries[{}].score={} < entries[{}].score={}",
                i, entries[i].score, i + 1, entries[i + 1].score
            );
        }

        // Invariant 3: Contains only top 10 highest scores from input
        let mut sorted_scores = scores.clone();
        sorted_scores.sort_unstable_by(|a, b| b.cmp(a));
        sorted_scores.truncate(10);

        let leaderboard_scores: Vec<u32> = entries.iter().map(|e| e.score).collect();
        prop_assert_eq!(
            leaderboard_scores, sorted_scores,
            "Leaderboard scores do not match top 10 from input"
        );
    }
}

use xmahjong::storage::Settings;

// Feature: xmahjong, Property 18: Mute State Persistence Round-Trip
//
// **Validates: Requirements 11.5**
//
// For any mute state (true or false), serializing settings and then
// deserializing them preserves the mute state exactly.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn property_18_mute_state_persistence_round_trip(
        muted in proptest::bool::ANY,
    ) {
        let settings = Settings { muted };

        // Serialize to JSON (simulates save)
        let json = serde_json::to_string(&settings)
            .expect("Settings serialization should not fail");

        // Deserialize from JSON (simulates load)
        let loaded: Settings = serde_json::from_str(&json)
            .expect("Settings deserialization should not fail");

        // The mute state must be preserved exactly
        prop_assert_eq!(
            loaded.muted, settings.muted,
            "Mute state not preserved: saved {:?}, loaded {:?}",
            settings.muted, loaded.muted
        );
    }
}
