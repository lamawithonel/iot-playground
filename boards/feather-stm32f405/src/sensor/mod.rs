#![deny(warnings)]
//! Sensor module for environmental data acquisition
//!
//! Provides a hardware-agnostic `SensorReading` type and the SEN66
//! driver wrapper.  The reading struct is `Copy` so it can be sent
//! through an `rtic-sync` channel without allocation.

#![deny(unsafe_code)]

pub mod sen66;

/// Environmental sensor reading (all fields optional)
///
/// Values use fixed-point integer scaling to avoid float formatting
/// in JSON payloads.  Each field is `None` when the sensor has not
/// yet produced a valid measurement.
#[derive(Clone, Copy, defmt::Format)]
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
