#![deny(unsafe_code)]
#![deny(warnings)]
//! Network IP layer module
//!
//! This module handles IP-level networking operations including DHCP, DNS, and stack management.
//! It provides a clean abstraction over embassy-net for application use.

use defmt::info;
use embassy_net::Stack;

/// Wait for network to be configured and log the assigned IP address
pub async fn wait_for_config(stack: &Stack<'_>) {
    info!("Waiting for DHCP...");
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
}
