#![deny(unsafe_code)]
#![deny(warnings)]
//! Network module: W5500 Ethernet with embassy-net
//!
//! This module contains network logic isolated from hardware setup:
//! - Hardware setup occurs in RTIC init() task
//! - Driver initialization happens in the network task
//! - Application logic is encapsulated in this module

use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use heapless::String;

use crate::time;

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

/// Run the network monitor task
/// This handles DHCP configuration, IP logging, message processing, and statistics
pub async fn run_network_monitor(stack: &Stack<'static>) {
    use defmt::info;

    let receiver = network_receiver();

    // Wait for network to come up
    stack.wait_config_up().await;
    info!("Network is UP!");

    // Log IP address
    if let Some(config) = stack.config_v4() {
        let ip = config.address.address();
        let octets = ip.octets();
        info!(
            "IP: {}.{}.{}.{}",
            octets[0], octets[1], octets[2], octets[3]
        );

        if let Some(gateway) = config.gateway {
            let gw_octets = gateway.octets();
            info!(
                "Gateway: {}.{}.{}.{}",
                gw_octets[0], gw_octets[1], gw_octets[2], gw_octets[3]
            );
        }
    }

    // Initialize SNTP time synchronization (SR-NET-006)
    // This will write the time to internal RTC (clocked by LSE)
    info!("Initializing SNTP time synchronization with RTC (LSE)...");
    match time::initialize_time(stack).await {
        Ok(ts) => {
            info!(
                "SNTP sync successful: {}.{:06} UTC (written to internal RTC)",
                ts.unix_secs, ts.micros
            );
        }
        Err(e) => {
            defmt::warn!("SNTP initialization failed: {:?}", e);
        }
    }

    // Main event loop
    let mut stats_timer = embassy_time::Ticker::every(embassy_time::Duration::from_secs(10));

    loop {
        // Use select to handle both channel messages and periodic stats
        embassy_futures::select::select(receiver.receive(), stats_timer.next()).await;

        // Check for messages
        if let Ok(msg) = receiver.try_receive() {
            match msg {
                NetworkMessage::LogFrame { data } => {
                    info!("Received frame: {}", data.as_str());
                }
            }
        }

        // Log network statistics
        if let Some(config) = stack.config_v4() {
            let ip = config.address.address();
            let octets = ip.octets();
            info!(
                "IP: {}.{}.{}.{}",
                octets[0], octets[1], octets[2], octets[3]
            );
        }
    }
}

/// Run the SNTP periodic resync task
/// This task syncs time every 15 minutes per SR-NET-007
pub async fn run_sntp_resync(stack: &Stack<'static>) -> ! {
    use defmt::info;
    use embassy_time::Timer;

    // Wait for initial sync to complete
    stack.wait_config_up().await;
    Timer::after_secs(30).await; // Give initial sync time to complete

    info!("SNTP resync task started (15-minute interval)");
    time::start_resync_task(stack).await // This function never returns
}
