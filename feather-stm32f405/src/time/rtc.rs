//! RTC (Real-Time Clock) wrapper and timestamp operations
//!
//! Provides safe access to the STM32 hardware RTC and manages the time
//! synchronization status in CCM RAM.
#![deny(unsafe_code)]
#![deny(warnings)]

use crate::ccmram::TIME_SYNCED;

use core::cell::RefCell;
use core::sync::atomic::Ordering;
use critical_section::Mutex;
use defmt::{info, Format};
use embassy_stm32::rtc::Rtc;

use super::calendar::{datetime_to_unix, unix_to_datetime};

/// Global internal RTC instance
static RTC: Mutex<RefCell<Option<Rtc>>> = Mutex::new(RefCell::new(None));

/// Timestamp with millisecond precision
#[derive(Debug, Clone, Copy, Format)]
pub struct Timestamp {
    /// Unix timestamp in seconds since epoch (1970-01-01 00:00:00 UTC)
    pub unix_secs: u64,
    /// Milliseconds component (0-999)
    pub millis: u32,
}

impl Timestamp {
    /// Create a new timestamp
    pub const fn new(unix_secs: u64, millis: u32) -> Self {
        Self { unix_secs, millis }
    }

    /// Convert from NTP timestamp (seconds since 1900-01-01)
    pub fn from_ntp(ntp_secs: u64, ntp_frac: u32) -> Self {
        /// NTP epoch offset (1900-01-01 to 1970-01-01 in seconds)
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

        let unix_secs = ntp_secs.saturating_sub(NTP_UNIX_OFFSET);
        // Convert NTP fractional part to milliseconds (2^-32 seconds)
        let millis = ((ntp_frac as u64 * 1_000) >> 32) as u32;
        Self::new(unix_secs, millis)
    }
}

/// RTC operation errors
#[derive(Debug, Clone, Copy, Format)]
pub enum RtcError {
    /// RTC not initialized
    NotInitialized,
    /// RTC hardware error
    HardwareError,
}

/// Initialize internal RTC
///
/// Must be called once during system initialization before any time operations.
pub fn initialize_rtc(rtc: Rtc) {
    critical_section::with(|cs| {
        RTC.borrow(cs).replace(Some(rtc));
    });
    info!("Internal RTC initialized");
}

/// Check if time has been synchronized with NTP
///
/// Returns `true` if at least one successful NTP sync has occurred.
/// Time read from `get_timestamp()` is only valid when this returns `true`.
#[allow(dead_code)]
pub fn is_time_synced() -> bool {
    TIME_SYNCED.load(Ordering::Acquire)
}

/// Write timestamp to internal RTC hardware
///
/// Only sets TIME_SYNCED flag if the write succeeds.
///
/// **Note**: The milliseconds component of the timestamp is not written to the RTC.
/// The RTC subsecond counter will start from 0 after the write. This is acceptable
/// because this function is primarily used during NTP synchronization, where the
/// subsecond precision comes from the network round-trip time and begins counting
/// immediately after the RTC is set.
pub fn write_rtc(timestamp: Timestamp) -> Result<(), RtcError> {
    let datetime = unix_to_datetime(timestamp.unix_secs);

    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow(cs).borrow_mut().as_mut() {
            // Only set TIME_SYNCED if set_datetime succeeds
            rtc.set_datetime(datetime)
                .map_err(|_| RtcError::HardwareError)?;
            TIME_SYNCED.store(true, Ordering::Release);
            Ok(())
        } else {
            Err(RtcError::NotInitialized)
        }
    })
}

/// Read timestamp from internal RTC hardware with millisecond granularity
///
/// Returns an error if time has not been synchronized yet.
/// Uses the STM32 RTC subsecond register (RTC_SSR) to provide millisecond precision.
pub fn read_rtc() -> Result<Timestamp, RtcError> {
    if !TIME_SYNCED.load(Ordering::Acquire) {
        return Err(RtcError::NotInitialized);
    }

    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow(cs).borrow_mut().as_mut() {
            let datetime = rtc.now().map_err(|_| RtcError::HardwareError)?;
            // Extract milliseconds from microseconds (truncated to nearest millisecond)
            // DateTime.microsecond() is guaranteed to be 0..=999_999, so division by 1000 yields 0..=999
            let millis = datetime.microsecond() / 1_000;
            let unix_secs = datetime_to_unix(datetime);
            Ok(Timestamp::new(unix_secs, millis))
        } else {
            Err(RtcError::NotInitialized)
        }
    })
}

/// Get current timestamp from internal RTC hardware
///
/// Returns `Timestamp` with `unix_secs = 0` until first sync.
pub fn get_timestamp() -> Timestamp {
    read_rtc().unwrap_or(Timestamp::new(0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntp_to_unix_conversion() {
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;
        let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET, 0);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.millis, 0);
    }

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1704067200, 500);
        assert_eq!(ts.unix_secs, 1704067200);
        assert_eq!(ts.millis, 500);
    }
}
