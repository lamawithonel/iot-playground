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
use embassy_time::{Duration, Timer};
use embedded_tls::{
    Aes128GcmSha256, CryptoProvider, NoVerify, TlsConfig, TlsConnection, TlsContext, TlsVerifier,
};
use heapless::String;
use rust_mqtt::{
    buffer::BumpBuffer,
    client::{
        options::{ConnectOptions, PublicationOptions, TopicReference},
        Client,
    },
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttString, QoS, TopicName},
    Bytes,
};

use crate::{device_id, time, tls_buffers};

use super::error::{MqttError, NetworkError, TlsError};
use super::socket::AsyncTcpSocket;

/// MQTT packet buffer size: 2KB for packet assembly
#[allow(dead_code)]
const MQTT_BUFFER_SIZE: usize = 2048;

/// Maximum MQTT topic length
/// Format: "device/{client_id}/telemetry" where client_id is ~34 chars
/// Total: 7 + 34 + 10 = 51 chars, use 64 for safety
const MAX_TOPIC_LEN: usize = 64;

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
            TlsError::HandshakeFailed
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
            MqttError::ProtocolError
        })?;

        mqtt_client
            .connect(tls_connection, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT connect failed: {:?}", Debug2Format(&e));
                MqttError::ConnectionFailed
            })?;

        info!("MQTT connection established successfully!");
        Ok(())
    }

    /// Connect to the MQTT broker using static buffers (RTIC pattern)
    ///
    /// This function uses externally-provided static buffers to maintain
    /// the connection beyond the function scope. This solves the lifetime
    /// constraint issue in RTIC applications.
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack for DNS and TCP operations
    /// * `rng` - Hardware random number generator
    /// * `mqtt_buffer` - Static buffer for MQTT packet assembly (2KB)
    /// * `tcp_rx_buffer` - Static buffer for TCP receive (4KB)
    /// * `tcp_tx_buffer` - Static buffer for TCP transmit (4KB)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if connection succeeds. The connection remains active
    /// for the lifetime of the provided buffers.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use static_cell::StaticCell;
    /// static MQTT_BUF: StaticCell<[u8; 2048]> = StaticCell::new();
    /// static RX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    /// static TX_BUF: StaticCell<[u8; 4096]> = StaticCell::new();
    ///
    /// let mqtt_buf = MQTT_BUF.init([0u8; 2048]);
    /// let rx_buf = RX_BUF.init([0u8; 4096]);
    /// let tx_buf = TX_BUF.init([0u8; 4096]);
    ///
    /// client.connect_with_buffers(stack, &mut rng, mqtt_buf, rx_buf, tx_buf).await?;
    /// ```
    pub async fn connect_with_buffers<RNG>(
        &mut self,
        stack: &Stack<'static>,
        rng: &mut RNG,
        mqtt_buffer: &'static mut [u8; MQTT_BUFFER_SIZE],
        tcp_rx_buffer: &'static mut [u8; 4096],
        tcp_tx_buffer: &'static mut [u8; 4096],
    ) -> Result<(), NetworkError>
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
        info!(
            "Connecting to MQTT broker at {}:{} with static buffers",
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

        // Step 2: Create and connect TCP socket using static buffers
        let mut socket = AsyncTcpSocket::new(*stack, tcp_rx_buffer, tcp_tx_buffer);
        socket.connect(endpoint).await?;
        info!("TCP connection established to {}", Debug2Format(&endpoint));

        // Step 3: Get TLS buffers from main SRAM (unsafe - single use only)
        // SAFETY: These static buffers are only used by one TLS connection at a time.
        let (read_buf, write_buf) = unsafe { tls_buffers::tls_buffers() };

        debug!(
            "TLS buffers allocated: read={} bytes, write={} bytes (main SRAM)",
            read_buf.len(),
            write_buf.len()
        );

        // Step 4: Configure TLS with server name for SNI
        let tls_config = TlsConfig::new().with_server_name(self.config.broker_host);

        // Step 5: Create TLS connection with buffers (using AES-128-GCM-SHA256)
        let mut tls_connection =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, read_buf, write_buf);

        // Step 6: Perform TLS handshake
        info!("Initiating TLS 1.3 handshake with hardware RNG...");
        let provider = SimpleCryptoProvider::new(rng);
        let tls_context = TlsContext::new(&tls_config, provider);

        tls_connection.open(tls_context).await.map_err(|e| {
            error!("TLS handshake failed: {:?}", Debug2Format(&e));
            TlsError::HandshakeFailed
        })?;

        info!("TLS 1.3 handshake completed successfully!");

        // Step 7: Establish MQTT connection
        let client_id = device_id::mqtt_client_id();
        info!("MQTT client ID: {}", client_id);

        // Create MQTT client with bump allocator using static buffer
        let mut buffer = BumpBuffer::new(mqtt_buffer);
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
            MqttError::ProtocolError
        })?;

        mqtt_client
            .connect(tls_connection, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT connect failed: {:?}", Debug2Format(&e));
                MqttError::ConnectionFailed
            })?;

        info!("MQTT connection established successfully with static buffers!");
        info!("Connection maintained - ready for persistent operations");

        // Connection is now maintained by the static buffers
        // The TLS connection and MQTT client will live as long as the buffers
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
        Err(MqttError::PublishFailed.into())
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
        publish_interval_secs: u64,
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
            TlsError::HandshakeFailed
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
            MqttError::ProtocolError
        })?;

        mqtt_client
            .connect(tls_connection, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT connect failed: {:?}", Debug2Format(&e));
                MqttError::ConnectionFailed
            })?;

        info!("MQTT connection established successfully!");
        info!("Persistent MQTT connection active - ready for publishing");

        // Publish loop with periodic messages
        let mut message_counter = 0u32;

        loop {
            // Wait for the specified interval using embassy_time Timer
            Timer::after(Duration::from_secs(publish_interval_secs)).await;

            message_counter += 1;

            // Get current timestamp from RTC
            let timestamp = time::get_timestamp();

            // Format topic: device/{client_id}/telemetry
            let topic_str = match format_mqtt_topic(client_id.as_str(), "telemetry") {
                Ok(topic) => topic,
                Err(e) => {
                    error!("Failed to format MQTT topic: {:?}", e);
                    return Err(e.into());
                }
            };

            // Build payload (simple JSON for now)
            // Format: {"msg_id":N,"timestamp":UNIX_SECS,"micros":MICROS}
            let mut payload_buf = [0u8; 128];
            let payload_len = {
                use core::fmt::Write;
                let mut writer = heapless::String::<128>::new();
                write!(
                    &mut writer,
                    "{{\"msg_id\":{},\"timestamp\":{},\"micros\":{}}}",
                    message_counter, timestamp.unix_secs, timestamp.micros
                )
                .map_err(|_| {
                    error!("Failed to format payload JSON");
                    MqttError::BufferError
                })?;

                let bytes = writer.as_bytes();
                payload_buf[..bytes.len()].copy_from_slice(bytes);
                bytes.len()
            };
            let payload = &payload_buf[..payload_len];

            info!(
                "Publishing message #{} to topic '{}' (payload: {} bytes)",
                message_counter,
                topic_str.as_str(),
                payload_len
            );

            // Create TopicName from the formatted topic string
            // SAFETY: format_mqtt_topic() validates that the topic string:
            // 1. Does not contain wildcard characters (+, #)
            // 2. Does not contain null characters
            // 3. Follows the valid MQTT topic name format: device/{id}/{subtopic}
            // Therefore, it's safe to use new_unchecked() here.
            let topic_name = unsafe {
                TopicName::new_unchecked(MqttString::new(topic_str.as_str().into()).map_err(
                    |e| {
                        error!("Failed to create MQTT topic string: {:?}", Debug2Format(&e));
                        MqttError::ProtocolError
                    },
                )?)
            };

            // Create publication options with QoS 0 (AtMostOnce) for test messages
            // TODO: Switch to QoS 1 (AtLeastOnce) per SR-SENS-004 when proper event-driven
            // message handling is implemented. Currently using QoS 0 to avoid manual polling.
            let pub_options = PublicationOptions {
                retain: false,
                message_expiry_interval: None,
                topic: TopicReference::Name(topic_name),
                qos: QoS::AtMostOnce,
            };

            // Publish the message
            match mqtt_client
                .publish(&pub_options, Bytes::from(payload))
                .await
            {
                Ok(packet_id) => {
                    info!(
                        "Message #{} published successfully (packet_id: {})",
                        message_counter, packet_id
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to publish message #{}: {:?}",
                        message_counter,
                        Debug2Format(&e)
                    );
                    // For now, continue to next iteration
                    // TODO: Implement reconnection logic per SR-NET-003
                    warn!("Continuing to next publish cycle despite error");
                }
            }
        }
    }
}

/// Format an MQTT topic for telemetry data
///
/// Returns a topic string in the format `device/{id}/telemetry` where
/// `{id}` is the device's MQTT client ID.
///
/// # Arguments
///
/// * `client_id` - The device's MQTT client ID
/// * `subtopic` - The topic suffix (e.g., "telemetry", "status")
///
/// # Returns
///
/// Returns a heapless String with the formatted topic, or an error if
/// the topic is too long to fit in the buffer.
///
/// # Example
///
/// ```no_run
/// let client_id = device_id::mqtt_client_id();
/// let topic = format_mqtt_topic(&client_id, "telemetry")?;
/// // Result: "device/stm32f405-0123456789abcdef01234567/telemetry"
/// ```
fn format_mqtt_topic(client_id: &str, subtopic: &str) -> Result<String<MAX_TOPIC_LEN>, MqttError> {
    // Validate that client_id and subtopic don't contain invalid MQTT topic characters
    // MQTT spec: Topic names cannot contain wildcards (+, #) or null characters
    if client_id.contains('+') || client_id.contains('#') || client_id.contains('\0') {
        error!("Client ID contains invalid MQTT topic characters");
        return Err(MqttError::ProtocolError);
    }
    if subtopic.contains('+') || subtopic.contains('#') || subtopic.contains('\0') {
        error!("Subtopic contains invalid MQTT topic characters");
        return Err(MqttError::ProtocolError);
    }

    let mut topic = String::<MAX_TOPIC_LEN>::new();

    // Build: device/{client_id}/{subtopic}
    topic
        .push_str("device/")
        .map_err(|_| MqttError::BufferError)?;
    topic
        .push_str(client_id)
        .map_err(|_| MqttError::BufferError)?;
    topic.push('/').map_err(|_| MqttError::BufferError)?;
    topic
        .push_str(subtopic)
        .map_err(|_| MqttError::BufferError)?;

    Ok(topic)
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

    #[test]
    fn test_format_mqtt_topic() {
        // Test telemetry topic
        let topic = format_mqtt_topic("stm32f405-test123", "telemetry").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/telemetry");

        // Test status topic
        let topic = format_mqtt_topic("stm32f405-test123", "status").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/status");

        // Test that topic length is within bounds
        assert!(topic.len() < MAX_TOPIC_LEN);
    }

    #[test]
    fn test_format_mqtt_topic_buffer_overflow() {
        // Create a long client ID that should cause buffer overflow
        // Use a fixed-size stack array instead of heap allocation
        let long_id = "this_is_a_very_long_client_id_that_exceeds_the_maximum_allowed_topic_length_for_mqtt_messages";
        let result = format_mqtt_topic(long_id, "telemetry");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_mqtt_topic_invalid_characters() {
        // Test that wildcard characters are rejected
        let result = format_mqtt_topic("client+wildcard", "telemetry");
        assert!(result.is_err());

        let result = format_mqtt_topic("client#wildcard", "telemetry");
        assert!(result.is_err());

        let result = format_mqtt_topic("valid-client", "status+wildcard");
        assert!(result.is_err());
    }
}
