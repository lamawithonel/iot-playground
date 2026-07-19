//! Transport-agnostic network protocol clients
//!
//! Shared MQTT/TLS/SNTP/DNS client logic over `embassy-net`,
//! extracted from the Feather board crate so any board profile
//! (F4, H7, future ARS reporting nodes) can reuse it.  This crate
//! is hardware-free: it depends on `embassy-net` for the socket
//! and DNS APIs but never on `embassy-stm32`, a PAC, or a board
//! crate.  All hardware concerns (buffer placement, RTC writes,
//! wall-clock calibration, telemetry counters) are injected by the
//! caller through buffers and closures.
//!
//! # Modules
//!
//! - [`client`]: `NetworkClient` trait for protocol implementations
//! - [`config`]: Configuration structs with `Default` implementations
//! - [`error`]: Re-exports of `iot_core`'s canonical error enums
//! - [`manager`]: DHCP wait/log helper for `embassy-net` stacks
//! - [`mqtt`]: MQTT v5.0 client over TLS 1.3 with reconnection
//! - [`sntp`]: SNTP client implementing `NetworkClient`
//! - [`socket`]: Async TCP socket wrapper for embedded-io-async
//! - [`tls`]: TLS 1.3 configuration constants

#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

pub mod client;
pub mod config;
pub mod error;
pub mod manager;
pub mod mqtt;
pub mod sntp;
pub mod socket;
pub mod tls;

pub use client::NetworkClient;
pub use config::SntpConfig;
pub use error::{MqttError, NetworkError, SntpError, TlsError};
pub use mqtt::{MqttBuffers, MqttClient, MqttConfig};
pub use sntp::SntpClient;
