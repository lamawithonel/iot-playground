#![deny(unsafe_code)]
#![deny(warnings)]
//! SNTP client implementing NetworkClient trait

use defmt::{error, info, warn, Debug2Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_time::{Duration, Instant, Timer};
use rand_core::RngCore;
use rtic_monotonics::fugit::ExtU64;
use rtic_monotonics::Monotonic;

use crate::ccmram;
use crate::time::{write_rtc, Timestamp};
use crate::Mono;

use super::client::NetworkClient;
use super::config::SntpConfig;
use super::error::NetworkError;

/// SNTP/NTP port (UDP 123)
const SNTP_PORT: u16 = 123;

/// SNTP client for time synchronization
pub struct SntpClient {
    config: SntpConfig,
}

impl SntpClient {
    /// Create a new SNTP client with default configuration
    pub fn new() -> Self {
        Self {
            config: SntpConfig::default(),
        }
    }

    /// Create a new SNTP client with custom configuration
    #[allow(dead_code)]
    pub fn with_config(config: SntpConfig) -> Self {
        Self { config }
    }

    /// Perform SNTP synchronization with internal RTC update
    async fn sync<R: RngCore>(
        &self,
        stack: &Stack<'static>,
        rng: &mut R,
    ) -> Result<Timestamp, NetworkError> {
        info!("Starting SNTP synchronization");
        for server in self.config.servers {
            for attempt in 0..self.config.retry_count {
                info!(
                    "Attempting SNTP sync with {} (attempt {})",
                    server,
                    attempt + 1
                );
                match self.sntp_request(stack, server, rng).await {
                    Ok(timestamp) => {
                        info!(
                            "SNTP sync successful: {}.{:06} UTC",
                            timestamp.unix_secs, timestamp.micros
                        );
                        write_rtc(timestamp)?;
                        self.calibrate_wallclock(timestamp);
                        return Ok(timestamp);
                    }
                    Err(e) => {
                        warn!("SNTP sync failed: {:?}, retrying...", e);
                        Mono::delay(2000_u64.millis()).await;
                    }
                }
            }
        }
        error!("All SNTP sync attempts failed");
        Err(NetworkError::AllServersFailed)
    }

    fn calibrate_wallclock(&self, timestamp: Timestamp) {
        let mono_micros = Mono::now().ticks() as u32;
        ccmram::calibrate_wallclock(timestamp.unix_secs as u32, timestamp.micros, mono_micros);
        info!(
            "Wall-clock calibrated: RTC updated, mono={} µs",
            mono_micros
        );
    }

    async fn sntp_request<R: RngCore>(
        &self,
        stack: &Stack<'static>,
        server: &str,
        rng: &mut R,
    ) -> Result<Timestamp, NetworkError> {
        let server_ip = stack
            .dns_query(server, DnsQueryType::A)
            .await
            .map_err(|_| NetworkError::DnsError)?
            .first()
            .copied()
            .ok_or(NetworkError::DnsError)?;

        let server_endpoint = IpEndpoint::new(server_ip, SNTP_PORT);
        info!("Resolved {} to {}", server, Debug2Format(&server_endpoint));

        let mut rx_meta = [PacketMetadata::EMPTY; 2];
        let mut rx_buffer = [0u8; 64];
        let mut tx_meta = [PacketMetadata::EMPTY; 2];
        let mut tx_buffer = [0u8; 64];
        let mut socket = UdpSocket::new(
            *stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );
        socket.bind(0).map_err(|_| NetworkError::SocketError)?;

        // Build the request with an unpredictable transmit value the
        // server echoes back, so the reply can be matched to it.
        let transmit = rng.next_u64();
        let ntp_packet = iot_core::network::sntp::build_request(transmit);
        let transmit_time = Instant::now();
        socket
            .send_to(&ntp_packet, server_endpoint)
            .await
            .map_err(|_| NetworkError::SocketError)?;
        info!("Sent NTP request to {}", Debug2Format(&server_endpoint));

        let mut response = [0u8; 48];
        let timeout_future = Timer::after(Duration::from_millis(self.config.timeout_ms));
        let recv_future = socket.recv_from(&mut response);
        let (recv_len, from_addr) =
            match embassy_futures::select::select(timeout_future, recv_future).await {
                embassy_futures::select::Either::First(_) => return Err(NetworkError::Timeout),
                embassy_futures::select::Either::Second(result) => {
                    result.map_err(|_| NetworkError::SocketError)?
                }
            };
        let receive_time = Instant::now();

        info!(
            "Received {} bytes from {}",
            recv_len,
            Debug2Format(&from_addr)
        );

        // Validate the reply's fixed fields (source, mode, leap
        // indicator, stratum, and the echoed transmit value) before
        // trusting its time.  A reply that fails any check carries an
        // indeterminate value and is discarded rather than written to
        // the clock.
        let source_matches = from_addr.endpoint.addr == server_ip;
        let (ntp_secs, ntp_frac) = iot_core::network::sntp::validate_reply(
            &response,
            recv_len,
            source_matches,
            transmit,
            self.config.max_stratum,
        )
        .map_err(|e| {
            warn!("Rejected NTP reply: {:?}", e);
            NetworkError::InvalidResponse
        })?;

        let rtt = receive_time.duration_since(transmit_time);
        let rtt_correction_micros = rtt.as_micros() / 2;

        // `Timestamp::new` carries any microsecond overflow into
        // seconds, so the RTT correction cannot leave micros >= 1e6.
        let base = Timestamp::from_ntp(ntp_secs, ntp_frac);
        let timestamp = Timestamp::new(
            base.unix_secs,
            base.micros.saturating_add(rtt_correction_micros as u32),
        );

        info!(
            "NTP timestamp: {}.{:06} UTC (RTT correction: {} µs)",
            timestamp.unix_secs, timestamp.micros, rtt_correction_micros
        );
        Ok(timestamp)
    }
}

impl Default for SntpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkClient for SntpClient {
    type Output = Timestamp;

    async fn run<R: RngCore>(
        &mut self,
        stack: &Stack<'static>,
        rng: &mut R,
    ) -> Result<Self::Output, NetworkError> {
        self.sync(stack, rng).await
    }
}
