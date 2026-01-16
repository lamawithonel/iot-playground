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
//! - Test server: `broker.emqx.io:8883` (public MQTT broker with TLS support)
//!
//! # Memory Usage
//!
//! - TLS read buffer: 18 KB in main SRAM (see `src/tls_buffers.rs`)
//! - TLS write buffer: 16 KB in main SRAM (see `src/tls_buffers.rs`)
//! - TCP socket buffers: 8 KB in main SRAM (4 KB RX + 4 KB TX)

#![allow(unsafe_code)] // Required for static TLS buffer access

use defmt::{debug, error, info, warn, Debug2Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::{IpEndpoint, Stack};
use embedded_tls::{
    Aes128GcmSha256, CryptoProvider, NoVerify, TlsConfig, TlsConnection, TlsContext, TlsVerifier,
};

use crate::tls_buffers;

use super::error::{NetworkError, TlsError};
use super::socket::AsyncTcpSocket;

/// Simple crypto provider that wraps an RNG for TLS operations
struct SimpleCryptoProvider<RNG> {
    rng: RNG,
    verifier: NoVerify,
}

impl<RNG> SimpleCryptoProvider<RNG> {
    fn new(rng: RNG) -> Self {
        Self {
            rng,
            verifier: NoVerify,
        }
    }
}

impl<RNG> CryptoProvider for SimpleCryptoProvider<RNG>
where
    RNG: rand_core::CryptoRngCore,
{
    type CipherSuite = Aes128GcmSha256;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl rand_core::CryptoRngCore {
        &mut self.rng
    }

    fn verifier(
        &mut self,
    ) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, embedded_tls::TlsError> {
        Ok(&mut self.verifier)
    }
}

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
            server_name: "broker.emqx.io",
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
    ///     server_name: "broker.emqx.io",
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
    /// 3. Performs the TLS 1.3 handshake using hardware RNG
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
    /// * `rng` - Hardware random number generator (STM32F405 RNG peripheral)
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
    /// client.test_handshake(stack, &mut rng).await?;
    /// ```
    #[allow(dead_code)] // Phase 1: Will be used when TLS is integrated
    pub async fn test_handshake<RNG>(
        &self,
        stack: &Stack<'static>,
        rng: &mut RNG,
    ) -> Result<(), NetworkError>
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
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

        // Step 4: Get TLS buffers from main SRAM (unsafe - single use only)
        // SAFETY: These static buffers are only used by one TLS connection at a time.
        // The buffers are obtained once and used for the duration of this function.
        let (read_buf, write_buf) = unsafe { tls_buffers::tls_buffers() };

        debug!(
            "TLS buffers allocated: read={} bytes, write={} bytes (main SRAM)",
            read_buf.len(),
            write_buf.len()
        );

        // Step 5: Configure TLS with server name for SNI
        let config = TlsConfig::new().with_server_name(self.config.server_name);

        if self.config.verify_server {
            warn!("Certificate verification requested but not yet implemented");
            warn!("Phase 1: proceeding without verification");
        }

        // Step 6: Create TLS connection with buffers (using AES-128-GCM-SHA256)
        let mut tls_connection =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, read_buf, write_buf);

        // Step 7: Create crypto provider and TLS context, then perform handshake
        info!("Initiating TLS 1.3 handshake with hardware RNG...");
        let provider = SimpleCryptoProvider::new(rng);
        let tls_context = TlsContext::new(&config, provider);

        tls_connection.open(tls_context).await.map_err(|e| {
            error!("TLS handshake failed: {:?}", Debug2Format(&e));
            TlsError::HandshakeFailed
        })?;

        info!("TLS 1.3 handshake completed successfully!");

        // Step 8: Close the connection
        tls_connection.close().await.map_err(|(_socket, e)| {
            warn!("TLS close returned error: {:?}", Debug2Format(&e));
            TlsError::ConnectionClosed
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
        assert_eq!(config.server_name, "broker.emqx.io");
        assert_eq!(config.server_port, 8883);
        assert!(!config.verify_server);
    }
}
