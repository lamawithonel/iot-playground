#![deny(unsafe_code)]
#![deny(warnings)]
//! Network configuration helper
//!
//! Waits for DHCP configuration to come up and logs the assigned
//! address.  Transport-agnostic: it operates only on an
//! `embassy_net::Stack`, so it works over any link driver (W5500
//! SPI, on-chip RMII, etc.)-- stack and device creation stay in the
//! board crate.

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
