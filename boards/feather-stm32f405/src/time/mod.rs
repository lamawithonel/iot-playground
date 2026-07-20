//! Time Synchronization Module with Hardware RTC
//!
//! Implements time synchronization using SNTP (Simple Network Time
//! Protocol) per RFC 5905, fulfilling requirements SR-NET-006 and
//! SR-NET-007.
//!
//! ## Architecture
//!
//! - The SNTP client syncs once at network bring-up (periodic
//!   resync is a planned follow-up)
//! - Time is written to the STM32 internal hardware RTC
//! - Between syncs, timestamps are read from the internal RTC
//! - Sync status is stored atomically in CCM RAM
//!
//! ## Usage
//!
//! ```no_run
//! time::initialize_rtc(rtc, rtc_time);
//!
//! // The SNTP client lives in the shared iot-net crate; its
//! // on_sync callback applies each validated timestamp.
//! let mut sntp = iot_net::SntpClient::new(|ts| {
//!     time::write_rtc(&ts).map_err(|_| NetworkError::SntpFailed)
//! });
//! sntp.run(stack, &mut rng).await?;
//!
//! let timestamp = time::get_timestamp();
//! ```

#![deny(unsafe_code)]
#![deny(warnings)]

mod calendar;
mod rtc;

// Re-export public API
#[allow(unused_imports)]
pub use rtc::{get_timestamp, initialize_rtc, is_time_synced, write_rtc, RtcError, Timestamp};
