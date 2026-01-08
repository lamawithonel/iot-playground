#![deny(unsafe_code)]
#![deny(warnings)]
//! Network module: Messages for network operations
//!
//! This module defines message types for inter-task communication
//! related to network operations.

/// Messages that can be sent to the network task
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    /// Request SNTP synchronization
    SntpSync,
}
