//! Sensor types and conditioning logic
//!
//! Platform-agnostic sensor data types for IoT firmware.
//! Hardware driver code remains in the board crate.
//!
//! The generic utilities ([`to_deci`],
//! [`conditioning::ConditioningState`]) are always available.
//! SEN66-specific types ([`sen66::Sen66Reading`]) and phase
//! configuration ([`sen66`]) require the `sen66` feature.

pub mod conditioning;

#[cfg(feature = "sen66")]
pub mod sen66;

// Re-export for ergonomics: `iot_core::sensor::Sen66Reading`
#[cfg(feature = "sen66")]
pub use sen66::Sen66Reading;

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
}
