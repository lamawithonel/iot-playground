#![deny(unsafe_code)]
#![deny(warnings)]
//! Network stack manager
//!
//! Handles W5500 hardware initialization and embassy-net stack creation.
//! This module isolates hardware setup from application logic.

use defmt::info;
use embassy_net::Stack;

/// Wait for network configuration (DHCP) and log IP address
pub async fn wait_for_config(stack: &Stack<'_>) {
    info!("Waiting for DHCP...");
    stack.wait_config_up().await;
    info!("Network is UP!");

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
