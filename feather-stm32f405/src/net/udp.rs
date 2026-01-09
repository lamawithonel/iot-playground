#![deny(unsafe_code)]
#![deny(warnings)]
//! UDP network layer module

use defmt::{info, warn};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_time::{Duration, Timer};

/// UDP socket error types
#[derive(Debug, Clone, Copy, defmt::Format)]
#[allow(dead_code)]
pub enum UdpError {
    /// Failed to bind socket to local port
    BindFailed,
    /// Failed to send data
    SendFailed,
    /// Failed to receive data
    ReceiveFailed,
    /// Operation timed out
    Timeout,
    /// Invalid endpoint
    InvalidEndpoint,
}

/// UDP socket configuration
#[allow(dead_code)]
pub struct UdpConfig {
    pub local_port: u16,
    pub rx_buffer_size: usize,
    pub tx_buffer_size: usize,
    pub packet_metadata_count: usize,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            local_port: 0,
            rx_buffer_size: 256,
            tx_buffer_size: 256,
            packet_metadata_count: 4,
        }
    }
}

/// Send a UDP packet to a remote endpoint
///
/// Convenience function for one-shot UDP sends.
#[allow(dead_code)]
pub async fn send_to(stack: &Stack<'_>, data: &[u8], endpoint: IpEndpoint) -> Result<(), UdpError> {
    info!("Sending {} bytes to UDP {:?}", data.len(), endpoint);
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0u8; 64];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 512];

    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    socket.bind(0).map_err(|e| {
        warn!("UDP bind failed: {:?}", e);
        UdpError::BindFailed
    })?;

    socket.send_to(data, endpoint).await.map_err(|e| {
        warn!("UDP send failed: {:?}", e);
        UdpError::SendFailed
    })?;

    info!("UDP packet sent successfully");
    Ok(())
}

/// Receive a UDP packet with timeout
///
/// Convenience function for one-shot UDP receives.
/// Returns the received data length and source endpoint.
#[allow(dead_code)]
pub async fn recv_from_with_timeout(
    stack: &Stack<'_>,
    local_port: u16,
    buffer: &mut [u8],
    timeout_ms: u64,
) -> Result<(usize, IpEndpoint), UdpError> {
    info!(
        "Waiting for UDP packet on port {} (timeout: {}ms)",
        local_port, timeout_ms
    );

    let mut rx_meta = [PacketMetadata::EMPTY; 2];
    let mut rx_buffer = [0u8; 512];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 64];

    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    socket.bind(local_port).map_err(|e| {
        warn!("UDP bind to port {} failed: {:?}", local_port, e);
        UdpError::BindFailed
    })?;

    let timeout_future = Timer::after(Duration::from_millis(timeout_ms));
    let recv_future = socket.recv_from(buffer);

    let (recv_len, from_addr) =
        match embassy_futures::select::select(timeout_future, recv_future).await {
            embassy_futures::select::Either::First(_) => {
                warn!("UDP receive timeout after {}ms", timeout_ms);
                return Err(UdpError::Timeout);
            }
            embassy_futures::select::Either::Second(result) => result.map_err(|e| {
                warn!("UDP receive failed: {:?}", e);
                UdpError::ReceiveFailed
            })?,
        };

    info!(
        "Received {} bytes from UDP {:?}",
        recv_len, from_addr.endpoint
    );
    Ok((recv_len, from_addr.endpoint))
}

/// Create a UDP socket with custom configuration
#[allow(dead_code)]
pub fn create_socket_with_config<'a>(
    stack: &Stack<'a>,
    config: UdpConfig,
    rx_meta: &'a mut [PacketMetadata],
    rx_buffer: &'a mut [u8],
    tx_meta: &'a mut [PacketMetadata],
    tx_buffer: &'a mut [u8],
) -> Result<UdpSocket<'a>, UdpError> {
    info!(
        "Creating UDP socket on port {} (rx_buf={}, tx_buf={})",
        config.local_port,
        rx_buffer.len(),
        tx_buffer.len()
    );

    let mut socket = UdpSocket::new(*stack, rx_meta, rx_buffer, tx_meta, tx_buffer);

    socket.bind(config.local_port).map_err(|e| {
        warn!("UDP bind to port {} failed: {:?}", config.local_port, e);
        UdpError::BindFailed
    })?;

    info!("UDP socket created and bound successfully");
    Ok(socket)
}
