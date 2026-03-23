//! Calendar date/time conversions using O(1) algorithms
//!
//! Implements Howard Hinnant's civil_from_days and days_from_civil
//! algorithms.
//! Reference: <http://howardhinnant.github.io/date_algorithms.html>
//!
//! These algorithms are used in C++20's `<chrono>` library and
//! provide:
//! - O(1) time complexity (no year iteration)
//! - Correct handling of leap years
//!
//! # Supported range
//!
//! Year is represented as `u16`, so the valid range is 1–65535 CE.
//! The higher-level `unix_to_civil` / `civil_to_unix` functions
//! further restrict to 1970-01-01 onward (positive Unix timestamps).

/// Check if year is a leap year (Gregorian calendar)
///
/// Correctly implements standard leap year rules:
/// - Divisible by 4: leap year
/// - EXCEPT divisible by 100: not a leap year
/// - EXCEPT divisible by 400: leap year
///
/// # Examples
///
/// ```
/// use iot_core::time::calendar::is_leap_year;
/// assert!(is_leap_year(2000));   // Divisible by 400
/// assert!(!is_leap_year(1900));  // Divisible by 100, not 400
/// assert!(is_leap_year(2024));   // Divisible by 4, not by 100
/// ```
pub fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Convert days since Unix epoch to civil date (year, month, day)
///
/// Howard Hinnant's civil_from_days algorithm.
/// Reference: <http://howardhinnant.github.io/date_algorithms.html>
///
/// This is an O(1) algorithm that correctly handles all leap years.
/// The result year is returned as `u16` (valid range 1–65535 CE).
pub fn civil_from_days(days_since_epoch: i32) -> (u16, u8, u8) {
    // Shift epoch from 1970-01-01 to 0000-03-01 (March 1, year 0)
    let z = days_since_epoch + 719468;

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

    let year_u16 = u16::try_from(year).expect("civil_from_days: computed year is out of u16 range");

    (year_u16, m, d)
}

/// Convert civil date (year, month, day) to days since Unix epoch
///
/// Howard Hinnant's days_from_civil algorithm.
/// Reference: <http://howardhinnant.github.io/date_algorithms.html>
///
/// This is an O(1) algorithm that correctly handles all leap years.
/// Year is `u16` (valid range 1–65535 CE).
pub fn days_from_civil(year: u16, month: u8, day: u8) -> i32 {
    let y = year as i32;
    let m = month as i32;
    let d = day as i32;

    // Adjust year and month to make March = month 0
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };

    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (m as u32) + 2) / 5 + (d as u32) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    era * 146097 + (doe as i32) - 719468
}

/// Convert Unix timestamp to civil date and time components
///
/// Returns `(year, month, day, hour, minute, second)`.
/// UTC only — no timezone support.
///
/// # Valid range
///
/// Supports timestamps from 0 (1970-01-01) through the `u16` year
/// limit (~year 65535).
///
/// # Panics
///
/// Panics if `unix_secs` is large enough that the day count
/// overflows `i32` (approximately year 5.8 million).
pub fn unix_to_civil(unix_secs: u64) -> (u16, u8, u8, u8, u8, u8) {
    const SECONDS_PER_DAY: u64 = 86400;

    let days_since_epoch = i32::try_from(unix_secs / SECONDS_PER_DAY)
        .expect("unix_to_civil: timestamp out of supported range");
    let secs_today = unix_secs % SECONDS_PER_DAY;

    let hour = (secs_today / 3600) as u8;
    let minute = ((secs_today % 3600) / 60) as u8;
    let second = (secs_today % 60) as u8;

    let (year, month, day) = civil_from_days(days_since_epoch);
    (year, month, day, hour, minute, second)
}

/// Convert civil date and time components to Unix timestamp
///
/// UTC only — no timezone support.
///
/// # Panics
///
/// Panics if the resulting date is before 1970-01-01 (negative
/// day count).
pub fn civil_to_unix(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u64 {
    const SECONDS_PER_DAY: u64 = 86400;

    let days_since_epoch = days_from_civil(year, month, day);
    assert!(
        days_since_epoch >= 0,
        "civil_to_unix: date before 1970-01-01 is not supported"
    );
    (days_since_epoch as u64) * SECONDS_PER_DAY
        + (hour as u64) * 3600
        + (minute as u64) * 60
        + (second as u64)
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
        let (y, m, d, h, min, s) = unix_to_civil(0);
        assert_eq!((y, m, d, h, min, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_round_trip_conversion() {
        let test_dates = [
            0u64,       // 1970-01-01 00:00:00
            946684800,  // 2000-01-01 00:00:00
            1609459200, // 2021-01-01 00:00:00
            1704067200, // 2024-01-01 00:00:00
            2147483647, // 2038-01-19 03:14:07 (32-bit limit)
            4102444800, // 2100-01-01 00:00:00
        ];

        for &unix_secs in &test_dates {
            let (y, m, d, h, min, s) = unix_to_civil(unix_secs);
            let converted_back = civil_to_unix(y, m, d, h, min, s);
            assert_eq!(
                unix_secs, converted_back,
                "Round trip failed for timestamp {}",
                unix_secs
            );
        }
    }

    #[test]
    fn test_leap_day_2024() {
        let leap_day = civil_to_unix(2024, 2, 29, 0, 0, 0);
        let (y, m, d, _, _, _) = unix_to_civil(leap_day);
        assert_eq!((y, m, d), (2024, 2, 29));
    }

    #[test]
    fn test_end_of_century() {
        let unix_secs = civil_to_unix(1999, 12, 31, 23, 59, 59);
        let (y, m, d, h, min, s) = unix_to_civil(unix_secs);
        assert_eq!((y, m, d, h, min, s), (1999, 12, 31, 23, 59, 59));
    }

    #[test]
    fn test_y2k() {
        let (y, m, d, h, min, s) = unix_to_civil(946684800);
        assert_eq!((y, m, d, h, min, s), (2000, 1, 1, 0, 0, 0));
    }
}
