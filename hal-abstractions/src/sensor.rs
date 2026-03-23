//! Environmental sensor data abstraction
//!
//! Defines a common interface for sensors that measure
//! environmental parameters.  Implementations override methods
//! for the measurements they support; all others return `None`.
//!
//! Adding new methods with default `None` impls is a non-breaking
//! change, so the trait grows safely as new sensor types appear.

/// Common interface for environmental sensor readings
///
/// Each accessor returns `None` when the sensor does not support
/// that measurement or the reading has not yet stabilized.
/// Implementations override only the methods relevant to their
/// hardware.
///
/// Fixed-point scaling conventions:
///
/// - **deci-** prefix: value × 10 (one decimal place).
///   Example: `225` represents `22.5 °C`.
/// - **ppm** suffix: parts per million, integer.
pub trait EnvironmentalReading {
    /// Temperature in deci-°C (225 = 22.5 °C)
    fn temperature_deci(&self) -> Option<i32> {
        None
    }

    /// Relative humidity in deci-% (452 = 45.2%)
    fn humidity_deci(&self) -> Option<i32> {
        None
    }

    /// CO₂ concentration in ppm
    fn co2_ppm(&self) -> Option<u16> {
        None
    }

    /// VOC index in deci-points (950 = 95.0)
    fn voc_index_deci(&self) -> Option<i32> {
        None
    }

    /// NOx index in deci-points
    fn nox_index_deci(&self) -> Option<i32> {
        None
    }

    /// PM1.0 in deci-µg/m³
    fn pm1_0_deci(&self) -> Option<i32> {
        None
    }

    /// PM2.5 in deci-µg/m³
    fn pm2_5_deci(&self) -> Option<i32> {
        None
    }

    /// PM4.0 in deci-µg/m³
    fn pm4_0_deci(&self) -> Option<i32> {
        None
    }

    /// PM10 in deci-µg/m³
    fn pm10_deci(&self) -> Option<i32> {
        None
    }
}
