//! SEN66-specific sensor configuration and types
//!
//! Conditioning phase indices, warmup thresholds, and the
//! [`Sen66Reading`] data type for the Sensirion SEN66
//! environmental combo sensor.
//!
//! The SEN66 contains five sub-sensors, each with a different
//! warmup period before readings stabilize:
//!
//! | Phase     | Index | Threshold | Sub-sensor |
//! |-----------|-------|-----------|------------|
//! | `TEMP_RH` | 0     | 8 s       | SHT4x     |
//! | `PM`      | 1     | 120 s     | SPS6x     |
//! | `CO2`     | 2     | 180 s     | SCD41     |
//! | `VOC`     | 3     | 60 s      | SGP41     |
//! | `NOX`     | 4     | 600 s     | SGP41     |

use super::conditioning::ConditioningState;
use hal_abstractions::sensor::EnvironmentalReading;

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

/// Conditioning state tracker sized for the SEN66's five phases
pub type Sen66State = ConditioningState<NUM_PHASES>;

/// Environmental sensor reading for SEN66 (all fields optional)
///
/// Values use fixed-point integer scaling to avoid float formatting
/// in JSON payloads.  Each field is `None` when the sensor has not
/// yet produced a valid measurement or is still conditioning.
///
/// This struct models the SEN66's complete output set (9 sub-sensor
/// readings).  Future sensor types define their own reading structs
/// behind separate feature gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Sen66Reading {
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

impl Sen66Reading {
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

impl EnvironmentalReading for Sen66Reading {
    fn temperature_deci(&self) -> Option<i32> {
        self.temp_c
    }

    fn humidity_deci(&self) -> Option<i32> {
        self.humidity
    }

    fn co2_ppm(&self) -> Option<u16> {
        self.co2
    }

    fn voc_index_deci(&self) -> Option<i32> {
        self.voc
    }

    fn nox_index_deci(&self) -> Option<i32> {
        self.nox
    }

    fn pm1_0_deci(&self) -> Option<i32> {
        self.pm1_0
    }

    fn pm2_5_deci(&self) -> Option<i32> {
        self.pm2_5
    }

    fn pm4_0_deci(&self) -> Option<i32> {
        self.pm4_0
    }

    fn pm10_deci(&self) -> Option<i32> {
        self.pm10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_reading_empty() {
        let r = Sen66Reading::empty();
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

    #[test]
    fn test_sensor_data_trait_impl() {
        let r = Sen66Reading {
            pm1_0: Some(52),
            pm2_5: Some(128),
            pm4_0: None,
            pm10: None,
            co2: Some(412),
            voc: Some(950),
            nox: Some(10),
            temp_c: Some(225),
            humidity: Some(452),
        };
        assert_eq!(r.temperature_deci(), Some(225));
        assert_eq!(r.humidity_deci(), Some(452));
        assert_eq!(r.co2_ppm(), Some(412));
        assert_eq!(r.voc_index_deci(), Some(950));
        assert_eq!(r.nox_index_deci(), Some(10));
        assert_eq!(r.pm1_0_deci(), Some(52));
        assert_eq!(r.pm2_5_deci(), Some(128));
        assert_eq!(r.pm4_0_deci(), None);
        assert_eq!(r.pm10_deci(), None);
    }

    #[test]
    fn test_sensor_data_empty_returns_none() {
        let r = Sen66Reading::empty();
        assert_eq!(r.temperature_deci(), None);
        assert_eq!(r.humidity_deci(), None);
        assert_eq!(r.co2_ppm(), None);
        assert_eq!(r.voc_index_deci(), None);
        assert_eq!(r.nox_index_deci(), None);
        assert_eq!(r.pm1_0_deci(), None);
        assert_eq!(r.pm2_5_deci(), None);
        assert_eq!(r.pm4_0_deci(), None);
        assert_eq!(r.pm10_deci(), None);
    }

    #[test]
    fn test_temp_rh_ready_timing() {
        let mut state = Sen66State::new(5, 2, THRESHOLDS);
        state.record_read(); // 2s
        assert!(!state.ready(TEMP_RH));

        state.record_read(); // 7s
        assert!(!state.ready(TEMP_RH));

        state.record_read(); // 12s
        assert!(state.ready(TEMP_RH));
    }

    #[test]
    fn test_voc_ready_timing() {
        let mut state = Sen66State::new(10, 2, THRESHOLDS);
        // Advance to just under 60s
        for _ in 0..6 {
            state.record_read(); // 2, 12, 22, 32, 42, 52
        }
        assert!(!state.ready(VOC));

        state.record_read(); // 62s
        assert!(state.ready(VOC));
    }

    #[test]
    fn test_nox_ready_timing() {
        let mut state = Sen66State::new(60, 2, THRESHOLDS);
        // 10 reads at 60s interval: 2, 62, 122, ..., 542
        for _ in 0..10 {
            state.record_read();
        }
        assert!(!state.ready(NOX)); // 542s < 600s

        state.record_read(); // 602s
        assert!(state.ready(NOX));
    }

    #[test]
    fn test_milestone_fires_once() {
        let mut state = Sen66State::new(5, 2, THRESHOLDS);
        state.record_read(); // 2s
        state.record_read(); // 7s
        state.record_read(); // 12s — temp_rh should fire

        let flags = state.check_milestones();
        assert!(flags[TEMP_RH]);

        // Second call should not fire again
        let flags = state.check_milestones();
        assert!(!flags[TEMP_RH]);
    }

    #[test]
    fn test_all_milestones_eventually() {
        let mut state = Sen66State::new(1, 1, THRESHOLDS);
        let mut got_nox = false;

        for _ in 0..650 {
            state.record_read();
            let flags = state.check_milestones();
            if flags[NOX] {
                got_nox = true;
                break;
            }
        }
        assert!(got_nox, "NOx milestone never fired");
    }
}
