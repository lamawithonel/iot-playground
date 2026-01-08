#![deny(unsafe_code)]
#![deny(warnings)]
//! UDP network layer module
//!
//! This module provides a clean abstraction over embassy-net UDP operations.
//! It handles UDP socket creation, sending, and receiving with proper error handling.

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
    /// Local port to bind to (0 for ephemeral port)
    pub local_port: u16,
    /// Size of receive buffer (default 256 bytes)
    pub rx_buffer_size: usize,
    /// Size of transmit buffer (default 256 bytes)
    pub tx_buffer_size: usize,
    /// Number of packet metadata entries (default 4)
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
/// This is a convenience function for one-shot UDP sends.
/// For multiple operations, create a socket with `create_socket_with_config` instead.
#[allow(dead_code)]
pub async fn send_to(stack: &Stack<'_>, data: &[u8], endpoint: IpEndpoint) -> Result<(), UdpError> {
    info!("Sending {} bytes to UDP {:?}", data.len(), endpoint);

    // Create temporary socket with minimal buffers
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

    // Bind to ephemeral port
    socket.bind(0).map_err(|e| {
        warn!("UDP bind failed: {:?}", e);
        UdpError::BindFailed
    })?;

    // Send data
    socket.send_to(data, endpoint).await.map_err(|e| {
        warn!("UDP send failed: {:?}", e);
        UdpError::SendFailed
    })?;

    info!("UDP packet sent successfully");
    Ok(())
}

/// Receive a UDP packet with timeout
///
/// This is a convenience function for one-shot UDP receives.
/// Returns the received data length and source endpoint.
///
/// Note: This function uses a Vec for the internal buffer, which requires
/// an allocator. For no_std embedded use without alloc, use the lower-level
/// `create_socket_with_config` function and manage buffers manually.
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

    // Create temporary socket with stack-allocated buffers
    let mut rx_meta = [PacketMetadata::EMPTY; 2];
    let mut rx_buffer = [0u8; 512]; // Fixed size buffer on stack
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 64];

    let mut socket = UdpSocket::new(
        *stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    // Bind to specified port
    socket.bind(local_port).map_err(|e| {
        warn!("UDP bind to port {} failed: {:?}", local_port, e);
        UdpError::BindFailed
    })?;

    // Receive with timeout
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
///
/// Returns a configured UdpSocket bound to the specified local port.
/// The caller is responsible for managing the socket lifetime and buffers.
///
/// # Example
/// ```no_run
/// let config = UdpConfig {
///     local_port: 8080,
///     ..Default::default()
/// };
/// let mut rx_meta = [PacketMetadata::EMPTY; 4];
/// let mut rx_buf = [0u8; 512];
/// let mut tx_meta = [PacketMetadata::EMPTY; 4];
/// let mut tx_buf = [0u8; 512];
/// let socket = create_socket_with_config(
///     stack, config, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf
/// )?;
/// ```
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
