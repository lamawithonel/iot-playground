//! Calendar date/time conversions for STM32 RTC
//!
//! Thin wrappers around [`iot_core::time::calendar`] algorithms,
//! converting between primitive tuples and the embassy-stm32
//! `DateTime` type.
#![deny(unsafe_code)]
#![deny(warnings)]

use embassy_stm32::rtc::{DateTime, DayOfWeek};

/// Convert Unix timestamp to RTC DateTime
///
/// Uses O(1) Hinnant algorithm via [`iot_core::time::calendar`].
/// **Limitations**: Day of week is always `Monday` (placeholder).
pub fn unix_to_datetime(unix_secs: u64) -> DateTime {
    let (year, month, day, hour, minute, second) =
        iot_core::time::calendar::unix_to_civil(unix_secs);

    DateTime::from(year, month, day, DayOfWeek::Monday, hour, minute, second, 0)
        .unwrap_or_else(|_| DateTime::from(1970, 1, 1, DayOfWeek::Thursday, 0, 0, 0, 0).unwrap())
}

/// Convert RTC DateTime to Unix timestamp
///
/// Uses O(1) Hinnant algorithm via [`iot_core::time::calendar`].
#[allow(dead_code)]
pub fn datetime_to_unix(dt: DateTime) -> u64 {
    iot_core::time::calendar::civil_to_unix(
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let test_dates = [
            0u64, 946684800, 1609459200, 1704067200, 2147483647, 4102444800,
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
        let leap_day =
            datetime_to_unix(DateTime::from(2024, 2, 29, DayOfWeek::Monday, 0, 0, 0, 0).unwrap());
        let dt = unix_to_datetime(leap_day);
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 29);
    }

    #[test]
    fn test_end_of_century() {
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
