#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client error types
//!
//! Re-exports from [`iot_core::network::error`].  All error
//! definitions live in the platform-agnostic core crate.

pub use iot_core::network::error::{MqttError, NetworkError, SntpError, TlsError};
