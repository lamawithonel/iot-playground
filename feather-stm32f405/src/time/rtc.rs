//! RTC (Real-Time Clock) operations
//!
//! Provides utilities for working with the STM32 hardware RTC and manages
//! time synchronization status in CCM RAM.
//!
//! The RTC itself is stored in RTIC's Shared struct and accessed via RTIC's
//! resource management. This module provides helper functions and the sync status.
#![deny(unsafe_code)]
#![deny(warnings)]

use crate::ccmram::{CACHED_MICROS, CACHED_UNIX_SECS_HI, CACHED_UNIX_SECS_LO, TIME_SYNCED};
use core::sync::atomic::Ordering;
use defmt::Format;
use embassy_stm32::rtc::{DateTime, Rtc};

use super::calendar::{datetime_to_unix, unix_to_datetime};

/// Timestamp with microsecond precision
///
/// Stores Unix timestamp (seconds since 1970-01-01 00:00:00 UTC) plus
/// microsecond component for subsecond precision.
#[derive(Debug, Clone, Copy, Format)]
pub struct Timestamp {
    /// Unix timestamp in seconds since epoch (1970-01-01 00:00:00 UTC)
    pub unix_secs: u64,
    /// Microseconds component (0-999,999)
    pub micros: u32,
}

impl Timestamp {
    /// Create a new timestamp
    pub const fn new(unix_secs: u64, micros: u32) -> Self {
        Self { unix_secs, micros }
    }

    /// Convert from NTP timestamp (seconds since 1900-01-01)
    pub fn from_ntp(ntp_secs: u64, ntp_frac: u32) -> Self {
        /// NTP epoch offset (1900-01-01 to 1970-01-01 in seconds)
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

        let unix_secs = ntp_secs.saturating_sub(NTP_UNIX_OFFSET);
        // Convert NTP fractional part to microseconds (2^-32 seconds)
        let micros = ((ntp_frac as u64 * 1_000_000) >> 32) as u32;
        Self::new(unix_secs, micros)
    }

    /// Convert from embassy-stm32 DateTime
    ///
    /// Extracts Unix timestamp and microseconds from the HAL's DateTime struct.
    pub fn from_datetime(datetime: DateTime) -> Self {
        let micros = datetime.microsecond();
        let unix_secs = datetime_to_unix(datetime);
        Self::new(unix_secs, micros)
    }
}

/// RTC operation errors
#[derive(Debug, Clone, Copy, Format)]
pub enum RtcError {
    /// RTC not initialized or time not synced
    NotSynced,
    /// RTC hardware error
    HardwareError,
}

/// Check if time has been synchronized with NTP
///
/// Returns `true` if at least one successful NTP sync has occurred.
/// Time read from the RTC is only valid when this returns `true`.
pub fn is_time_synced() -> bool {
    TIME_SYNCED.load(Ordering::Acquire)
}

/// Mark time as synchronized
///
/// Called after successfully setting RTC time from NTP.
pub fn set_time_synced() {
    TIME_SYNCED.store(true, Ordering::Release);
}

/// Write timestamp to RTC hardware
///
/// Converts Unix timestamp to DateTime and writes to the RTC.
/// Only sets TIME_SYNCED flag if the write succeeds.
///
/// **Note**: The microseconds component is not written to the RTC.
/// The RTC subsecond counter will start from 0 after the write. This is acceptable
/// because this function is primarily used during NTP synchronization, where the
/// subsecond precision comes from the network round-trip time and begins counting
/// immediately after the RTC is set.
pub fn write_rtc(rtc: &mut Rtc, timestamp: Timestamp) -> Result<(), RtcError> {
    let datetime = unix_to_datetime(timestamp.unix_secs);
    rtc.set_datetime(datetime)
        .map_err(|_| RtcError::HardwareError)?;
    set_time_synced();
    Ok(())
}

/// Read timestamp from RTC hardware
///
/// Returns an error if time has not been synchronized yet.
/// Extracts both Unix time and microseconds from the HAL's DateTime.
/// Updates the cached timestamp for defmt.
pub fn read_rtc(rtc: &mut Rtc) -> Result<Timestamp, RtcError> {
    if !is_time_synced() {
        return Err(RtcError::NotSynced);
    }

    let datetime = rtc.now().map_err(|_| RtcError::HardwareError)?;
    let timestamp = Timestamp::from_datetime(datetime);
    
    // Update cached timestamp for defmt (split u64 into two u32s for atomics)
    CACHED_UNIX_SECS_LO.store(timestamp.unix_secs as u32, Ordering::Relaxed);
    CACHED_UNIX_SECS_HI.store((timestamp.unix_secs >> 32) as u32, Ordering::Relaxed);
    CACHED_MICROS.store(timestamp.micros, Ordering::Relaxed);
    
    Ok(timestamp)
}

/// Get cached timestamp for defmt
///
/// Returns the last cached timestamp. Used by defmt timestamp macro.
/// Returns zero timestamp if never updated.
pub fn get_cached_timestamp() -> Timestamp {
    // Read cached timestamp (reconstruct u64 from two u32s)
    let lo = CACHED_UNIX_SECS_LO.load(Ordering::Relaxed) as u64;
    let hi = CACHED_UNIX_SECS_HI.load(Ordering::Relaxed) as u64;
    let unix_secs = (hi << 32) | lo;
    let micros = CACHED_MICROS.load(Ordering::Relaxed);
    Timestamp::new(unix_secs, micros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntp_to_unix_conversion() {
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;
        let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET, 0);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.micros, 0);
    }

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1704067200, 500000);
        assert_eq!(ts.unix_secs, 1704067200);
        assert_eq!(ts.micros, 500000);
    }
}
