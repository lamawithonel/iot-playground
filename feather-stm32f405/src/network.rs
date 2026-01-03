#![deny(unsafe_code)]
#![deny(warnings)]
//! Network module: W5500 Ethernet with embassy-net
//!
//! This module implements the "Init-Inside-Task" pattern:
//! - Raw peripherals (Send) are passed to the task
//! - Stack, Runner, and drivers (!Send) are constructed inside the task
//! - Maintains RTIC's SRP guarantees while using embassy-net

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use heapless::String;

/// Messages that can be sent to the network task
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    /// Log a message (for demonstration)
    LogFrame { data: String<128> },
}

/// Channel for sending messages to network task
/// Using CriticalSectionRawMutex makes it safe across all RTIC priorities
pub static NETWORK_CHANNEL: Channel<CriticalSectionRawMutex, NetworkMessage, 8> = Channel::new();

/// Get a sender for the network channel (can be called from any task)
pub fn network_sender() -> Sender<'static, CriticalSectionRawMutex, NetworkMessage, 8> {
    NETWORK_CHANNEL.sender()
}

/// Get a receiver for the network channel (used inside network task)
pub fn network_receiver() -> Receiver<'static, CriticalSectionRawMutex, NetworkMessage, 8> {
    NETWORK_CHANNEL.receiver()
}
