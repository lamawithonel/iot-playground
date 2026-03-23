//! SEN66-specific sensor configuration
//!
//! Conditioning phase indices and warmup thresholds for the
//! Sensirion SEN66 environmental combo sensor.  The SEN66
//! contains five sub-sensors, each with a different warmup
//! period before readings stabilize.
//!
//! | Phase     | Index | Threshold | Sub-sensor |
//! |-----------|-------|-----------|------------|
//! | `TEMP_RH` | 0     | 8 s       | SHT4x     |
//! | `PM`      | 1     | 120 s     | SPS6x     |
//! | `CO2`     | 2     | 180 s     | SCD41     |
//! | `VOC`     | 3     | 60 s      | SGP41     |
//! | `NOX`     | 4     | 600 s     | SGP41     |

/// Phase index: Temp/RH (SHT4x sub-sensor)
pub const TEMP_RH: usize = 0;

/// Phase index: Particulate matter (SPS6x sub-sensor)
pub const PM: usize = 1;

/// Phase index: CO₂ (SCD41 sub-sensor)
pub const CO2: usize = 2;

/// Phase index: VOC index (SGP41 sub-sensor)
pub const VOC: usize = 3;

/// Phase index: NOx index (SGP41 sub-sensor)
pub const NOX: usize = 4;

/// Total number of conditioning phases
pub const NUM_PHASES: usize = 5;

/// Warmup thresholds in seconds, indexed by phase constant
///
/// Each sub-sensor needs a minimum warmup period after
/// `start_continuous_measurement()` before its readings are
/// reliable.  Values are from the SEN66 datasheet.
pub const THRESHOLDS: [u64; NUM_PHASES] = [8, 120, 180, 60, 600];
