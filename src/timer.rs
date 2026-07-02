//! Timer module.
//!
//! Tracks elapsed game time with pause and resume support,
//! providing formatted display output in MM:SS format.

use std::time::Instant;

/// Tracks elapsed game time with pause/resume support.
pub struct GameTimer {
    /// Total elapsed milliseconds (accumulated while running).
    pub elapsed_ms: u64,
    /// Whether the timer is currently running.
    pub running: bool,
    /// The instant when the timer was last started/resumed.
    pub last_tick: Option<Instant>,
}

impl GameTimer {
    /// Creates a new timer in stopped state with zero elapsed time.
    pub fn new() -> Self {
        Self {
            elapsed_ms: 0,
            running: false,
            last_tick: None,
        }
    }

    /// Resets the timer to zero and stops it.
    pub fn reset(&mut self) {
        self.elapsed_ms = 0;
        self.running = false;
        self.last_tick = None;
    }

    /// Starts the timer from the current moment.
    pub fn start(&mut self) {
        self.running = true;
        self.last_tick = Some(Instant::now());
    }

    /// Pauses the timer, accumulating elapsed time.
    pub fn pause(&mut self) {
        if self.running {
            if let Some(last) = self.last_tick {
                self.elapsed_ms += last.elapsed().as_millis() as u64;
            }
            self.running = false;
            self.last_tick = None;
        }
    }

    /// Resumes the timer from a paused state.
    pub fn resume(&mut self) {
        if !self.running {
            self.running = true;
            self.last_tick = Some(Instant::now());
        }
    }

    /// Updates the timer. Call each frame to keep elapsed_ms current.
    pub fn update(&mut self) {
        if self.running {
            if let Some(last) = self.last_tick {
                self.elapsed_ms += last.elapsed().as_millis() as u64;
                self.last_tick = Some(Instant::now());
            }
        }
    }

    /// Returns the total elapsed time in whole seconds.
    pub fn elapsed_seconds(&self) -> u32 {
        let total_ms = if self.running {
            self.elapsed_ms
                + self
                    .last_tick
                    .map(|t| t.elapsed().as_millis() as u64)
                    .unwrap_or(0)
        } else {
            self.elapsed_ms
        };
        (total_ms / 1000) as u32
    }

    /// Formats the elapsed time as "MM:SS" with zero-padding.
    pub fn format_display(&self) -> String {
        let secs = self.elapsed_seconds();
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

impl Default for GameTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn new_timer_is_stopped_with_zero_elapsed() {
        let timer = GameTimer::new();
        assert_eq!(timer.elapsed_ms, 0);
        assert!(!timer.running);
        assert!(timer.last_tick.is_none());
    }

    #[test]
    fn start_sets_running_and_last_tick() {
        let mut timer = GameTimer::new();
        timer.start();
        assert!(timer.running);
        assert!(timer.last_tick.is_some());
    }

    #[test]
    fn pause_stops_timer_and_accumulates_time() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(50));
        timer.pause();

        assert!(!timer.running);
        assert!(timer.last_tick.is_none());
        // Should have accumulated at least some time
        assert!(timer.elapsed_ms > 0);
    }

    #[test]
    fn pause_when_already_paused_is_idempotent() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(20));
        timer.pause();
        let elapsed_after_first_pause = timer.elapsed_ms;

        // Pausing again should not change anything
        timer.pause();
        assert_eq!(timer.elapsed_ms, elapsed_after_first_pause);
        assert!(!timer.running);
        assert!(timer.last_tick.is_none());
    }

    #[test]
    fn resume_when_already_running_is_idempotent() {
        let mut timer = GameTimer::new();
        timer.start();
        let last_tick_before = timer.last_tick;

        // Resume on already running timer should not change state
        timer.resume();
        assert!(timer.running);
        // last_tick should remain unchanged (same Instant)
        assert_eq!(timer.last_tick, last_tick_before);
    }

    #[test]
    fn resume_restarts_timing_from_pause() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(20));
        timer.pause();
        let elapsed_at_pause = timer.elapsed_ms;

        timer.resume();
        assert!(timer.running);
        assert!(timer.last_tick.is_some());
        // Elapsed should not have changed just from resuming
        assert_eq!(timer.elapsed_ms, elapsed_at_pause);

        // After some more time and an update, elapsed should increase
        thread::sleep(Duration::from_millis(20));
        timer.update();
        assert!(timer.elapsed_ms > elapsed_at_pause);
    }

    #[test]
    fn update_accumulates_time_while_running() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(30));
        timer.update();

        assert!(timer.elapsed_ms > 0);
        assert!(timer.running);
    }

    #[test]
    fn update_does_nothing_while_stopped() {
        let mut timer = GameTimer::new();
        // Timer is not running
        timer.update();
        assert_eq!(timer.elapsed_ms, 0);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(20));
        timer.update();
        assert!(timer.elapsed_ms > 0);

        timer.reset();
        assert_eq!(timer.elapsed_ms, 0);
        assert!(!timer.running);
        assert!(timer.last_tick.is_none());
    }

    #[test]
    fn elapsed_seconds_returns_whole_seconds() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 2500; // 2.5 seconds
        assert_eq!(timer.elapsed_seconds(), 2);
    }

    #[test]
    fn elapsed_seconds_includes_current_running_segment() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 1000; // 1 second accumulated
        timer.start();
        thread::sleep(Duration::from_millis(50));
        // Should include the currently running segment
        let secs = timer.elapsed_seconds();
        assert!(secs >= 1);
    }

    #[test]
    fn elapsed_seconds_zero_when_new() {
        let timer = GameTimer::new();
        assert_eq!(timer.elapsed_seconds(), 0);
    }

    #[test]
    fn format_display_zero() {
        let timer = GameTimer::new();
        assert_eq!(timer.format_display(), "00:00");
    }

    #[test]
    fn format_display_seconds_only() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 5000; // 5 seconds
        assert_eq!(timer.format_display(), "00:05");
    }

    #[test]
    fn format_display_minutes_and_seconds() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 65000; // 1 minute 5 seconds
        assert_eq!(timer.format_display(), "01:05");
    }

    #[test]
    fn format_display_large_time() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 3599_000; // 59 minutes 59 seconds
        assert_eq!(timer.format_display(), "59:59");
    }

    #[test]
    fn format_display_over_one_hour() {
        let mut timer = GameTimer::new();
        timer.elapsed_ms = 3661_000; // 61 minutes 1 second
        assert_eq!(timer.format_display(), "61:01");
    }

    #[test]
    fn timer_does_not_advance_while_paused() {
        let mut timer = GameTimer::new();
        timer.start();
        thread::sleep(Duration::from_millis(20));
        timer.pause();
        let elapsed_at_pause = timer.elapsed_ms;

        // Wait while paused
        thread::sleep(Duration::from_millis(50));
        timer.update();

        // Should not have advanced
        assert_eq!(timer.elapsed_ms, elapsed_at_pause);
    }

    #[test]
    fn pause_resume_accumulates_correctly() {
        let mut timer = GameTimer::new();

        // First segment
        timer.start();
        thread::sleep(Duration::from_millis(30));
        timer.pause();
        let after_first = timer.elapsed_ms;
        assert!(after_first > 0);

        // Gap while paused - should not count
        thread::sleep(Duration::from_millis(50));

        // Second segment
        timer.resume();
        thread::sleep(Duration::from_millis(30));
        timer.pause();
        let after_second = timer.elapsed_ms;

        // Second segment should only add ~30ms, not the 50ms pause gap
        assert!(after_second > after_first);
        assert!(after_second < after_first + 80); // generous upper bound
    }

    #[test]
    fn default_is_same_as_new() {
        let timer = GameTimer::default();
        assert_eq!(timer.elapsed_ms, 0);
        assert!(!timer.running);
        assert!(timer.last_tick.is_none());
    }
}
