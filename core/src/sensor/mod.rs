//! Sensor types and conditioning logic
//!
//! Platform-agnostic sensor data types for IoT firmware.
//! Hardware driver code remains in the board crate.

pub mod conditioning;

/// Scale an f32 to deci-units (one decimal place) as i32
///
/// Rounds half-away-from-zero after scaling.  Non-finite values
/// (NaN, ±Inf) map to 0 rather than producing a plausible number.
///
/// Example: `22.45` → `225`, `-3.14` → `-31`
pub fn to_deci(val: f32) -> i32 {
    if !val.is_finite() {
        return 0;
    }
    let scaled = val * 10.0;
    if scaled >= 0.0 {
        (scaled + 0.5) as i32
    } else {
        (scaled - 0.5) as i32
    }
}

/// Environmental sensor reading (all fields optional)
///
/// Values use fixed-point integer scaling to avoid float formatting
/// in JSON payloads.  Each field is `None` when the sensor has not
/// yet produced a valid measurement or is still conditioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SensorReading {
    /// PM1.0 concentration in deci-µg/m³ (value 52 = 5.2 µg/m³)
    pub pm1_0: Option<i32>,
    /// PM2.5 concentration in deci-µg/m³
    pub pm2_5: Option<i32>,
    /// PM4.0 concentration in deci-µg/m³
    pub pm4_0: Option<i32>,
    /// PM10 concentration in deci-µg/m³
    pub pm10: Option<i32>,
    /// CO₂ concentration in ppm (integer, no scaling)
    pub co2: Option<u16>,
    /// VOC index in deci-points (value 950 = 95.0)
    pub voc: Option<i32>,
    /// NOx index in deci-points
    pub nox: Option<i32>,
    /// Temperature in deci-°C (value 225 = 22.5 °C)
    pub temp_c: Option<i32>,
    /// Relative humidity in deci-% (value 452 = 45.2 %)
    pub humidity: Option<i32>,
}

impl SensorReading {
    /// Create an empty reading (all fields `None`)
    pub const fn empty() -> Self {
        Self {
            pm1_0: None,
            pm2_5: None,
            pm4_0: None,
            pm10: None,
            co2: None,
            voc: None,
            nox: None,
            temp_c: None,
            humidity: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_deci_positive() {
        assert_eq!(to_deci(22.45), 225);
        assert_eq!(to_deci(0.0), 0);
        assert_eq!(to_deci(1.0), 10);
        assert_eq!(to_deci(99.99), 1000);
    }

    #[test]
    fn test_to_deci_negative() {
        assert_eq!(to_deci(-3.14), -31);
        assert_eq!(to_deci(-0.05), -1);
        assert_eq!(to_deci(-10.0), -100);
    }

    #[test]
    fn test_to_deci_rounding() {
        // Exact halves round away from zero
        assert_eq!(to_deci(1.05), 11);
        assert_eq!(to_deci(-1.05), -11);
        // Just below half rounds toward zero
        assert_eq!(to_deci(1.04), 10);
        assert_eq!(to_deci(-1.04), -10);
    }

    #[test]
    fn test_to_deci_nan_inf() {
        assert_eq!(to_deci(f32::NAN), 0);
        assert_eq!(to_deci(f32::INFINITY), 0);
        assert_eq!(to_deci(f32::NEG_INFINITY), 0);
    }

    #[test]
    fn test_sensor_reading_empty() {
        let r = SensorReading::empty();
        assert_eq!(r.pm1_0, None);
        assert_eq!(r.pm2_5, None);
        assert_eq!(r.pm4_0, None);
        assert_eq!(r.pm10, None);
        assert_eq!(r.co2, None);
        assert_eq!(r.voc, None);
        assert_eq!(r.nox, None);
        assert_eq!(r.temp_c, None);
        assert_eq!(r.humidity, None);
    }
}
