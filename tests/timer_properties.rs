//! Property-based tests for timer formatting.
//!
//! Feature: xmahjong, Property 15: Timer Formatting

use xmahjong::timer::GameTimer;
use proptest::prelude::*;

// Feature: xmahjong, Property 15: Timer Formatting
// Validates: Requirements 8.1
proptest! {
    #[test]
    fn prop_timer_format_produces_mm_ss_and_round_trips(elapsed_seconds in 0u32..=99999) {
        // Set up a stopped timer with a specific number of elapsed seconds
        let mut timer = GameTimer::new();
        timer.elapsed_ms = elapsed_seconds as u64 * 1000;

        let formatted = timer.format_display();

        // Assert the format matches "MM:SS" pattern (at least 2 digits each, colon separated)
        let parts: Vec<&str> = formatted.split(':').collect();
        prop_assert_eq!(parts.len(), 2, "Expected format 'MM:SS', got '{}'", formatted);

        let mm_str = parts[0];
        let ss_str = parts[1];

        // Minutes part must be at least 2 digits (zero-padded)
        prop_assert!(mm_str.len() >= 2, "Minutes part '{}' must be at least 2 digits", mm_str);
        // Seconds part must be exactly 2 digits (0-59 zero-padded)
        prop_assert_eq!(ss_str.len(), 2, "Seconds part '{}' must be exactly 2 digits", ss_str);

        // Both parts must be valid numeric values
        let minutes: u32 = mm_str.parse().map_err(|e| {
            proptest::test_runner::TestCaseError::Fail(
                format!("Failed to parse minutes '{}': {}", mm_str, e).into()
            )
        })?;
        let seconds: u32 = ss_str.parse().map_err(|e| {
            proptest::test_runner::TestCaseError::Fail(
                format!("Failed to parse seconds '{}': {}", ss_str, e).into()
            )
        })?;

        // Seconds must be in range 0..59
        prop_assert!(seconds < 60, "Seconds value {} must be less than 60", seconds);

        // Round-trip: parse(format(n)) == n
        let reconstructed = minutes * 60 + seconds;
        prop_assert_eq!(
            reconstructed, elapsed_seconds,
            "Round-trip failed: format({}) = '{}', parse back = {}",
            elapsed_seconds, formatted, reconstructed
        );
    }
}
