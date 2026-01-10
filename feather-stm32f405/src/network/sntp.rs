#![deny(unsafe_code)]
#![deny(warnings)]
//! SNTP client implementing NetworkClient trait

use defmt::{error, info, warn, Debug2Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_time::{Duration, Instant, Timer};
use rtic_monotonics::fugit::ExtU64;
use rtic_monotonics::Monotonic;

use crate::ccmram;
use crate::time::{write_rtc, RtcError, Timestamp};
use crate::Mono;

use super::client::NetworkClient;
use super::config::SntpConfig;
use super::error::NetworkError;

impl From<RtcError> for NetworkError {
    fn from(e: RtcError) -> Self {
        match e {
            RtcError::NotInitialized => NetworkError::RtcNotInitialized,
            RtcError::HardwareError => NetworkError::RtcHardwareError,
        }
    }
}

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
    async fn sync(&self, stack: &Stack<'static>) -> Result<Timestamp, NetworkError> {
        info!("Starting SNTP synchronization");
        for server in self.config.servers {
            for attempt in 0..self.config.retry_count {
                info!(
                    "Attempting SNTP sync with {} (attempt {})",
                    server,
                    attempt + 1
                );
                match self.sntp_request(stack, server).await {
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

    async fn sntp_request(
        &self,
        stack: &Stack<'static>,
        server: &str,
    ) -> Result<Timestamp, NetworkError> {
        let server_ip = stack
            .dns_query(server, DnsQueryType::A)
            .await
            .map_err(|_| NetworkError::DnsError)?
            .first()
            .copied()
            .ok_or(NetworkError::DnsError)?;

        let server_endpoint = IpEndpoint::new(server_ip, 123);
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

        // NTP request: LI=0, VN=3, Mode=3 (Client)
        let mut ntp_packet = [0u8; 48];
        ntp_packet[0] = 0x1B;
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

        if recv_len < 48 || from_addr.endpoint.addr != server_ip {
            return Err(NetworkError::InvalidResponse);
        }

        let stratum = response[1];
        info!("NTP server stratum: {}", stratum);

        if stratum == 0 || stratum > self.config.max_stratum {
            warn!(
                "Invalid stratum {} (max {})",
                stratum, self.config.max_stratum
            );
            return Err(NetworkError::ServerError);
        }

        let tx_timestamp_secs =
            u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
        let tx_timestamp_frac =
            u32::from_be_bytes([response[44], response[45], response[46], response[47]]);

        let rtt = receive_time.duration_since(transmit_time);
        let rtt_correction_micros = rtt.as_micros() / 2;

        let mut timestamp = Timestamp::from_ntp(tx_timestamp_secs, tx_timestamp_frac);
        timestamp.micros = timestamp
            .micros
            .saturating_add(rtt_correction_micros as u32);
        if timestamp.micros >= 1_000_000 {
            timestamp.unix_secs = timestamp.unix_secs.saturating_add(1);
            timestamp.micros -= 1_000_000;
        }

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

    async fn run(&mut self, stack: &Stack<'static>) -> Result<Self::Output, NetworkError> {
        self.sync(stack).await
    }
}
