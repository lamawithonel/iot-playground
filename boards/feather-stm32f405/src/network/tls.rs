#![deny(unsafe_code)]
#![deny(warnings)]
//! TLS 1.3 configuration for embedded-tls
//!
//! TLS transport is handled inline by the MQTT client (see `mqtt.rs`),
//! which configures `embedded-tls` directly.  This module exists only
//! to hold shared configuration constants and types that may be used
//! by future protocol clients.
//!
//! # Cipher Suite
//!
//! AES-128-GCM-SHA256 — the only cipher suite supported by
//! `embedded-tls` and the mandatory-to-implement suite for TLS 1.3.
//!
//! # Certificate Verification
//!
//! Phase 2 uses `NoVerify`.  Phase 4 (AWS IoT readiness) will add
//! certificate verification with a pinned CA bundle.
//!
//! # Memory
//!
//! TLS read/write buffers are in CCM RAM (see `ccmram.rs`).
//! TCP socket buffers are caller-provided (typically `StaticCell`).

/// Default TLS port for MQTTS
pub const MQTTS_PORT: u16 = 8883;

/// Maximum backoff delay in seconds for TLS/MQTT reconnection
pub const MAX_RECONNECT_BACKOFF_SECS: u64 = 60;

/// Initial backoff delay in seconds for TLS/MQTT reconnection
pub const INITIAL_RECONNECT_BACKOFF_SECS: u64 = 5;
