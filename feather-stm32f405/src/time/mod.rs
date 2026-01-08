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
//! - defmt timestamps use RTC for Unix epoch time display
//!
//! ## Features
//! - UDP socket communication with NTP servers
//! - DNS hostname resolution
//! - Multi-server fallback with retries
//! - 15-minute automatic re-synchronization
//! - Hardware internal RTC for accurate timekeeping between syncs
//! - Atomic sync status in CCM RAM
//! - Custom defmt timestamps (Unix epoch time instead of uptime)
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
//! ## defmt Timestamps
//!
//! This module provides a custom `defmt::timestamp!()` implementation using the hardware RTC.
//! Returns Unix epoch time in seconds (u64) formatted as ISO8601 date-time.
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
//! See: <https://defmt.ferrous-systems.com/timestamps>
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
mod sntp;

// Re-export public API
#[allow(unused_imports)]
pub use rtc::{get_timestamp, is_time_synced, Timestamp};
#[allow(unused_imports)]
pub use sntp::{initialize_time, sync_sntp, SntpError};

/// Initialize the time system with hardware RTC
/// Called from init() to set up the RTC with LSE clock
pub fn init_time_system(rtc_peripheral: embassy_stm32::peripherals::RTC) {
    use embassy_stm32::rtc::{Rtc, RtcConfig};
    
    let rtc_config = RtcConfig::default();
    let rtc = Rtc::new(rtc_peripheral, rtc_config);
    defmt::info!("Internal RTC initialized with LSE (32.768kHz, ±20-50ppm accuracy)");
    
    rtc::initialize_rtc(rtc);
}

/// Run the SNTP periodic resync task
/// This task wakes every 15 minutes using the RTIC monotonic timer
/// to perform SNTP synchronization
pub async fn run_sntp_resync_task() -> ! {
    use defmt::info;
    
    info!("SNTP resync task started, waiting for network stack...");
    
    // Wait for network stack to be available
    let stack = loop {
        if let Some(stack) = crate::ccmram::get_network_stack() {
            break stack;
        }
        // Use embassy-time to avoid circular dependency on Mono
        embassy_time::Timer::after_millis(100).await;
    };
    
    info!("Network stack ready, waiting for initial DHCP...");
    
    // Wait for network to come up
    stack.wait_config_up().await;
    
    // Wait for initial time sync to complete
    embassy_time::Timer::after_secs(30).await;
    
    info!("Starting SNTP periodic resync (15-minute interval using RTIC Mono timer)");
    
    // Periodic resync every 15 minutes using embassy-time
    loop {
        embassy_time::Timer::after_secs(15 * 60).await; // 15 minutes
        
        if let Err(e) = sync_sntp(stack).await {
            defmt::warn!("Periodic SNTP resync failed: {:?}", e);
        } else {
            info!("SNTP periodic resync successful");
        }
    }
}

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
