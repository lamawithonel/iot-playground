#![deny(warnings)]
//! MQTT v5.0 client with persistent connection and reconnection
//!
//! Provides a single-connection MQTT client over TLS 1.3 that:
//! - Accepts caller-provided static buffers (RTIC `StaticCell` pattern)
//! - Maintains a persistent connection with periodic publishing
//! - Reconnects with exponential backoff on broker disconnection
//!
//! # Memory Management
//!
//! All buffers are caller-provided to keep lifetimes explicit:
//! - **MQTT packet buffer**: 2 KB for `rust-mqtt` bump allocator
//! - **TCP RX/TX buffers**: 4 KB each (main SRAM, via `StaticCell`)
//! - **TLS read/write buffers**: 34 KB total in CCM RAM (via
//!   `tls_buffers`)
//!
//! # Usage
//!
//! ```no_run
//! let mut client = MqttClient::new(MqttConfig::default());
//! // Never returns under normal operation
//! client.run(stack, &mut rng, mqtt_buf, rx_buf, tx_buf, 30).await;
//! ```

#![deny(unsafe_code)]

use defmt::{error, info, warn, Debug2Format};
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
use super::tls;

/// MQTT packet buffer size: 2 KB for packet assembly
const MQTT_BUFFER_SIZE: usize = 2048;

/// Maximum MQTT topic length
///
/// Format: `device/{client_id}/telemetry` where client_id is ~34
/// chars.  Total: 7 + 34 + 10 = 51 chars; 64 provides safety margin.
const MAX_TOPIC_LEN: usize = 64;

/// Simple crypto provider wrapping an RNG for embedded-tls
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
    /// Broker hostname (for DNS and TLS SNI)
    pub broker_host: &'static str,
    /// Broker port (typically 8883 for MQTTS)
    pub broker_port: u16,
    /// MQTT keep-alive interval in seconds (0 = infinite)
    pub keep_alive_secs: u16,
    /// Clean start flag (true = new session)
    pub clean_start: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_host: "192.168.1.1",
            broker_port: tls::MQTTS_PORT,
            keep_alive_secs: 60,
            clean_start: true,
        }
    }
}

/// MQTT v5.0 client with persistent connection
///
/// Manages the full lifecycle: DNS → TCP → TLS 1.3 → MQTT CONNECT →
/// publish loop, with automatic reconnection on failure.
pub struct MqttClient {
    config: MqttConfig,
}

impl MqttClient {
    /// Create a new MQTT client with the given configuration
    pub fn new(config: MqttConfig) -> Self {
        Self { config }
    }

    /// Run the MQTT client forever with periodic publishing
    ///
    /// Establishes a TLS+MQTT connection, then publishes telemetry
    /// at `publish_interval_secs`.  On any failure, reconnects with
    /// exponential backoff (5 s → 60 s cap).
    ///
    /// # Arguments
    ///
    /// * `stack` — Embassy network stack
    /// * `rng` — Hardware RNG (STM32F405 RNG peripheral)
    /// * `mqtt_buffer` — 2 KB buffer for `rust-mqtt` packet assembly
    /// * `tcp_rx_buffer` — 4 KB TCP receive buffer
    /// * `tcp_tx_buffer` — 4 KB TCP transmit buffer
    /// * `publish_interval_secs` — Seconds between telemetry publishes
    ///
    /// # Safety
    ///
    /// Accesses CCM RAM TLS buffers via `tls_buffers::tls_buffers()`.
    /// Only one TLS connection may use those buffers at a time.
    pub async fn run<RNG>(
        &mut self,
        stack: &Stack<'static>,
        rng: &mut RNG,
        mqtt_buffer: &mut [u8; MQTT_BUFFER_SIZE],
        tcp_rx_buffer: &mut [u8],
        tcp_tx_buffer: &mut [u8],
        publish_interval_secs: u64,
    ) -> !
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
        let mut backoff_secs = tls::INITIAL_RECONNECT_BACKOFF_SECS;

        loop {
            info!(
                "Connecting to MQTT broker at {}:{}...",
                self.config.broker_host, self.config.broker_port
            );

            match self
                .run_session(
                    stack,
                    rng,
                    mqtt_buffer,
                    tcp_rx_buffer,
                    tcp_tx_buffer,
                    publish_interval_secs,
                )
                .await
            {
                Ok(()) => {
                    // run_session's inner loop is infinite; this path
                    // is unreachable in practice but satisfies the
                    // compiler.
                    warn!("MQTT session ended unexpectedly");
                    backoff_secs = tls::INITIAL_RECONNECT_BACKOFF_SECS;
                }
                Err(NetworkError::Mqtt(MqttError::Disconnected)) => {
                    // Was connected, then lost — reset backoff so
                    // the next reconnect starts quickly.
                    error!("MQTT session disconnected");
                    backoff_secs = tls::INITIAL_RECONNECT_BACKOFF_SECS;
                }
                Err(e) => {
                    // Connection never established (DNS, TCP, TLS,
                    // or CONNECT failure) — escalate backoff.
                    error!("MQTT session failed: {:?}", e);
                }
            }

            info!("Reconnecting in {} seconds (backoff)...", backoff_secs);
            Timer::after(Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(tls::MAX_RECONNECT_BACKOFF_SECS);
        }
    }

    /// Run a single MQTT session: connect, then publish in a loop
    ///
    /// Returns `Err` when the connection is lost or any fatal error
    /// occurs.  The caller (`run`) handles reconnection.
    #[allow(unsafe_code)] // Calls tls_buffers::tls_buffers() and TopicName::new_unchecked()
    async fn run_session<RNG>(
        &self,
        stack: &Stack<'static>,
        rng: &mut RNG,
        mqtt_buffer: &mut [u8; MQTT_BUFFER_SIZE],
        tcp_rx_buffer: &mut [u8],
        tcp_tx_buffer: &mut [u8],
        publish_interval_secs: u64,
    ) -> Result<(), NetworkError>
    where
        RNG: rand_core::RngCore + rand_core::CryptoRng,
    {
        // --- DNS resolution ---
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

        // --- TCP connection ---
        let mut socket = AsyncTcpSocket::new(*stack, tcp_rx_buffer, tcp_tx_buffer);
        socket.connect(endpoint).await?;
        info!("TCP connected to {}", Debug2Format(&endpoint));

        // --- TLS 1.3 handshake ---
        // SAFETY: Single TLS connection at a time; buffers are used
        // exclusively for the duration of this session.
        let (tls_read, tls_write) = unsafe { tls_buffers::tls_buffers() };

        let tls_config = TlsConfig::new().with_server_name(self.config.broker_host);
        let mut tls_conn =
            TlsConnection::<AsyncTcpSocket, Aes128GcmSha256>::new(socket, tls_read, tls_write);

        info!("TLS 1.3 handshake...");
        let provider = SimpleCryptoProvider::new(rng);
        let tls_context = TlsContext::new(&tls_config, provider);

        tls_conn.open(tls_context).await.map_err(|e| {
            error!("TLS handshake failed: {:?}", Debug2Format(&e));
            TlsError::HandshakeFailed
        })?;
        info!("TLS 1.3 handshake OK");

        // --- MQTT CONNECT ---
        let client_id = device_id::mqtt_client_id();
        info!("MQTT client ID: {}", client_id);

        let mut buffer = BumpBuffer::new(mqtt_buffer);
        let mut mqtt = Client::<'_, _, _, 1, 1, 1, 0>::new(&mut buffer);

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

        let mqtt_client_id = MqttString::new(client_id.as_str().into()).map_err(|e| {
            error!("Invalid MQTT client ID: {:?}", Debug2Format(&e));
            MqttError::ProtocolError
        })?;

        mqtt.connect(tls_conn, &connect_opts, Some(mqtt_client_id))
            .await
            .map_err(|e| {
                error!("MQTT CONNECT failed: {:?}", Debug2Format(&e));
                MqttError::ConnectionFailed
            })?;

        info!("MQTT connected — entering publish loop");

        // --- Publish loop ---
        let mut msg_count = 0u32;

        loop {
            Timer::after(Duration::from_secs(publish_interval_secs)).await;
            msg_count += 1;

            let ts = time::get_timestamp();
            let topic_str = format_mqtt_topic(client_id.as_str(), "telemetry")?;
            let payload = format_json_payload(msg_count, &ts)?;

            info!(
                "Publishing #{} to '{}' ({} bytes)",
                msg_count,
                topic_str.as_str(),
                payload.len()
            );

            // SAFETY: format_mqtt_topic validates no wildcards or nulls
            let topic_name = unsafe {
                TopicName::new_unchecked(MqttString::new(topic_str.as_str().into()).map_err(
                    |e| {
                        error!("Topic string error: {:?}", Debug2Format(&e));
                        MqttError::ProtocolError
                    },
                )?)
            };

            let pub_opts = PublicationOptions {
                retain: false,
                message_expiry_interval: None,
                topic: TopicReference::Name(topic_name),
                // QoS 0 until event-driven handling (SR-SENS-004)
                qos: QoS::AtMostOnce,
            };

            match mqtt
                .publish(&pub_opts, Bytes::from(payload.as_bytes()))
                .await
            {
                Ok(packet_id) => {
                    info!("#{} published (packet_id: {})", msg_count, packet_id);
                }
                Err(e) => {
                    error!("Publish #{} failed: {:?}", msg_count, Debug2Format(&e));
                    return Err(MqttError::Disconnected.into());
                }
            }
        }
    }
}

/// Format an MQTT topic: `device/{client_id}/{subtopic}`
///
/// Validates that neither `client_id` nor `subtopic` contain MQTT
/// wildcard characters (`+`, `#`) or null bytes.
fn format_mqtt_topic(client_id: &str, subtopic: &str) -> Result<String<MAX_TOPIC_LEN>, MqttError> {
    if client_id.contains('+') || client_id.contains('#') || client_id.contains('\0') {
        error!("Client ID contains invalid MQTT topic characters");
        return Err(MqttError::ProtocolError);
    }
    if subtopic.contains('+') || subtopic.contains('#') || subtopic.contains('\0') {
        error!("Subtopic contains invalid MQTT topic characters");
        return Err(MqttError::ProtocolError);
    }

    let mut topic = String::<MAX_TOPIC_LEN>::new();
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

/// Format a JSON telemetry payload
///
/// Produces: `{"msg_id":N,"timestamp":SECS,"micros":MICROS}`
fn format_json_payload(msg_id: u32, ts: &time::Timestamp) -> Result<String<128>, MqttError> {
    use core::fmt::Write;

    let mut buf = String::<128>::new();
    write!(
        &mut buf,
        "{{\"msg_id\":{},\"timestamp\":{},\"micros\":{}}}",
        msg_id, ts.unix_secs, ts.micros
    )
    .map_err(|_| {
        error!("Failed to format payload JSON");
        MqttError::BufferError
    })?;

    Ok(buf)
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
        let topic = format_mqtt_topic("stm32f405-test123", "telemetry").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/telemetry");

        let topic = format_mqtt_topic("stm32f405-test123", "status").unwrap();
        assert_eq!(topic.as_str(), "device/stm32f405-test123/status");

        assert!(topic.len() < MAX_TOPIC_LEN);
    }

    #[test]
    fn test_format_mqtt_topic_buffer_overflow() {
        let long_id = "this_is_a_very_long_client_id_that_exceeds_the\
            _maximum_allowed_topic_length_for_mqtt_messages";
        let result = format_mqtt_topic(long_id, "telemetry");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_mqtt_topic_invalid_characters() {
        assert!(format_mqtt_topic("client+wildcard", "telemetry").is_err());
        assert!(format_mqtt_topic("client#wildcard", "telemetry").is_err());
        assert!(format_mqtt_topic("valid-client", "status+wildcard").is_err());
    }

    #[test]
    fn test_format_json_payload() {
        let ts = time::Timestamp::new(1_700_000_000, 123_456);
        let result = format_json_payload(42, &ts).unwrap();
        assert_eq!(
            result.as_str(),
            r#"{"msg_id":42,"timestamp":1700000000,"micros":123456}"#
        );
    }
}
