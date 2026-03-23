//! Network error types and MQTT formatting
//!
//! Platform-agnostic error enums, configuration types, and payload
//! formatting functions.  Hardware networking (TLS, TCP, DNS)
//! remains in the board crate.

pub mod error;
pub mod mqtt;
