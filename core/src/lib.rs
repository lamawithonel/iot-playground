//! Platform-agnostic core logic for IoT firmware
//!
//! This crate contains business logic that can be shared across all
//! supported boards and tiers.  It has NO hardware dependencies.
//!
//! # Features
//!
//! - `defmt` — Enable `defmt::Format` derives on public types
//!   (disabled by default so host-side tests compile without a
//!   defmt backend)
//! - `embedded-io` — Enable `embedded_io_async::Error` impls
//!   on network error types
//! - `sen66` — Enable SEN66-specific sensor types
//!   (`Sen66Reading`), warmup thresholds, conditioning phase
//!   configuration, and `EnvironmentalReading` trait implementation

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![deny(warnings)]

pub mod network;
pub mod sensor;
pub mod time;
