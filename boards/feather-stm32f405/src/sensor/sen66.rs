#![deny(warnings)]
//! SEN66 environmental sensor driver wrapper
//!
//! Wraps the `sen6x` async driver for use with embassy-stm32 I2C.
//! Converts floating-point measurements to fixed-point integers
//! suitable for JSON serialization without `std` float formatting.
//!
//! # Conditioning
//!
//! The SEN66 contains multiple sub-sensors (SPS6x, SCD41, SGP41,
//! SHT4x), each with different warmup periods.  The `read()`
//! function accepts a [`Sen66State`] tracker and returns `None`
//! for fields that have not yet met their conditioning threshold.
//! During NOx conditioning, raw ticks are logged for hardware
//! diagnostics.

#![deny(unsafe_code)]

use defmt::{debug, error, info, warn};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use sen6x::asynchronous::Sen6x;

use super::{sen66_config, to_deci, Sen66Reading, Sen66State};

/// Initialize the SEN66 sensor and start continuous measurement
///
/// Returns the driver instance ready for periodic reads.  The sensor
/// needs ~1 s after `start_continuous_measurement()` before the first
/// valid sample is available.
pub async fn init<I2C, D>(delay: D, i2c: I2C) -> Result<Sen6x<I2C, D>, sen6x::Sen6xError>
where
    I2C: I2c,
    D: DelayNs,
{
    let mut sensor = Sen6x::new(delay, i2c);

    info!("SEN66: starting continuous measurement");
    sensor.start_continuous_measurement().await?;

    Ok(sensor)
}

/// Read the latest sample with conditioning guards
///
/// Returns `Sen66Reading::empty()` if the sensor reports data not
/// ready.  Fields that have not yet met their conditioning threshold
/// are returned as `None`.  During NOx conditioning, raw sensor ticks
/// are logged for hardware diagnostics.
pub async fn read<I2C, D>(sensor: &mut Sen6x<I2C, D>, state: &mut Sen66State) -> Sen66Reading
where
    I2C: I2c,
    D: DelayNs,
{
    let ready = match sensor.get_is_data_ready().await {
        Ok(r) => r,
        Err(e) => {
            warn!("SEN66: data ready check failed: {:?}", e);
            return Sen66Reading::empty();
        }
    };

    if !ready {
        return Sen66Reading::empty();
    }

    // Log raw NOx ticks during conditioning for diagnostics
    if !state.ready(sen66_config::NOX) {
        log_raw_nox(sensor).await;
    }

    match sensor.get_sample().await {
        Ok(sample) => {
            let elapsed = state.record_read();
            let milestones = state.check_milestones();

            if milestones[sen66_config::TEMP_RH] {
                info!("SEN66: Temp/RH conditioning complete");
            }
            if milestones[sen66_config::VOC] {
                info!("SEN66: VOC conditioning complete");
            }
            if milestones[sen66_config::PM] {
                info!("SEN66: PM conditioning complete");
            }
            if milestones[sen66_config::CO2] {
                info!("SEN66: CO₂ conditioning complete");
            }
            if milestones[sen66_config::NOX] {
                info!("SEN66: NOx conditioning complete — all readings now valid");
            }

            if !state.ready(sen66_config::NOX) {
                info!(
                    "SEN66: ~{}s elapsed (conditioning) — PM2.5={} CO2={} T={} RH={} NOx=suppressed",
                    elapsed,
                    to_deci(sample.pm2_5),
                    sample.co2,
                    to_deci(sample.temperature),
                    to_deci(sample.humidity),
                );
            } else {
                info!(
                    "SEN66: PM2.5={} CO2={} T={} RH={} NOx={}",
                    to_deci(sample.pm2_5),
                    sample.co2,
                    to_deci(sample.temperature),
                    to_deci(sample.humidity),
                    to_deci(sample.nox),
                );
            }

            // Temp/RH: suppress until SHT4x has stabilized (~8 s)
            let (temp_c, humidity) = if state.ready(sen66_config::TEMP_RH) {
                (
                    Some(to_deci(sample.temperature)),
                    Some(to_deci(sample.humidity)),
                )
            } else {
                (None, None)
            };

            // PM: suppress during first ~2 min
            let (pm1_0, pm2_5, pm4_0, pm10) = if state.ready(sen66_config::PM) {
                (
                    Some(to_deci(sample.pm1)),
                    Some(to_deci(sample.pm2_5)),
                    Some(to_deci(sample.pm4)),
                    Some(to_deci(sample.pm10)),
                )
            } else {
                (None, None, None, None)
            };

            // CO₂: suppress during first ~3 min
            let co2 = if state.ready(sen66_config::CO2) {
                Some(sample.co2)
            } else {
                None
            };

            // VOC: suppress during first ~60 s
            let voc = if state.ready(sen66_config::VOC) {
                Some(to_deci(sample.voc))
            } else {
                None
            };

            // NOx: suppress during first ~10 min
            let nox = if state.ready(sen66_config::NOX) {
                Some(to_deci(sample.nox))
            } else {
                None
            };

            Sen66Reading {
                pm1_0,
                pm2_5,
                pm4_0,
                pm10,
                co2,
                voc,
                nox,
                temp_c,
                humidity,
            }
        }
        Err(e) => {
            error!("SEN66: read failed: {:?}", e);
            Sen66Reading::empty()
        }
    }
}

/// Log raw NOx ticks for hardware diagnostics
///
/// Called during the NOx conditioning period to verify the MOx
/// element is responding to the environment, even though the
/// algorithm output is suppressed.
async fn log_raw_nox<I2C, D>(sensor: &mut Sen6x<I2C, D>)
where
    I2C: I2c,
    D: DelayNs,
{
    match sensor.get_raw_sample().await {
        Ok(raw) => {
            debug!(
                "SEN66 raw: NOx={} VOC={} T={} RH={}",
                raw.raw_nox, raw.raw_voc, raw.raw_temperature, raw.raw_humidity
            );
        }
        Err(e) => {
            warn!("SEN66: raw sample read failed: {:?}", e);
        }
    }
}
