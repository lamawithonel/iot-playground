//! RTC (Real-Time Clock) wrapper and timestamp operations
#![deny(unsafe_code)]
#![deny(warnings)]

use crate::ccmram::TIME_SYNCED;

use core::cell::RefCell;
use core::sync::atomic::Ordering;
use critical_section::Mutex;
use defmt::info;
use embassy_stm32::rtc::{Rtc, RtcTimeProvider};

use super::calendar::{datetime_to_unix, unix_to_datetime};

// Re-export core types (BSP consumers use these via `crate::time::*`)
pub use iot_core::time::{RtcError, Timestamp};

/// Global internal RTC instance (write half: `set_datetime`)
static RTC: Mutex<RefCell<Option<Rtc>>> = Mutex::new(RefCell::new(None));

/// Global internal RTC time provider (read half: `now`)
///
/// embassy-stm32 0.6.0 split `Rtc` into a write-only handle plus a
/// separate [`RtcTimeProvider`] for reads; both come from the same
/// `Rtc::new()` call and are stored here together.
static RTC_TIME: Mutex<RefCell<Option<RtcTimeProvider>>> = Mutex::new(RefCell::new(None));

/// Initialize internal RTC
pub fn initialize_rtc(rtc: Rtc, rtc_time: RtcTimeProvider) {
    critical_section::with(|cs| {
        RTC.borrow(cs).replace(Some(rtc));
        RTC_TIME.borrow(cs).replace(Some(rtc_time));
    });
    info!("Internal RTC initialized");
}

/// Check if time has been synchronized with NTP
#[allow(dead_code)]
pub fn is_time_synced() -> bool {
    TIME_SYNCED.load(Ordering::Acquire)
}

/// Write timestamp to internal RTC hardware
pub fn write_rtc(timestamp: Timestamp) -> Result<(), RtcError> {
    let datetime = unix_to_datetime(timestamp.unix_secs);

    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow(cs).borrow_mut().as_mut() {
            rtc.set_datetime(datetime)
                .map_err(|_| RtcError::HardwareError)?;
            TIME_SYNCED.store(true, Ordering::Release);
            Ok(())
        } else {
            Err(RtcError::NotInitialized)
        }
    })
}

/// Read timestamp from internal RTC hardware
#[allow(dead_code)]
pub fn read_rtc() -> Result<Timestamp, RtcError> {
    if !TIME_SYNCED.load(Ordering::Acquire) {
        return Err(RtcError::NotInitialized);
    }

    critical_section::with(|cs| {
        if let Some(rtc_time) = RTC_TIME.borrow(cs).borrow().as_ref() {
            let datetime = rtc_time.now().map_err(|_| RtcError::HardwareError)?;
            let unix_secs = datetime_to_unix(datetime);
            Ok(Timestamp::new(unix_secs, 0))
        } else {
            Err(RtcError::NotInitialized)
        }
    })
}

/// Get current timestamp from internal RTC hardware
#[allow(dead_code)]
pub fn get_timestamp() -> Timestamp {
    read_rtc().unwrap_or(Timestamp::new(0, 0))
}
