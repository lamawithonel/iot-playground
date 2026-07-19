//! Battery-backed wall-clock abstraction

use crate::time::{RtcError, Timestamp};

/// Battery-backed wall-clock abstraction.
///
/// Implementations back this with hardware RTC registers or, for
/// host tests, an in-memory mock (see
/// [`crate::test_support::MockRtc`]).
///
/// ```rust,ignore
/// fn log_boot_time<R: Rtc>(rtc: &mut R) {
///     match rtc.now() {
///         Ok(ts) => defmt::info!("boot time: {}", ts.unix_secs),
///         Err(_) => defmt::warn!("clock not yet synchronized"),
///     }
/// }
/// ```
pub trait Rtc {
    /// Read the current wall-clock time.
    ///
    /// Returns `Err(RtcError::NotInitialized)` until the clock has
    /// been set at least once since power-up.  Sub-second precision
    /// is optional-- implementations MAY return `micros == 0`.
    fn now(&mut self) -> Result<Timestamp, RtcError>;

    /// Set the wall-clock time.
    ///
    /// Marks the clock synced on success.
    fn set(&mut self, timestamp: Timestamp) -> Result<(), RtcError>;

    /// Whether wall-clock time has been synchronized (e.g., via
    /// SNTP) since power-up.
    fn is_synced(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockRtc;

    #[test]
    fn fresh_mock_is_unsynced() {
        let mut rtc = MockRtc::new();
        assert!(!rtc.is_synced());
        assert_eq!(rtc.now(), Err(RtcError::NotInitialized));
    }

    #[test]
    fn set_then_now_round_trips() {
        let mut rtc = MockRtc::new();
        let t0 = Timestamp::new(1_000, 0);
        assert_eq!(rtc.set(t0), Ok(()));
        assert!(rtc.is_synced());
        assert_eq!(rtc.now(), Ok(t0));
    }

    #[test]
    fn advance_secs_moves_now_forward() {
        let mut rtc = MockRtc::new();
        let t0 = Timestamp::new(1_000, 0);
        rtc.set(t0).unwrap();
        rtc.advance_secs(5);
        assert_eq!(rtc.now(), Ok(Timestamp::new(1_005, 0)));
    }

    #[test]
    fn injected_failure_recovers_after_one_call() {
        let mut rtc = MockRtc::new();
        rtc.set(Timestamp::new(1_000, 0)).unwrap();

        rtc.fail_next(RtcError::HardwareError);
        assert_eq!(rtc.now(), Err(RtcError::HardwareError));
        // Recovered: the next call succeeds.
        assert_eq!(rtc.now(), Ok(Timestamp::new(1_000, 0)));
    }

    #[test]
    fn failed_set_does_not_mark_synced() {
        let mut rtc = MockRtc::new();
        rtc.fail_next(RtcError::HardwareError);
        assert_eq!(
            rtc.set(Timestamp::new(1_000, 0)),
            Err(RtcError::HardwareError)
        );
        assert!(!rtc.is_synced());
        assert_eq!(rtc.now(), Err(RtcError::NotInitialized));
    }
}
