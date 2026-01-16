#![deny(unsafe_code)]
#![deny(warnings)]
#![allow(dead_code)] // Phase 2: Will be used when integrated into main.rs
//! Device identifier utilities for STM32F405
//!
//! This module provides functions to retrieve and format the factory-programmed
//! 96-bit unique device ID from the STM32F405 microcontroller. This ID is stable
//! across reboots and unique to each chip.
//!
//! # Usage
//!
//! ```no_run
//! use feather_stm32f405::device_id;
//!
//! // Get the device ID as a hex string
//! let id_hex = device_id::uid_hex();
//! info!("Device UID: {}", id_hex);
//!
//! // Get formatted MQTT client ID
//! let client_id = device_id::mqtt_client_id();
//! info!("MQTT Client ID: {}", client_id);
//! ```

use defmt::Format;
use heapless::String;

/// Maximum length of the client ID string
/// Format: "stm32f405-" (10 chars) + 24 hex chars = 34 chars total
#[allow(dead_code)]
const CLIENT_ID_MAX_LEN: usize = 34;

/// Get the STM32F405 unique device ID as a hex string
///
/// Returns a 24-character hex string representing the 96-bit UID.
/// The UID is factory-programmed and unique to each STM32F405 chip.
///
/// # Example
///
/// ```no_run
/// let uid = device_id::uid_hex();
/// assert_eq!(uid.len(), 24);
/// ```
pub fn uid_hex() -> &'static str {
    embassy_stm32::uid::uid_hex()
}

/// Get the device UID bytes
///
/// Returns the raw 12-byte (96-bit) unique device ID.
///
/// # Example
///
/// ```no_run
/// let uid_bytes = device_id::uid();
/// assert_eq!(uid_bytes.len(), 12);
/// ```
pub fn uid() -> &'static [u8; 12] {
    embassy_stm32::uid::uid()
}

/// Generate an MQTT client ID from the device UID
///
/// Returns a client ID in the format `stm32f405-{24_hex_chars}`.
/// This provides a stable, unique identifier for MQTT connections.
///
/// # Example
///
/// ```no_run
/// let client_id = device_id::mqtt_client_id();
/// // Result: "stm32f405-0123456789abcdef01234567"
/// ```
pub fn mqtt_client_id() -> String<CLIENT_ID_MAX_LEN> {
    let uid = uid_hex();
    let mut client_id = String::<CLIENT_ID_MAX_LEN>::new();

    // Format: stm32f405-{uid}
    // These push_str calls cannot fail because:
    // - "stm32f405-" is 10 bytes
    // - uid is 24 bytes
    // - Total is 34 bytes, which exactly matches CLIENT_ID_MAX_LEN
    client_id.push_str("stm32f405-").expect("prefix should fit");
    client_id.push_str(uid).expect("UID should fit");

    client_id
}

/// Device identifier structure for formatting
///
/// Provides a defmt-compatible wrapper for device identifiers
#[derive(Clone, Copy, Format)]
pub struct DeviceId {
    uid: &'static [u8; 12],
}

impl DeviceId {
    /// Create a new DeviceId from the current device
    pub fn new() -> Self {
        Self { uid: uid() }
    }

    /// Get the UID as a hex string
    pub fn as_hex(&self) -> &'static str {
        uid_hex()
    }

    /// Get the MQTT client ID
    pub fn as_mqtt_client_id(&self) -> String<CLIENT_ID_MAX_LEN> {
        mqtt_client_id()
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_id_format() {
        // We can't test the actual UID in unit tests (requires hardware),
        // but we can verify the string capacity is correct
        let client_id = String::<CLIENT_ID_MAX_LEN>::new();
        assert!(client_id.capacity() >= "stm32f405-".len() + 24);
    }
}
