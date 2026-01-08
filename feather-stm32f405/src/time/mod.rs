//! SNTP Time Synchronization Module with Hardware RTC
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
//! ## Features
//! - UDP socket communication with NTP servers
//! - DNS hostname resolution
//! - Multi-server fallback with retries
//! - 15-minute automatic re-synchronization
//! - Hardware internal RTC for accurate timekeeping between syncs
//! - Atomic sync status in CCM RAM
//! - Stratum validation (rejects servers with stratum > 3)
//! - RTT/2 correction for more accurate synchronization
//!
//! ## Custom Date/Time Conversions
//!
//! This module uses ~200 lines of custom calendar math instead of external crates.
//!
//! **Why custom?** Saves ~12.6 KB binary size vs chrono crate (no_std).
//!
//! **Improvements in this version:**
//! - ✅ O(1) calendar algorithms (Howard Hinnant's civil_from_days/days_from_civil)
//! - ✅ Stable Rust (no unstable features)
//! - ✅ RTT/2 correction for better accuracy
//! - ✅ Proper error handling
//!
//! **Limitations:**
//! - ✅ Accurate for dates 1970-2099 (NTP use case)
//! - ⚠️ Year range limited to 1970-2105 (u16 overflow)
//! - ⚠️ Day of week always wrong (placeholder)
//! - ✅ No timezone support (UTC only)
//! - ✅ No leap seconds (NTP ignores them too)
//!
//! **📖 See `../CUSTOM_TIME_LIMITATIONS.md` for detailed analysis**
//!
//! ### Behavior:
//! - Before first NTP sync: Shows 0 (timestamp not available)
//! - After NTP sync: Shows ISO8601 formatted time from RTC (1-second resolution)
//! - Between syncs: RTC continues counting (±20-50ppm accuracy from LSE)
//!
//! ### Format:
//! The `:iso8601s` display hint formats Unix epoch seconds as ISO8601 date-time strings.
//! Example: `1767571200` → `2026-01-05T01:00:00Z`
//!
//! ## Usage
//! ```no_run
//! // Initialize internal RTC and perform initial SNTP sync
//! time::initialize_rtc(rtc);
//! if let Ok(ts) = time::initialize_time(&stack).await {
//!     info!("Time synced: {}.{:06}", ts.unix_secs, ts.micros);
//! }
//!
//! // Check if time is synchronized
//! if time::is_time_synced() {
//!     // Safe to use timestamps
//! }
//!
//! // Spawn periodic resync task
//! spawner.spawn(time_resync(stack)).ok();
//!
//! // Get timestamp from internal RTC for sensor data
//! let timestamp = time::get_timestamp();
//! ```

// Allow unsafe code for #[link_section] attribute used in CCM RAM allocation
#![deny(unsafe_code)]
#![deny(warnings)]

mod calendar;
mod rtc;

// Re-export public API
#[allow(unused_imports)]
pub use rtc::{get_timestamp, initialize_rtc, is_time_synced, Timestamp};

// Export sntp module for direct access to sync_sntp
pub mod sntp;

// Internal exports for tests
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
