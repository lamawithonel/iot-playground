//! Time types and calendar algorithms
//!
//! Platform-agnostic time primitives for IoT firmware.  Hardware
//! RTC operations remain in the board crate; this module provides
//! the data types and pure conversion algorithms.

pub mod calendar;

/// Timestamp with microsecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Timestamp {
    /// Unix timestamp in seconds since epoch (1970-01-01 00:00:00 UTC)
    pub unix_secs: u64,
    /// Microseconds component (0-999,999)
    pub micros: u32,
}

impl Timestamp {
    /// Create a new timestamp
    ///
    /// If `micros` exceeds 999,999 the overflow is carried into
    /// `unix_secs` so the invariant `micros < 1_000_000` always
    /// holds.
    pub const fn new(unix_secs: u64, micros: u32) -> Self {
        let extra_secs = (micros / 1_000_000) as u64;
        Self {
            unix_secs: unix_secs + extra_secs,
            micros: micros % 1_000_000,
        }
    }

    /// Convert from NTP timestamp (seconds since 1900-01-01)
    ///
    /// Pre-epoch NTP timestamps (before 1970-01-01) saturate to
    /// `Timestamp { unix_secs: 0, micros: 0 }`.
    pub fn from_ntp(ntp_secs: u64, ntp_frac: u32) -> Self {
        /// NTP epoch offset (1900-01-01 to 1970-01-01 in seconds)
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

        if ntp_secs < NTP_UNIX_OFFSET {
            return Self::new(0, 0);
        }
        let unix_secs = ntp_secs - NTP_UNIX_OFFSET;
        // Convert NTP fractional part to microseconds (2^-32 seconds)
        let micros = ((ntp_frac as u64 * 1_000_000) >> 32) as u32;
        Self::new(unix_secs, micros)
    }
}

/// RTC operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RtcError {
    /// RTC not initialized
    NotInitialized,
    /// RTC hardware error
    HardwareError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_creation() {
        let ts = Timestamp::new(1704067200, 500000);
        assert_eq!(ts.unix_secs, 1704067200);
        assert_eq!(ts.micros, 500000);
    }

    #[test]
    fn test_timestamp_micros_normalization() {
        // 1_500_000 µs = 1 extra second + 500_000 µs
        let ts = Timestamp::new(100, 1_500_000);
        assert_eq!(ts.unix_secs, 101);
        assert_eq!(ts.micros, 500_000);

        // Exact boundary — 1_000_000 µs = 1 extra second + 0 µs
        let ts = Timestamp::new(100, 1_000_000);
        assert_eq!(ts.unix_secs, 101);
        assert_eq!(ts.micros, 0);

        // Normal case — no normalization needed
        let ts = Timestamp::new(100, 999_999);
        assert_eq!(ts.unix_secs, 100);
        assert_eq!(ts.micros, 999_999);
    }

    #[test]
    fn test_ntp_to_unix_conversion() {
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;
        let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET, 0);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.micros, 0);
    }

    #[test]
    fn test_ntp_fractional() {
        const NTP_UNIX_OFFSET: u64 = 2_208_988_800;
        // Half-second in NTP fractional: 2^31 = 2147483648
        let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET + 100, 2_147_483_648);
        assert_eq!(ts.unix_secs, 100);
        assert_eq!(ts.micros, 500000);
    }

    #[test]
    fn test_ntp_before_unix_epoch() {
        // NTP seconds before Unix epoch should saturate to (0, 0)
        let ts = Timestamp::from_ntp(0, 0);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.micros, 0);

        // Pre-epoch with fractional part should also saturate micros
        let ts = Timestamp::from_ntp(0, 2_147_483_648);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.micros, 0);
    }
}
