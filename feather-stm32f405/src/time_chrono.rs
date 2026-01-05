//! SNTP Time Synchronization Module with Hardware RTC (Chrono version)
//!
//! This is a TEST VERSION using chrono crate for date/time conversions.
//! Used for binary size comparison against custom implementations.

use chrono::{Datelike, NaiveDateTime, Timelike};
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use critical_section::Mutex;
use defmt::{error, info};
use embassy_stm32::rtc::{DateTime, DayOfWeek, Rtc};

/// Global internal RTC instance
static RTC: Mutex<RefCell<Option<Rtc>>> = Mutex::new(RefCell::new(None));

/// System time synchronization status in CCM RAM
#[link_section = ".ccmram"]
static TIME_SYNCED: AtomicBool = AtomicBool::new(false);

/// Convert Unix timestamp to RTC DateTime using chrono
fn unix_to_datetime_chrono(unix_secs: i64) -> DateTime {
    let naive_dt = NaiveDateTime::from_timestamp_opt(unix_secs, 0)
        .unwrap_or_else(|| {
            error!("Failed to construct NaiveDateTime, falling back to epoch");
            NaiveDateTime::from_timestamp_opt(0, 0).unwrap()
        });

    DateTime::from(
        naive_dt.year() as u16,
        naive_dt.month() as u8,
        naive_dt.day() as u8,
        DayOfWeek::Monday,  // Placeholder
        naive_dt.hour() as u8,
        naive_dt.minute() as u8,
        naive_dt.second() as u8,
        0,
    )
    .unwrap_or_else(|_| {
        error!("Failed to construct DateTime, falling back to epoch");
        DateTime::from(1970, 1, 1, DayOfWeek::Thursday, 0, 0, 0, 0).unwrap()
    })
}

/// Convert RTC DateTime to Unix timestamp using chrono
fn datetime_to_unix_chrono(dt: DateTime) -> i64 {
    let naive_dt = NaiveDateTime::parse_from_str(
        &format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year(), dt.month(), dt.day(),
            dt.hour(), dt.minute(), dt.second()
        ),
        "%Y-%m-%d %H:%M:%S"
    ).unwrap_or_else(|_| {
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap()
    });

    naive_dt.timestamp()
}

/// Test function to measure size impact
pub fn test_conversions() {
    let unix_time = 1735948800i64; // 2025-01-04 00:00:00 UTC
    let dt = unix_to_datetime_chrono(unix_time);
    let back_to_unix = datetime_to_unix_chrono(dt);
    info!("Unix: {}, back: {}", unix_time, back_to_unix);
}
