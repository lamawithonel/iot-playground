#![deny(warnings)]
//! Network module with trait-based client architecture
//!
//! This module provides a modular network stack with:
//! - **`client`**: `NetworkClient` trait for protocol implementations
//! - **`config`**: Configuration structs with `Default` implementations
//! - **`error`**: Simple error enum for network operations
//! - **`manager`**: W5500/embassy-net stack initialization
//! - **`sntp`**: SNTP client implementing `NetworkClient`
//! - **`socket`**: Async TCP socket wrapper for embedded-io-async
//! - **`tls`**: TLS 1.3 client for secure communications
//!
//! ## Architecture
//!
//! The design follows the Open-Closed Principle: new protocols can be added
//! by implementing `NetworkClient` without modifying infrastructure code.
//!
//! ## Why not `embassy-net-driver` / `embassy-net-driver-channel`?
//!
//! These crates were evaluated but not directly used because:
//!
//! 1. **`embassy-net-driver`**: Provides the `Driver` trait that network devices
//!    implement. The W5500 driver (`embassy-net-wiznet`) already implements this
//!    trait internally, so we don't need to use it directly.
//!
//! 2. **`embassy-net-driver-channel`**: Provides a channel-based abstraction for
//!    network drivers, useful when you need to split RX/TX paths or implement
//!    custom buffering. The W5500 hardware handles its own buffering (8KB per
//!    socket), so this abstraction would add complexity without benefit.
//!
//! The current architecture uses `embassy-net-wiznet` directly, which provides
//! the W5500 device and runner. The `embassy-net` stack handles all TCP/IP
//! protocol processing, and applications use `embassy-net`'s socket APIs directly.

pub mod client;
pub mod config;
pub mod error;
pub mod manager;
pub mod sntp;
pub mod socket;
pub mod tls;

// Re-export commonly used types
pub use client::NetworkClient;
#[allow(unused_imports)]
pub use config::NetworkConfig;
#[allow(unused_imports)]
pub use config::SntpConfig;
#[allow(unused_imports)]
pub use error::NetworkError;
pub use sntp::SntpClient;
// TLS types are available but not re-exported yet (Phase 1)
// Will be added when integrated into main.rs
// pub use socket::AsyncTcpSocket;
// pub use tls::{TlsClient, TlsClientConfig};
