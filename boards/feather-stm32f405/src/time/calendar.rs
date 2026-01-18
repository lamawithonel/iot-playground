//! Calendar date/time conversions using O(1) algorithms
//!
//! Implements Howard Hinnant's civil_from_days and days_from_civil algorithms.
//! Reference: http://howardhinnant.github.io/date_algorithms.html
//!
//! These algorithms are used in C++20's `<chrono>` library and provide:
//! - O(1) time complexity (no year iteration)
//! - Correct handling of leap years
//! - Valid for all dates in the proleptic Gregorian calendar
#![deny(unsafe_code)]
#![deny(warnings)]

use embassy_stm32::rtc::{DateTime, DayOfWeek};

/// Check if year is a leap year (Gregorian calendar)
///
/// Correctly implements standard leap year rules:
/// - Divisible by 4: leap year
/// - EXCEPT divisible by 100: not a leap year
/// - EXCEPT divisible by 400: leap year
///
/// Examples:
/// - 2000: leap (divisible by 400)
/// - 1900: NOT leap (divisible by 100 but not 400)
/// - 2024: leap (divisible by 4, not by 100)
/// - 2100: NOT leap (divisible by 100 but not 400)
#[allow(dead_code)]
pub(crate) fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Convert Unix timestamp to RTC DateTime using O(1) algorithm
///
/// Uses Howard Hinnant's civil_from_days algorithm for efficient conversion.
/// **Limitations**: See `../CUSTOM_TIME_LIMITATIONS.md`
/// - Valid range: 1970-2105 (u16 year limit)
/// - Day of week is always `Monday` (placeholder)
/// - UTC only (no timezone support)
pub fn unix_to_datetime(unix_secs: u64) -> DateTime {
    const SECONDS_PER_DAY: u64 = 86400;

    let days_since_epoch = (unix_secs / SECONDS_PER_DAY) as i32;
    let secs_today = unix_secs % SECONDS_PER_DAY;

    let hour = (secs_today / 3600) as u8;
    let minute = ((secs_today % 3600) / 60) as u8;
    let second = (secs_today % 60) as u8;

    // Convert days since Unix epoch (1970-01-01) to civil date
    // Using Howard Hinnant's algorithm (O(1) complexity)
    let (year, month, day) = civil_from_days(days_since_epoch);

    // Build DateTime using separate arguments (embassy-stm32 v0.4.0 API)
    DateTime::from(
        year,
        month,
        day,
        DayOfWeek::Monday, // LIMITATION: Always wrong, but not needed for timekeeping
        hour,
        minute,
        second,
        0, // microsecond
    )
    .unwrap_or_else(|_| {
        // Fallback to Unix epoch if date construction fails
        DateTime::from(1970, 1, 1, DayOfWeek::Thursday, 0, 0, 0, 0).unwrap()
    })
}

/// Convert RTC DateTime to Unix timestamp using O(1) algorithm
///
/// Uses Howard Hinnant's days_from_civil algorithm for efficient conversion.
/// **Limitations**: See `../CUSTOM_TIME_LIMITATIONS.md`
/// - O(1) performance
/// - UTC only (no timezone support)
#[allow(dead_code)]
pub fn datetime_to_unix(dt: DateTime) -> u64 {
    const SECONDS_PER_DAY: u64 = 86400;

    // Convert civil date to days since Unix epoch using O(1) algorithm
    let days_since_epoch = days_from_civil(dt.year(), dt.month(), dt.day());

    // Convert to seconds and add time of day
    (days_since_epoch as u64) * SECONDS_PER_DAY
        + (dt.hour() as u64) * 3600
        + (dt.minute() as u64) * 60
        + (dt.second() as u64)
}

/// Convert days since Unix epoch to civil date (year, month, day)
///
/// Howard Hinnant's civil_from_days algorithm.
/// Reference: http://howardhinnant.github.io/date_algorithms.html
///
/// This is an O(1) algorithm that correctly handles all leap years.
fn civil_from_days(days_since_epoch: i32) -> (u16, u8, u8) {
    // Shift epoch from 1970-01-01 to 0000-03-01 (March 1, year 0)
    // This makes the year start on March 1, placing leap day at end of year
    let z = days_since_epoch + 719468; // 719468 = days from 0000-03-01 to 1970-01-01

    // Calculate era (400-year cycles)
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // day of era [0, 146096]

    // Calculate year of era [0, 399]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;

    // Calculate actual year
    let y = (yoe as i32) + era * 400;

    // Calculate day of year [0, 365]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);

    // Calculate month [0, 11] where 0 = March, 11 = February
    let mp = (5 * doy + 2) / 153;

    // Calculate day [1, 31]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;

    // Calculate month [1, 12] where 1 = January
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;

    // Adjust year for January and February
    let year = if m <= 2 { y + 1 } else { y };

    (year as u16, m, d)
}

/// Convert civil date (year, month, day) to days since Unix epoch
///
/// Howard Hinnant's days_from_civil algorithm.
/// Reference: http://howardhinnant.github.io/date_algorithms.html
///
/// This is an O(1) algorithm that correctly handles all leap years.
#[allow(dead_code)]
fn days_from_civil(year: u16, month: u8, day: u8) -> i32 {
    let y = year as i32;
    let m = month as i32;
    let d = day as i32;

    // Adjust year and month to make March = month 0, February = month 11
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };

    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32; // year of era [0, 399]
    let doy = (153 * (m as u32) + 2) / 5 + (d as u32) - 1; // day of year [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era [0, 146096]

    era * 146097 + (doe as i32) - 719468 // 719468 = days from 0000-03-01 to 1970-01-01
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000)); // Divisible by 400
        assert!(is_leap_year(2024)); // Divisible by 4
        assert!(!is_leap_year(1900)); // Divisible by 100, not 400
        assert!(!is_leap_year(2023)); // Not divisible by 4
        assert!(!is_leap_year(2100)); // Divisible by 100, not 400
    }

    #[test]
    fn test_unix_epoch() {
        let dt = unix_to_datetime(0);
        assert_eq!(dt.year(), 1970);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_round_trip_conversion() {
        // Test various dates in the valid range
        let test_dates = [
            0u64,       // 1970-01-01 00:00:00
            946684800,  // 2000-01-01 00:00:00
            1609459200, // 2021-01-01 00:00:00
            1704067200, // 2024-01-01 00:00:00
            2147483647, // 2038-01-19 03:14:07 (32-bit Unix time limit)
            4102444800, // 2100-01-01 00:00:00
        ];

        for &unix_secs in &test_dates {
            let dt = unix_to_datetime(unix_secs);
            let converted_back = datetime_to_unix(dt);
            assert_eq!(
                unix_secs, converted_back,
                "Round trip failed for timestamp {}",
                unix_secs
            );
        }
    }

    #[test]
    fn test_leap_day_2024() {
        // 2024-02-29 00:00:00 (leap day)
        let leap_day =
            datetime_to_unix(DateTime::from(2024, 2, 29, DayOfWeek::Monday, 0, 0, 0, 0).unwrap());
        let dt = unix_to_datetime(leap_day);
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 29);
    }

    #[test]
    fn test_end_of_century() {
        // 1999-12-31 23:59:59
        let dt = DateTime::from(1999, 12, 31, DayOfWeek::Monday, 23, 59, 59, 0).unwrap();
        let unix_secs = datetime_to_unix(dt);
        let converted = unix_to_datetime(unix_secs);
        assert_eq!(converted.year(), 1999);
        assert_eq!(converted.month(), 12);
        assert_eq!(converted.day(), 31);
        assert_eq!(converted.hour(), 23);
        assert_eq!(converted.minute(), 59);
        assert_eq!(converted.second(), 59);
    }
}
