#![deny(unsafe_code)]
#![deny(warnings)]
//! Network module: W5500 Ethernet with embassy-net
//!
//! This module implements the "Init-Inside-Task" pattern:
//! - Raw peripherals (Send) are passed to the task
//! - Stack, Runner, and drivers (!Send) are constructed inside the task
//! - Maintains RTIC's SRP guarantees while using embassy-net

use heapless::String;

/// Messages that can be sent to the network task
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    /// Log a message (for demonstration)
    LogFrame { data: String<128> },
    /// Request SNTP synchronization
    SntpSync,
}

// Note: With rtic-sync, channels are created using make_channel! macro
// at the usage site, not as static globals. The sender/receiver are
// passed to tasks as needed.
