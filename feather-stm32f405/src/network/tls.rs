#![deny(warnings)]
//! TLS 1.3 client implementation using embedded-tls
//!
//! This module provides TLS 1.3 client functionality for secure MQTT communication.
//! It uses the `embedded-tls` crate which is designed for no_std environments.
//!
//! # Safety Note
//!
//! This module uses `unsafe` code to access CCM RAM buffers for TLS operations.
//! The unsafe code is isolated to buffer access and is carefully reviewed and documented.
//! All other code follows safe Rust practices.
//!
//! # Phase 1 Limitations
//!
//! - Certificate verification is disabled (set `verify_server: false`)
//! - Single connection at a time (due to static CCM RAM buffers)
//! - Test server: `test.mosquitto.org:8883`
//!
//! # Memory Usage
//!
//! - TLS read buffer: 16 KB in CCM RAM
//! - TLS write buffer: 16 KB in CCM RAM
//! - TCP socket buffers: 8 KB in main SRAM (4 KB RX + 4 KB TX)

#![allow(unsafe_code)] // Required for CCM RAM buffer access

use defmt::{debug, error, info, warn, Debug2Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::{IpEndpoint, Stack};
use embedded_tls::{Aes128GcmSha256, NoVerify, TlsConfig, TlsConnection, TlsContext};

use crate::ccmram;

use super::error::NetworkError;
use super::socket::AsyncTcpSocket;

/// Simple counter-based RNG for Phase 1 testing
///
/// WARNING: This is NOT cryptographically secure and should be replaced
/// with a hardware RNG (STM32F4 has RNG peripheral) for production use.
///
/// This implementation uses a simple counter that increments on each random
/// number generation. It satisfies the RngCore trait requirements for embedded-tls
/// but does not provide cryptographic security.
#[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
struct SimpleRng {
    counter: u64,
}

impl SimpleRng {
    #[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
    fn new() -> Self {
        // TODO: Use STM32F4 hardware RNG peripheral for production
        // For Phase 1 testing, initialize with a fixed but non-zero seed
        // This provides reproducible behavior for debugging while still
        // satisfying the RngCore trait requirements
        Self {
            counter: 0x0123_4567_89AB_CDEF,
        }
    }
}

impl rand_core::RngCore for SimpleRng {
    fn next_u32(&mut self) -> u32 {
        // Increment using golden ratio to ensure good distribution
        // 0x9E3779B97F4A7C15 is the 64-bit fractional part of the golden ratio
        // This constant is commonly used in hash functions for good avalanche properties
        self.counter = self.counter.wrapping_add(0x9E3779B97F4A7C15);
        (self.counter >> 32) as u32
    }

    fn next_u64(&mut self) -> u64 {
        // Increment using golden ratio constant for good distribution
        self.counter = self.counter.wrapping_add(0x9E3779B97F4A7C15);
        self.counter
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let random = self.next_u64();
            let bytes = random.to_le_bytes();
            let len = chunk.len().min(8);
            chunk[..len].copy_from_slice(&bytes[..len]);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl rand_core::CryptoRng for SimpleRng {}

/// TLS client configuration
#[derive(Clone, Copy)]
#[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
pub struct TlsClientConfig {
    /// Server hostname for SNI (Server Name Indication)
    pub server_name: &'static str,
    /// Server port (typically 8883 for MQTTS)
    pub server_port: u16,
    /// Enable certificate verification (false for Phase 1 testing)
    pub verify_server: bool,
}

impl Default for TlsClientConfig {
    fn default() -> Self {
        Self {
            server_name: "test.mosquitto.org",
            server_port: 8883,
            verify_server: false, // Phase 1: skip cert verification
        }
    }
}

/// TLS 1.3 client
///
/// Manages TLS handshake and secure communication over TCP.
/// Uses static CCM RAM buffers for zero-wait-state access.
#[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
pub struct TlsClient {
    config: TlsClientConfig,
}

impl TlsClient {
    /// Create a new TLS client with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = TlsClientConfig {
    ///     server_name: "test.mosquitto.org",
    ///     server_port: 8883,
    ///     verify_server: false,
    /// };
    /// let client = TlsClient::new(config);
    /// ```
    #[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
    pub fn new(config: TlsClientConfig) -> Self {
        Self { config }
    }

    /// Perform TLS handshake test with the configured server
    ///
    /// This function:
    /// 1. Resolves the server hostname via DNS
    /// 2. Establishes a TCP connection
    /// 3. Performs the TLS 1.3 handshake
    /// 4. Closes the connection
    ///
    /// # Phase 1 Limitation
    ///
    /// This is a test function that establishes the connection and immediately closes it.
    /// It demonstrates that TLS 1.3 handshake works but doesn't return a usable connection.
    /// A future version will manage buffers differently to allow returning connections.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack for DNS and TCP operations
    ///
    /// # Returns
    ///
    /// `Ok(())` if handshake succeeds, or a `NetworkError` if any step fails.
    ///
    /// # Safety
    ///
    /// This function uses unsafe code to access static CCM RAM buffers.
    /// These buffers must only be used by a single TLS connection at a time.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let client = TlsClient::new(TlsClientConfig::default());
    /// client.test_handshake(stack).await?;
    /// ```
    #[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
    pub async fn test_handshake(&self, stack: &Stack<'static>) -> Result<(), NetworkError> {
        info!(
            "Starting TLS handshake test with {}:{}",
            self.config.server_name, self.config.server_port
        );

        // Step 1: DNS resolution
        let server_ip = stack
            .dns_query(self.config.server_name, DnsQueryType::A)
            .await
            .map_err(|e| {
                error!("DNS query failed: {:?}", Debug2Format(&e));
                NetworkError::DnsError
            })?
            .first()
            .copied()
            .ok_or_else(|| {
                error!("DNS returned no results for {}", self.config.server_name);
                NetworkError::DnsError
            })?;

        let endpoint = IpEndpoint::new(server_ip, self.config.server_port);
        info!(
            "Resolved {} to {}",
            self.config.server_name,
            Debug2Format(&endpoint)
        );

        // Step 2: Allocate TCP socket buffers (in main SRAM, not CCM)
        let mut rx_buffer = [0u8; 4096];
        let mut tx_buffer = [0u8; 4096];

        // Step 3: Create and connect TCP socket
        let mut socket = AsyncTcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
        socket.connect(endpoint).await?;
        info!("TCP connection established to {}", Debug2Format(&endpoint));

        // Step 4: Get TLS buffers from CCM RAM (unsafe - single use only)
        // SAFETY: These static buffers are only used by one TLS connection at a time.
        // The buffers are obtained once and used for the duration of this function.
        let (read_buf, write_buf) = unsafe { ccmram::tls_buffers() };

        debug!(
            "TLS buffers allocated: read={} bytes, write={} bytes (CCM RAM)",
            read_buf.len(),
            write_buf.len()
        );

        // Step 5: Configure TLS with server name for SNI
        let config = TlsConfig::new().with_server_name(self.config.server_name);

        if self.config.verify_server {
            warn!("Certificate verification requested but not yet implemented");
            warn!("Phase 1: proceeding without verification");
        }

        // Step 6: Create TLS connection with buffers
        let mut tls_connection =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, read_buf, write_buf);

        // Step 7: Create a simple RNG for TLS handshake
        let mut rng = SimpleRng::new();

        // Step 8: Create TLS context and perform handshake
        info!("Initiating TLS 1.3 handshake...");
        let tls_context = TlsContext::new(&config, &mut rng);

        tls_connection
            .open::<SimpleRng, NoVerify>(tls_context)
            .await
            .map_err(|e| {
                error!("TLS handshake failed: {:?}", Debug2Format(&e));
                NetworkError::TlsHandshakeFailed
            })?;

        info!("TLS 1.3 handshake completed successfully!");

        // Step 9: Close the connection
        tls_connection.close().await.map_err(|(_socket, e)| {
            warn!("TLS close returned error: {:?}", Debug2Format(&e));
            NetworkError::TlsConnectionClosed
        })?;

        info!("TLS connection closed cleanly");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TlsClientConfig::default();
        assert_eq!(config.server_name, "test.mosquitto.org");
        assert_eq!(config.server_port, 8883);
        assert!(!config.verify_server);
    }
}
