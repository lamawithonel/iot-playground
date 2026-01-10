//! Time Synchronization Module with Hardware RTC
//!
//! Implements time synchronization using SNTP (Simple Network Time Protocol)
//! per RFC 5905, fulfilling requirements SR-NET-006 and SR-NET-007.
//!
//! ## Architecture
//! - SNTP client syncs with NTP servers every 15 minutes
//! - Time is written to STM32 hardware internal RTC
//! - Between syncs, timestamps are read from internal RTC hardware
//! - Sync status stored atomically in CCM RAM
//!
//! ## Usage
//! ```no_run
//! // Initialize internal RTC
//! time::initialize_rtc(rtc);
//!
//! // SNTP client is in network::SntpClient
//! let mut sntp = network::SntpClient::new();
//! if let Ok(ts) = sntp.run(&stack).await {
//!     info!("Time synced: {}.{:06}", ts.unix_secs, ts.micros);
//! }
//!
//! // Get timestamp from internal RTC for sensor data
//! let timestamp = time::get_timestamp();
//! ```

#![deny(unsafe_code)]
#![deny(warnings)]

mod calendar;
pub mod rtc;

// Re-export public API
#[allow(unused_imports)]
pub use rtc::{get_timestamp, initialize_rtc, is_time_synced, write_rtc, RtcError, Timestamp};

#[cfg(test)]
use calendar::is_leap_year;

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
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2023));
    }
}
