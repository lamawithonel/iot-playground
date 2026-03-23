//! Sensor conditioning state tracking
//!
//! Generic state machine for tracking sensor warmup periods.
//! [`ConditioningState`] tracks approximate elapsed time and exposes
//! readiness predicates for each sensor.
//!
//! SEN66-specific warmup constants and milestone tracking require
//! the `sen66` feature:
//!
//! | Sensor | Warmup  |
//! |--------|---------|
//! | Temp/RH| ~8 s    |
//! | PM     | ~2 min  |
//! | CO₂    | ~3 min  |
//! | VOC    | ~60 s   |
//! | NOx    | ~10 min |

/// Temp/RH sensor (SHT4x) warmup time in seconds
#[cfg(feature = "sen66")]
pub const TEMP_RH_WARMUP_SECS: u64 = 8;

/// PM sensor (SPS6x) warmup time in seconds
#[cfg(feature = "sen66")]
pub const PM_WARMUP_SECS: u64 = 120;

/// CO₂ sensor (SCD41) warmup time in seconds
#[cfg(feature = "sen66")]
pub const CO2_WARMUP_SECS: u64 = 180;

/// VOC index (SGP41) warmup time in seconds
#[cfg(feature = "sen66")]
pub const VOC_WARMUP_SECS: u64 = 60;

/// NOx index (SGP41) warmup time in seconds
#[cfg(feature = "sen66")]
pub const NOX_WARMUP_SECS: u64 = 600;

/// Milestone flags returned by [`ConditioningState::check_milestones`]
///
/// Each flag is `true` exactly once — the first time the sensor
/// crosses its warmup threshold.  The BSP uses these to drive
/// `defmt::info!` logging without pulling defmt into core.
#[cfg(feature = "sen66")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MilestoneFlags {
    /// Temp/RH conditioning just completed
    pub temp_rh: bool,
    /// PM conditioning just completed
    pub pm: bool,
    /// CO₂ conditioning just completed
    pub co2: bool,
    /// VOC conditioning just completed
    pub voc: bool,
    /// NOx conditioning just completed (all sensors ready)
    pub nox: bool,
}

/// Tracks sensor conditioning state across reads
///
/// Created once in the sensor task.  The generic state machine
/// tracks elapsed time via configurable intervals.  SEN66-specific
/// warmup thresholds and milestone tracking require the `sen66`
/// feature.
pub struct ConditioningState {
    /// Sample interval in seconds (from build config)
    interval_secs: u64,
    /// Initial delay before first sample, in seconds
    initial_delay_secs: u64,
    /// Approximate elapsed time since sensor start, in seconds.
    /// Zero indicates no reads have occurred yet.
    elapsed_secs: u64,
    /// Number of reads recorded (used for first-read detection)
    read_count: u32,
    /// Whether Temp/RH milestone has been reported
    #[cfg(feature = "sen66")]
    temp_rh_logged: bool,
    /// Whether PM milestone has been reported
    #[cfg(feature = "sen66")]
    pm_logged: bool,
    /// Whether CO₂ milestone has been reported
    #[cfg(feature = "sen66")]
    co2_logged: bool,
    /// Whether VOC milestone has been reported
    #[cfg(feature = "sen66")]
    voc_logged: bool,
    /// Whether NOx milestone has been reported
    #[cfg(feature = "sen66")]
    nox_logged: bool,
}

impl ConditioningState {
    /// Create a new state tracker
    ///
    /// `interval_secs` is the sample period (e.g., 5 for debug,
    /// 60 for release).  `initial_delay_secs` is the startup
    /// delay before the first read.
    pub const fn new(interval_secs: u64, initial_delay_secs: u64) -> Self {
        Self {
            interval_secs,
            initial_delay_secs,
            elapsed_secs: 0,
            read_count: 0,
            #[cfg(feature = "sen66")]
            temp_rh_logged: false,
            #[cfg(feature = "sen66")]
            pm_logged: false,
            #[cfg(feature = "sen66")]
            co2_logged: false,
            #[cfg(feature = "sen66")]
            voc_logged: false,
            #[cfg(feature = "sen66")]
            nox_logged: false,
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

    /// Whether Temp/RH readings are reliable
    #[cfg(feature = "sen66")]
    pub fn temp_rh_ready(&self) -> bool {
        self.elapsed_secs >= TEMP_RH_WARMUP_SECS
    }

    /// Whether PM readings are reliable
    #[cfg(feature = "sen66")]
    pub fn pm_ready(&self) -> bool {
        self.elapsed_secs >= PM_WARMUP_SECS
    }

    /// Whether CO₂ readings are reliable
    #[cfg(feature = "sen66")]
    pub fn co2_ready(&self) -> bool {
        self.elapsed_secs >= CO2_WARMUP_SECS
    }

    /// Whether VOC index is reliable
    #[cfg(feature = "sen66")]
    pub fn voc_ready(&self) -> bool {
        self.elapsed_secs >= VOC_WARMUP_SECS
    }

    /// Whether NOx index is reliable
    #[cfg(feature = "sen66")]
    pub fn nox_ready(&self) -> bool {
        self.elapsed_secs >= NOX_WARMUP_SECS
    }

    /// Check and consume milestone transitions
    ///
    /// Returns flags for milestones that just became true.  Each
    /// flag fires exactly once.  Call after [`record_read`].
    #[cfg(feature = "sen66")]
    pub fn check_milestones(&mut self) -> MilestoneFlags {
        let mut flags = MilestoneFlags::default();

        if !self.temp_rh_logged && self.temp_rh_ready() {
            flags.temp_rh = true;
            self.temp_rh_logged = true;
        }
        if !self.voc_logged && self.voc_ready() {
            flags.voc = true;
            self.voc_logged = true;
        }
        if !self.pm_logged && self.pm_ready() {
            flags.pm = true;
            self.pm_logged = true;
        }
        if !self.co2_logged && self.co2_ready() {
            flags.co2 = true;
            self.co2_logged = true;
        }
        if !self.nox_logged && self.nox_ready() {
            flags.nox = true;
            self.nox_logged = true;
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conditioning_first_read() {
        let mut state = ConditioningState::new(5, 2);
        assert_eq!(state.elapsed_secs(), 0);

        let elapsed = state.record_read();
        assert_eq!(elapsed, 2); // initial_delay_secs
    }

    #[test]
    fn test_conditioning_progression() {
        let mut state = ConditioningState::new(5, 2);
        state.record_read(); // 2s
        state.record_read(); // 7s
        state.record_read(); // 12s
        assert_eq!(state.elapsed_secs(), 12);
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_temp_rh_ready_timing() {
        let mut state = ConditioningState::new(5, 2);
        state.record_read(); // 2s
        assert!(!state.temp_rh_ready());

        state.record_read(); // 7s
        assert!(!state.temp_rh_ready());

        state.record_read(); // 12s
        assert!(state.temp_rh_ready());
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_voc_ready_timing() {
        let mut state = ConditioningState::new(10, 2);
        // Advance to just under 60s
        for _ in 0..6 {
            state.record_read(); // 2, 12, 22, 32, 42, 52
        }
        assert!(!state.voc_ready());

        state.record_read(); // 62s
        assert!(state.voc_ready());
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_nox_ready_timing() {
        let mut state = ConditioningState::new(60, 2);
        // 10 reads at 60s interval: 2, 62, 122, ..., 542
        for _ in 0..10 {
            state.record_read();
        }
        assert!(!state.nox_ready()); // 542s < 600s

        state.record_read(); // 602s
        assert!(state.nox_ready());
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_milestone_fires_once() {
        let mut state = ConditioningState::new(5, 2);
        state.record_read(); // 2s
        state.record_read(); // 7s
        state.record_read(); // 12s — temp_rh should fire

        let flags = state.check_milestones();
        assert!(flags.temp_rh);

        // Second call should not fire again
        let flags = state.check_milestones();
        assert!(!flags.temp_rh);
    }

    #[cfg(feature = "sen66")]
    #[test]
    fn test_all_milestones_eventually() {
        let mut state = ConditioningState::new(1, 1);
        let mut got_nox = false;

        for _ in 0..650 {
            state.record_read();
            let flags = state.check_milestones();
            if flags.nox {
                got_nox = true;
                break;
            }
        }
        assert!(got_nox, "NOx milestone never fired");
    }

    #[test]
    fn test_zero_initial_delay() {
        let mut state = ConditioningState::new(5, 0);

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
}
