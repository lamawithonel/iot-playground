#![deny(warnings)]
//! SEN66 environmental sensor driver wrapper
//!
//! Wraps the `sen6x` async driver for use with embassy-stm32 I2C.
//! Converts floating-point measurements to fixed-point integers
//! suitable for JSON serialization without `std` float formatting.

#![deny(unsafe_code)]

use defmt::{error, info, warn};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use sen6x::asynchronous::Sen6x;

use super::SensorReading;

/// Scale an f32 to deci-units (one decimal place) as i32
///
/// Rounds half-away-from-zero after scaling.  Non-finite values
/// (NaN, ±Inf) map to 0 rather than producing a plausible number.
///
/// Example: `22.45` → `225`, `-3.14` → `-31`
fn to_deci(val: f32) -> i32 {
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

/// Read the latest sample and convert to `SensorReading`
///
/// Returns `SensorReading::empty()` if the sensor reports data not
/// ready, rather than treating it as an error.
pub async fn read<I2C, D>(sensor: &mut Sen6x<I2C, D>) -> SensorReading
where
    I2C: I2c,
    D: DelayNs,
{
    let ready = match sensor.get_is_data_ready().await {
        Ok(r) => r,
        Err(e) => {
            warn!("SEN66: data ready check failed: {:?}", e);
            return SensorReading::empty();
        }
    };

    if !ready {
        return SensorReading::empty();
    }

    match sensor.get_sample().await {
        Ok(sample) => {
            info!(
                "SEN66: PM2.5={} CO2={} T={} RH={}",
                to_deci(sample.pm2_5),
                sample.co2,
                to_deci(sample.temperature),
                to_deci(sample.humidity),
            );

            SensorReading {
                pm1_0: Some(to_deci(sample.pm1)),
                pm2_5: Some(to_deci(sample.pm2_5)),
                pm4_0: Some(to_deci(sample.pm4)),
                pm10: Some(to_deci(sample.pm10)),
                co2: Some(sample.co2),
                voc: Some(to_deci(sample.voc)),
                nox: Some(to_deci(sample.nox)),
                temp_c: Some(to_deci(sample.temperature)),
                humidity: Some(to_deci(sample.humidity)),
            }
        }
        Err(e) => {
            error!("SEN66: read failed: {:?}", e);
            SensorReading::empty()
        }
    }
}
