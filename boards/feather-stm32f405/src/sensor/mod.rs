#![deny(warnings)]
//! Sensor module for environmental data acquisition
//!
//! Provides a hardware-agnostic `SensorReading` type and the SEN66
//! driver wrapper.  The reading struct is `Copy` so it can be sent
//! through an `rtic-sync` channel without allocation.
//!
//! # Conditioning
//!
//! Each SEN66 sub-sensor has a warmup period after power-on before
//! readings are reliable.  [`SensorState`] tracks approximate
//! elapsed time and suppresses unreliable fields (returning `None`)
//! until the warmup threshold is met.
//!
//! | Sensor | Warmup  |
//! |--------|---------|
//! | Temp/RH| ~8 s    |
//! | PM     | ~2 min  |
//! | CO₂    | ~3 min  |
//! | VOC    | ~60 s   |
//! | NOx    | ~10 min |

#![deny(unsafe_code)]

pub mod sen66;

use crate::config::SAMPLE_INTERVAL_SECS;

/// Initial delay before first sensor read, in seconds
///
/// The sensor needs ~1 s after `start_continuous_measurement()`
/// before the first valid sample is available.  We wait slightly
/// longer to ensure data is ready.
pub const INITIAL_DELAY_SECS: u64 = 2;

/// Temp/RH sensor (SHT4x) warmup time in seconds
const TEMP_RH_WARMUP_SECS: u64 = 8;

/// PM sensor (SPS6x) warmup time in seconds
const PM_WARMUP_SECS: u64 = 120;

/// CO₂ sensor (SCD41) warmup time in seconds
const CO2_WARMUP_SECS: u64 = 180;

/// VOC index (SGP41) warmup time in seconds
const VOC_WARMUP_SECS: u64 = 60;

/// NOx index (SGP41) warmup time in seconds
const NOX_WARMUP_SECS: u64 = 600;

/// Tracks sensor conditioning state across reads
///
/// Created once in the sensor task and passed to each
/// `read()` call.  Conditioning thresholds are based on
/// Sensirion datasheet recommendations for the SEN66's
/// sub-sensors (SPS6x, SCD41, SGP41, SHT4x).
///
/// Elapsed time is tracked approximately: the first read
/// records [`INITIAL_DELAY_SECS`] and each subsequent read
/// adds [`SAMPLE_INTERVAL_SECS`].
pub struct SensorState {
    /// Approximate elapsed time since sensor start, in seconds.
    /// Zero indicates no reads have occurred yet.
    elapsed_secs: u64,
    /// Whether Temp/RH conditioning milestone was logged
    temp_rh_logged: bool,
    /// Whether PM conditioning milestone was logged
    pm_logged: bool,
    /// Whether CO₂ conditioning milestone was logged
    co2_logged: bool,
    /// Whether VOC conditioning milestone was logged
    voc_logged: bool,
    /// Whether NOx conditioning milestone was logged
    nox_logged: bool,
}

impl SensorState {
    /// Create a new state tracker (all sensors conditioning)
    pub const fn new() -> Self {
        Self {
            elapsed_secs: 0,
            temp_rh_logged: false,
            pm_logged: false,
            co2_logged: false,
            voc_logged: false,
            nox_logged: false,
        }
    }

    /// Record a successful read and return elapsed seconds
    ///
    /// The first call records [`INITIAL_DELAY_SECS`]; subsequent
    /// calls add [`SAMPLE_INTERVAL_SECS`].
    pub fn record_read(&mut self) -> u64 {
        if self.elapsed_secs == 0 {
            self.elapsed_secs = INITIAL_DELAY_SECS;
        } else {
            self.elapsed_secs = self.elapsed_secs.saturating_add(SAMPLE_INTERVAL_SECS);
        }
        self.elapsed_secs
    }

    /// Whether Temp/RH readings are reliable
    pub fn temp_rh_ready(&self) -> bool {
        self.elapsed_secs >= TEMP_RH_WARMUP_SECS
    }

    /// Whether PM readings are reliable
    pub fn pm_ready(&self) -> bool {
        self.elapsed_secs >= PM_WARMUP_SECS
    }

    /// Whether CO₂ readings are reliable
    pub fn co2_ready(&self) -> bool {
        self.elapsed_secs >= CO2_WARMUP_SECS
    }

    /// Whether VOC index is reliable
    pub fn voc_ready(&self) -> bool {
        self.elapsed_secs >= VOC_WARMUP_SECS
    }

    /// Whether NOx index is reliable
    pub fn nox_ready(&self) -> bool {
        self.elapsed_secs >= NOX_WARMUP_SECS
    }

    /// Log milestone transitions (call after `record_read`)
    pub fn log_milestones(&mut self) {
        if !self.temp_rh_logged && self.temp_rh_ready() {
            defmt::info!("SEN66: Temp/RH conditioning complete");
            self.temp_rh_logged = true;
        }
        if !self.voc_logged && self.voc_ready() {
            defmt::info!("SEN66: VOC conditioning complete");
            self.voc_logged = true;
        }
        if !self.pm_logged && self.pm_ready() {
            defmt::info!("SEN66: PM conditioning complete");
            self.pm_logged = true;
        }
        if !self.co2_logged && self.co2_ready() {
            defmt::info!("SEN66: CO₂ conditioning complete");
            self.co2_logged = true;
        }
        if !self.nox_logged && self.nox_ready() {
            defmt::info!("SEN66: NOx conditioning complete — all readings now valid");
            self.nox_logged = true;
        }
    }
}

/// Environmental sensor reading (all fields optional)
///
/// Values use fixed-point integer scaling to avoid float formatting
/// in JSON payloads.  Each field is `None` when the sensor has not
/// yet produced a valid measurement or is still conditioning.
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
