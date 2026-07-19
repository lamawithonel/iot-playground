//! Hardware abstraction traits for IoT firmware
//!
//! This crate defines traits that abstract over hardware differences
//! between boards.  BSPs implement these traits.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![deny(warnings)]

pub mod adc_capture;
pub mod excitation;
pub mod message_port;
pub mod record_store;
pub mod rng;
pub mod rtc;
pub mod sensor;
pub mod time;

#[cfg(any(test, feature = "mock"))]
pub mod test_support;

// The `network` trait module (DNS/TCP/TLS/MQTT session
// abstraction) is a separate framework-track effort with its own
// design-- see the roadmap's framework track before starting.
// pub mod network;
