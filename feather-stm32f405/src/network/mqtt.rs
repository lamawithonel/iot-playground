#![deny(warnings)]
#![allow(dead_code)] // Phase 2: Will be used when integrated into main.rs
//! MQTT v5.0 client implementation for Phase 2 network stack
//!
//! This module provides MQTT v5.0 client functionality using the `rust-mqtt` crate
//! with TLS 1.3 transport. It integrates with the existing TLS infrastructure.
//!
//! # Memory Management
//!
//! Uses bump allocator pattern from `rust-mqtt` for no_std compatibility:
//! - MQTT packet buffer: 2KB for packet assembly
//! - TLS buffers: 34KB total (managed by TLS module)
//! - TCP buffers: 8KB total (managed by TLS module)
//!
//! # Example
//!
//! ```no_run
//! let config = MqttConfig {
//!     broker_host: "192.168.1.1",
//!     broker_port: 8883,
//!     keep_alive_secs: 60,
//!     clean_start: true,
//! };
//! let mut client = MqttClient::new(config);
//! client.connect(stack, &mut rng).await?;
//! client.publish("device/test", b"Hello!", QoS::AtLeastOnce, false).await?;
//! ```

#![allow(unsafe_code)] // Required for TLS buffer access

use defmt::{debug, error, info, warn, Debug2Format};
use embassy_net::{dns::DnsQueryType, IpEndpoint, Stack};
use embedded_tls::{
    Aes128GcmSha256, CryptoProvider, NoVerify, TlsConfig, TlsConnection, TlsContext, TlsVerifier,
};
use rust_mqtt::{
    buffer::BumpBuffer,
    client::{options::ConnectOptions, Client},
    config::{KeepAlive, SessionExpiryInterval},
    types::MqttString,
};

use crate::{device_id, tls_buffers};

use super::error::NetworkError;
use super::socket::AsyncTcpSocket;

/// MQTT packet buffer size: 2KB for packet assembly
#[allow(dead_code)]
const MQTT_BUFFER_SIZE: usize = 2048;

/// Simple crypto provider that wraps an RNG for TLS operations
struct SimpleCryptoProvider<'a, RNG> {
    rng: &'a mut RNG,
    verifier: NoVerify,
}

impl<'a, RNG> SimpleCryptoProvider<'a, RNG> {
    fn new(rng: &'a mut RNG) -> Self {
        Self {
            rng,
            verifier: NoVerify,
        }
    }
}

impl<'a, RNG> CryptoProvider for SimpleCryptoProvider<'a, RNG>
where
    RNG: rand_core::CryptoRngCore,
{
    type CipherSuite = Aes128GcmSha256;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl rand_core::CryptoRngCore {
        &mut *self.rng
    }

    fn verifier(
        &mut self,
    ) -> Result<&mut impl TlsVerifier<Self::CipherSuite>, embedded_tls::TlsError> {
        Ok(&mut self.verifier)
    }
}

/// MQTT client configuration
#[derive(Clone, Copy)]
pub struct MqttConfig {
    /// Broker hostname (for DNS and SNI)
    pub broker_host: &'static str,
    /// Broker port (typically 8883 for MQTTS)
    pub broker_port: u16,
    /// Keep-alive interval in seconds
    pub keep_alive_secs: u16,
    /// Clean start flag (true = new session)
    pub clean_start: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "192.168.1.1",
            broker_port: 8883,
            keep_alive_secs: 60,
            clean_start: true,
        }
    }
}

/// MQTT v5.0 client
///
/// Manages MQTT connections over TLS 1.3. The client handles:
/// - Connection establishment with automatic TLS handshake
/// - Publishing messages with QoS 0, 1, or 2
/// - Keep-alive management
/// - Clean session handling
pub struct MqttClient {
    config: MqttConfig,
}

impl MqttClient {
    /// Create a new MQTT client with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// let config = MqttConfig {
    ///     broker_host: "192.168.1.1",
    ///     broker_port: 8883,
    ///     keep_alive_secs: 60,
    ///     clean_start: true,
    /// };
    /// let client = MqttClient::new(config);
    /// ```
    pub fn new(config: MqttConfig) -> Self {
        Self { config }
    }

    /// Connect to the MQTT broker over TLS 1.3
    ///
    /// This function:
    /// 1. Resolves the broker hostname via DNS
    /// 2. Establishes a TCP connection
    /// 3. Performs TLS 1.3 handshake
    /// 4. Sends MQTT CONNECT packet
    /// 5. Waits for CONNACK
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack for DNS and TCP operations
    /// * `rng` - Hardware random number generator (STM32F405 RNG peripheral)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if connection succeeds, or a `NetworkError` if any step fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut client = MqttClient::new(MqttConfig::default());
    /// client.connect(stack, &mut rng).await?;
    /// ```
    pub async fn connect<RNG>(
        &mut self,
        stack: &Stack<'static>,
        rng: &mut RNG,
    ) -> Result<(), NetworkError>
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
        info!(
            "Connecting to MQTT broker at {}:{}",
            self.config.broker_host, self.config.broker_port
        );

        // Step 1: DNS resolution
        let server_ip = stack
            .dns_query(self.config.broker_host, DnsQueryType::A)
            .await
            .map_err(|e| {
                error!("DNS query failed: {:?}", Debug2Format(&e));
                NetworkError::DnsError
            })?
            .first()
            .copied()
            .ok_or_else(|| {
                error!("DNS returned no results for {}", self.config.broker_host);
                NetworkError::DnsError
            })?;

        let endpoint = IpEndpoint::new(server_ip, self.config.broker_port);
        info!(
            "Resolved {} to {}",
            self.config.broker_host,
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
        let (read_buf, write_buf) = unsafe { tls_buffers::tls_buffers() };

        debug!(
            "TLS buffers allocated: read={} bytes, write={} bytes (main SRAM)",
            read_buf.len(),
            write_buf.len()
        );

        // Step 5: Configure TLS with server name for SNI
        let tls_config = TlsConfig::new().with_server_name(self.config.broker_host);

        // Step 6: Create TLS connection with buffers (using AES-128-GCM-SHA256)
        let mut tls_connection =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, read_buf, write_buf);

        // Step 7: Perform TLS handshake
        info!("Initiating TLS 1.3 handshake with hardware RNG...");
        let provider = SimpleCryptoProvider::new(rng);
        let tls_context = TlsContext::new(&tls_config, provider);

        tls_connection.open(tls_context).await.map_err(|e| {
            error!("TLS handshake failed: {:?}", Debug2Format(&e));
            NetworkError::TlsHandshakeFailed
        })?;

        info!("TLS 1.3 handshake completed successfully!");

        // Step 8: Establish MQTT connection
        let client_id = device_id::mqtt_client_id();
        info!("MQTT client ID: {}", client_id);

        // Allocate MQTT packet buffer using bump allocator
        let mut mqtt_buffer = [0u8; MQTT_BUFFER_SIZE];
        let mut buffer = BumpBuffer::new(&mut mqtt_buffer);
        let mut mqtt_client = Client::<'_, _, _, 1, 1, 1, 0>::new(&mut buffer);

        // Connect to MQTT broker
        let connect_opts = ConnectOptions {
            session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
            clean_start: self.config.clean_start,
            keep_alive: if self.config.keep_alive_secs == 0 {
                KeepAlive::Infinite
            } else {
                KeepAlive::Seconds(self.config.keep_alive_secs)
            },
            will: None,
            user_name: None,
            password: None,
        };

        // Convert client_id to MqttString
        let mqtt_client_id = MqttString::new(client_id.as_str().into()).map_err(|e| {
            error!(
                "Failed to create MQTT client ID string: {:?}",
                Debug2Format(&e)
            );
            NetworkError::MqttProtocolError
        })?;

        mqtt_client
            .connect(tls_connection, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT connect failed: {:?}", Debug2Format(&e));
                NetworkError::MqttConnectionFailed
            })?;

        info!("MQTT connection established successfully!");
        Ok(())
    }

    /// Publish a message to an MQTT topic
    ///
    /// # Arguments
    ///
    /// * `topic` - Topic string (e.g., "device/status")
    /// * `payload` - Message payload bytes
    /// * `qos` - Quality of Service level (0, 1, or 2)
    /// * `retain` - Whether to retain the message on the broker
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if publish succeeds, or a `NetworkError` if it fails.
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. In the actual implementation,
    /// we'll need to keep the MQTT client and connection alive across calls.
    pub async fn publish(
        &mut self,
        _topic: &str,
        _payload: &[u8],
        _qos: u8,
        _retain: bool,
    ) -> Result<(), NetworkError> {
        warn!("MQTT publish not yet fully implemented (placeholder)");
        Err(NetworkError::MqttPublishFailed)
    }

    /// Run MQTT client loop with periodic publishing
    ///
    /// This function establishes an MQTT connection and maintains it,
    /// publishing test messages every publish_interval_secs.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack for DNS and TCP operations
    /// * `rng` - Hardware random number generator
    /// * `publish_interval_secs` - Interval between publish messages
    ///
    /// # Note
    ///
    /// This function never returns under normal operation. It maintains
    /// the connection and publishes messages periodically.
    pub async fn run_with_periodic_publish<RNG>(
        &mut self,
        stack: &Stack<'static>,
        rng: &mut RNG,
        _publish_interval_secs: u64,
    ) -> Result<(), NetworkError>
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
        info!(
            "Connecting to MQTT broker at {}:{} for persistent connection",
            self.config.broker_host, self.config.broker_port
        );

        // Step 1: DNS resolution
        let server_ip = stack
            .dns_query(self.config.broker_host, DnsQueryType::A)
            .await
            .map_err(|e| {
                error!("DNS query failed: {:?}", Debug2Format(&e));
                NetworkError::DnsError
            })?
            .first()
            .copied()
            .ok_or_else(|| {
                error!("DNS returned no results for {}", self.config.broker_host);
                NetworkError::DnsError
            })?;

        let endpoint = IpEndpoint::new(server_ip, self.config.broker_port);
        info!(
            "Resolved {} to {}",
            self.config.broker_host,
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
        let (read_buf, write_buf) = unsafe { tls_buffers::tls_buffers() };

        debug!(
            "TLS buffers allocated: read={} bytes, write={} bytes (main SRAM)",
            read_buf.len(),
            write_buf.len()
        );

        // Step 5: Configure TLS with server name for SNI
        let tls_config = TlsConfig::new().with_server_name(self.config.broker_host);

        // Step 6: Create TLS connection with buffers (using AES-128-GCM-SHA256)
        let mut tls_connection =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, read_buf, write_buf);

        // Step 7: Perform TLS handshake
        info!("Initiating TLS 1.3 handshake with hardware RNG...");
        let provider = SimpleCryptoProvider::new(rng);
        let tls_context = TlsContext::new(&tls_config, provider);

        tls_connection.open(tls_context).await.map_err(|e| {
            error!("TLS handshake failed: {:?}", Debug2Format(&e));
            NetworkError::TlsHandshakeFailed
        })?;

        info!("TLS 1.3 handshake completed successfully!");

        // Step 8: Establish MQTT connection
        let client_id = device_id::mqtt_client_id();
        info!("MQTT client ID: {}", client_id);

        // Allocate MQTT packet buffer using bump allocator
        let mut mqtt_buffer = [0u8; MQTT_BUFFER_SIZE];
        let mut buffer = BumpBuffer::new(&mut mqtt_buffer);
        let mut mqtt_client = Client::<'_, _, _, 1, 1, 1, 0>::new(&mut buffer);

        // Connect to MQTT broker
        let connect_opts = ConnectOptions {
            session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
            clean_start: self.config.clean_start,
            keep_alive: if self.config.keep_alive_secs == 0 {
                KeepAlive::Infinite
            } else {
                KeepAlive::Seconds(self.config.keep_alive_secs)
            },
            will: None,
            user_name: None,
            password: None,
        };

        // Convert client_id to MqttString
        let mqtt_client_id = MqttString::new(client_id.as_str().into()).map_err(|e| {
            error!(
                "Failed to create MQTT client ID string: {:?}",
                Debug2Format(&e)
            );
            NetworkError::MqttProtocolError
        })?;

        mqtt_client
            .connect(tls_connection, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT connect failed: {:?}", Debug2Format(&e));
                NetworkError::MqttConnectionFailed
            })?;

        info!("MQTT connection established successfully!");
        info!("Persistent MQTT connection active - ready for publishing");

        // TODO: Implement publish loop with periodic messages
        // The rust-mqtt Client now owns the TLS connection and can be used for publishing
        // Example:
        // loop {
        //     Mono::delay(Duration::from_secs(publish_interval_secs)).await;
        //     let topic = format!("device/{}/test", client_id);
        //     let payload = b"Hello from STM32F405!";
        //     mqtt_client.publish(&topic, payload, QoS::AtLeastOnce, false).await?;
        // }

        warn!("MQTT publish loop not yet implemented - connection will be dropped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MqttConfig::default();
        assert_eq!(config.broker_host, "192.168.1.1");
        assert_eq!(config.broker_port, 8883);
        assert_eq!(config.keep_alive_secs, 60);
        assert!(config.clean_start);
    }
}
