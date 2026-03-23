//! Sensor conditioning state tracking
//!
//! Generic state machine for tracking sensor warmup periods.
//! [`ConditioningState`] is parameterized by `N` conditioning phases,
//! each with a threshold in seconds.  The state machine tracks
//! elapsed time and exposes readiness predicates and fire-once
//! milestone transitions.
//!
//! Sensor-specific phase indices and thresholds are defined in
//! their own modules (e.g., [`super::sen66`]).

/// Tracks sensor conditioning state across reads
///
/// Generic over `N` warmup phases.  Each phase has a threshold
/// in seconds; the state machine tracks which phases have been
/// reached and fires milestone flags exactly once per phase.
///
/// `ConditioningState<0>` is valid for sensors with no warmup
/// requirements.
pub struct ConditioningState<const N: usize> {
    /// Sample interval in seconds (from build config)
    interval_secs: u64,
    /// Initial delay before first sample, in seconds
    initial_delay_secs: u64,
    /// Approximate elapsed time since sensor start, in seconds.
    /// Zero indicates no reads have occurred yet.
    elapsed_secs: u64,
    /// Number of reads recorded (used for first-read detection)
    read_count: u32,
    /// Per-phase warmup thresholds in seconds
    thresholds: [u64; N],
    /// Per-phase milestone-logged flags
    logged: [bool; N],
}

impl<const N: usize> ConditioningState<N> {
    /// Create a new state tracker
    ///
    /// `interval_secs` is the sample period (e.g., 5 for debug,
    /// 60 for release).  `initial_delay_secs` is the startup
    /// delay before the first read.  `thresholds` defines the
    /// warmup time in seconds for each conditioning phase.
    pub const fn new(interval_secs: u64, initial_delay_secs: u64, thresholds: [u64; N]) -> Self {
        Self {
            interval_secs,
            initial_delay_secs,
            elapsed_secs: 0,
            read_count: 0,
            thresholds,
            logged: [false; N],
        }
    }

    /// Record a successful read and return elapsed seconds
    ///
    /// The first call records `initial_delay_secs`; subsequent
    /// calls add `interval_secs`.
    pub fn record_read(&mut self) -> u64 {
        if self.read_count == 0 {
            self.elapsed_secs = self.initial_delay_secs;
        } else {
            self.elapsed_secs = self.elapsed_secs.saturating_add(self.interval_secs);
        }
        self.read_count = self.read_count.saturating_add(1);
        self.elapsed_secs
    }

    /// Approximate elapsed seconds since sensor start
    pub fn elapsed_secs(&self) -> u64 {
        self.elapsed_secs
    }

    /// Whether the given conditioning phase has reached its
    /// warmup threshold
    ///
    /// Returns `false` for out-of-bounds phase indices (safe
    /// for embedded — no panic).
    pub fn ready(&self, phase: usize) -> bool {
        self.thresholds
            .get(phase)
            .is_some_and(|&t| self.elapsed_secs >= t)
    }

    /// Check and consume milestone transitions
    ///
    /// Returns an array of flags.  Each element is `true`
    /// exactly once — the first time that phase crosses its
    /// warmup threshold.  Call after [`record_read`].
    pub fn check_milestones(&mut self) -> [bool; N] {
        let mut flags = [false; N];
        for (i, flag) in flags.iter_mut().enumerate() {
            if !self.logged[i] && self.elapsed_secs >= self.thresholds[i] {
                *flag = true;
                self.logged[i] = true;
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conditioning_first_read() {
        let mut state = ConditioningState::<0>::new(5, 2, []);
        assert_eq!(state.elapsed_secs(), 0);

        let elapsed = state.record_read();
        assert_eq!(elapsed, 2); // initial_delay_secs
    }

    #[test]
    fn test_conditioning_progression() {
        let mut state = ConditioningState::<0>::new(5, 2, []);
        state.record_read(); // 2s
        state.record_read(); // 7s
        state.record_read(); // 12s
        assert_eq!(state.elapsed_secs(), 12);
    }

    #[test]
    fn test_zero_initial_delay() {
        let mut state = ConditioningState::<0>::new(5, 0, []);

        // First read: elapsed = initial_delay_secs = 0
        let elapsed = state.record_read();
        assert_eq!(elapsed, 0);

        // Second read: elapsed = 0 + interval = 5
        let elapsed = state.record_read();
        assert_eq!(elapsed, 5);

        // Third read: elapsed = 5 + 5 = 10
        let elapsed = state.record_read();
        assert_eq!(elapsed, 10);
    }

    #[test]
    fn test_ready_with_thresholds() {
        let mut state = ConditioningState::new(5, 0, [3, 10]);
        assert!(!state.ready(0));
        assert!(!state.ready(1));

        state.record_read(); // 0s
        state.record_read(); // 5s
        assert!(state.ready(0)); // 5 >= 3
        assert!(!state.ready(1)); // 5 < 10

        state.record_read(); // 10s
        assert!(state.ready(0));
        assert!(state.ready(1)); // 10 >= 10
    }

    #[test]
    fn test_ready_out_of_bounds() {
        let state = ConditioningState::new(1, 0, [5]);
        assert!(!state.ready(99)); // out of bounds → false
    }

    #[test]
    fn test_milestone_fires_once() {
        let mut state = ConditioningState::new(1, 0, [3, 7]);
        for _ in 0..4 {
            state.record_read();
        }
        // 3s: phase 0 should fire
        let flags = state.check_milestones();
        assert!(flags[0]);
        assert!(!flags[1]);

        // Second call — should not fire again
        let flags = state.check_milestones();
        assert!(!flags[0]);
        assert!(!flags[1]);
    }

    #[test]
    fn test_all_milestones_eventually() {
        let mut state = ConditioningState::new(1, 0, [2, 5, 10]);
        let mut last_fired = [false; 3];

        for _ in 0..15 {
            state.record_read();
            let flags = state.check_milestones();
            for i in 0..3 {
                if flags[i] {
                    last_fired[i] = true;
                }
            }
        }
        assert!(last_fired[0], "phase 0 never fired");
        assert!(last_fired[1], "phase 1 never fired");
        assert!(last_fired[2], "phase 2 never fired");
    }
}
