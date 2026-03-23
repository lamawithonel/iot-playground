#![deny(warnings)]
//! Sensor module for environmental data acquisition
//!
//! Re-exports platform-agnostic types from `iot_core::sensor` and
//! provides the SEN66 driver wrapper.

#![deny(unsafe_code)]

pub mod sen66;

use crate::config::SAMPLE_INTERVAL_SECS;

// Re-export core types for BSP consumers
pub use iot_core::sensor::conditioning::SensorState;
pub use iot_core::sensor::{to_deci, SensorReading};

/// Initial delay before first sensor read, in seconds
///
/// The sensor needs ~1 s after `start_continuous_measurement()`
/// before the first valid sample is available.  We wait slightly
/// longer to ensure data is ready.
pub const INITIAL_DELAY_SECS: u64 = 2;

/// Create a new [`SensorState`] with BSP-configured intervals
pub fn new_sensor_state() -> SensorState {
    SensorState::new(SAMPLE_INTERVAL_SECS, INITIAL_DELAY_SECS)
}
