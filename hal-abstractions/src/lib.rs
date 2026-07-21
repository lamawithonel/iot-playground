//! Hardware abstraction traits for IoT firmware
//!
//! This crate defines traits that abstract over hardware differences
//! between boards.  BSPs implement these traits.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![deny(warnings)]

pub mod adc_capture;
pub mod excitation;
pub mod i2c_bus;
pub mod message_port;
pub mod mute_control;
pub mod network;
pub mod record_store;
pub mod rng;
pub mod rtc;
pub mod sensor;
pub mod time;

#[cfg(any(test, feature = "mock"))]
pub mod test_support;
