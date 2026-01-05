//! SNTP client implementation
//!
//! Handles SNTP requests, DNS resolution, and server communication.

use defmt::{error, info, warn, Debug2Format, Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_time::{Duration, Instant, Timer};

use super::rtc::{RtcError, Timestamp};

/// Default NTP servers with fallback
const NTP_SERVERS: &[&str] = &["pool.ntp.org", "time.google.com", "time.cloudflare.com"];

/// SNTP port (UDP 123)
const SNTP_PORT: u16 = 123;

/// SNTP request timeout
const SNTP_TIMEOUT_MS: u64 = 5000;

/// Number of retry attempts per server
const SNTP_RETRY_COUNT: usize = 3;

/// Retry backoff delay
const SNTP_RETRY_BACKOFF_MS: u64 = 2000;

/// Re-synchronization interval (15 minutes)
const SNTP_RESYNC_INTERVAL_SECS: u64 = 900;

/// Maximum accepted stratum level
///
/// Stratum 0 = reference clock (GPS, atomic)
/// Stratum 1 = primary servers (directly connected to stratum 0)
/// Stratum 2 = secondary servers (synced to stratum 1)
/// Stratum 3 = tertiary servers (synced to stratum 2)
/// Stratum 16 = unsynchronized
const MAX_STRATUM: u8 = 3;

/// Maximum RTT correction in milliseconds
///
/// Clamps RTT/2 correction to a reasonable maximum to prevent overflow.
/// Network RTT exceeding 2 seconds (correction > 1000ms) is unusual and likely an error.
#[allow(dead_code)]
const MAX_RTT_CORRECTION_MS: u64 = 1000;

/// SNTP client errors
#[derive(Debug, Clone, Copy, Format)]
#[allow(dead_code)]
pub enum SntpError {
    /// Network communication error
    NetworkError,
    /// Request timeout
    Timeout,
    /// Invalid NTP response packet
    InvalidResponse,
    /// Server stratum too high or invalid
    InvalidStratum,
    /// All configured servers failed
    AllServersFailed,
    /// RTC not initialized
    RtcNotInitialized,
}

impl From<RtcError> for SntpError {
    fn from(e: RtcError) -> Self {
        match e {
            RtcError::NotSynced => SntpError::RtcNotInitialized,
            RtcError::HardwareError => SntpError::NetworkError,
        }
    }
}

/// Perform SNTP synchronization
///
/// Tries each configured NTP server with retry logic.
/// Returns the synchronized timestamp for the caller to write to RTC.
pub async fn sync_sntp(stack: &Stack<'static>) -> Result<Timestamp, SntpError> {
    info!("Starting SNTP synchronization");
    for server in NTP_SERVERS {
        for attempt in 0..SNTP_RETRY_COUNT {
            info!(
                "Attempting SNTP sync with {} (attempt {})",
                server,
                attempt + 1
            );
            match sntp_request(stack, server).await {
                Ok(timestamp) => {
                    info!(
                        "SNTP sync successful: {}.{:06} UTC",
                        timestamp.unix_secs, timestamp.micros
                    );
                    return Ok(timestamp);
                }
                Err(e) => {
                    warn!("SNTP sync failed: {:?}, retrying...", e);
                    Timer::after(Duration::from_millis(SNTP_RETRY_BACKOFF_MS)).await;
                }
            }
        }
    }
    error!("All SNTP sync attempts failed");
    Err(SntpError::AllServersFailed)
}

/// Send SNTP request and parse response
async fn sntp_request(stack: &Stack<'static>, server: &str) -> Result<Timestamp, SntpError> {
    // Resolve DNS hostname to IP
    let server_ip = match stack
        .dns_query(server, DnsQueryType::A)
        .await
        .map_err(|_| SntpError::NetworkError)?
        .first()
    {
        Some(ip) => *ip,
        None => return Err(SntpError::NetworkError),
    };
    let server_endpoint = IpEndpoint::new(server_ip, SNTP_PORT);
    info!("Resolved {} to {}", server, Debug2Format(&server_endpoint));

    // Create UDP socket with optimized buffers (NTP packets are 48 bytes)
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
    socket.bind(0).map_err(|_| SntpError::NetworkError)?;

    // Create NTP request packet (48 bytes, Mode 3 = Client)
    let mut ntp_packet = [0u8; 48];
    ntp_packet[0] = 0x1B; // LI=0, VN=3, Mode=3

    // Record transmit time for RTT calculation
    let transmit_time = Instant::now();

    // Send NTP request
    socket
        .send_to(&ntp_packet, server_endpoint)
        .await
        .map_err(|_| SntpError::NetworkError)?;
    info!("Sent NTP request to {}", Debug2Format(&server_endpoint));

    // Receive response with timeout
    let mut response = [0u8; 48];
    let timeout_future = Timer::after(Duration::from_millis(SNTP_TIMEOUT_MS));
    let recv_future = socket.recv_from(&mut response);
    let (recv_len, from_addr) =
        match embassy_futures::select::select(timeout_future, recv_future).await {
            embassy_futures::select::Either::First(_) => return Err(SntpError::Timeout),
            embassy_futures::select::Either::Second(result) => {
                result.map_err(|_| SntpError::NetworkError)?
            }
        };

    // Record receive time for RTT calculation
    let receive_time = Instant::now();

    info!(
        "Received {} bytes from {}",
        recv_len,
        Debug2Format(&from_addr)
    );

    // Validate response length and source
    if recv_len < 48 || from_addr.endpoint.addr != server_ip {
        return Err(SntpError::InvalidResponse);
    }

    // Validate stratum (byte 1)
    let stratum = response[1];
    info!("NTP server stratum: {}", stratum);

    if stratum == 0 || stratum > MAX_STRATUM {
        warn!("Invalid stratum {} (max {})", stratum, MAX_STRATUM);
        return Err(SntpError::InvalidStratum);
    }

    // Extract transmit timestamp (bytes 40-47)
    let tx_timestamp_secs =
        u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
    let tx_timestamp_frac =
        u32::from_be_bytes([response[44], response[45], response[46], response[47]]);

    // Calculate round-trip time and apply RTT/2 correction
    let rtt = receive_time.duration_since(transmit_time);
    let rtt_correction_micros = rtt.as_micros() / 2;

    let mut timestamp = Timestamp::from_ntp(tx_timestamp_secs, tx_timestamp_frac);

    // Apply RTT/2 correction with bounds checking
    // Clamp to reasonable maximum (1 second = 1_000_000 microseconds)
    let correction = rtt_correction_micros.min(1_000_000) as u32;
    timestamp.micros = timestamp.micros.saturating_add(correction);
    if timestamp.micros >= 1_000_000 {
        timestamp.unix_secs = timestamp.unix_secs.saturating_add(1);
        timestamp.micros -= 1_000_000;
    }

    info!(
        "NTP timestamp: {}.{:06} UTC (RTT correction: {} µs)",
        timestamp.unix_secs, timestamp.micros, correction
    );
    Ok(timestamp)
}

/// Background task for periodic re-synchronization (15 minutes)
///
/// Returns timestamps for the caller to write to RTC.
#[allow(dead_code)]
pub async fn resync_task(stack: &Stack<'static>) -> ! {
    loop {
        Timer::after(Duration::from_secs(SNTP_RESYNC_INTERVAL_SECS)).await;
        match sync_sntp(stack).await {
            Ok(_timestamp) => {
                // Caller should write timestamp to RTC
            }
            Err(e) => {
                error!("Periodic SNTP resync failed: {:?}", e);
            }
        }
    }
}

/// Initialize time system with SNTP sync
///
/// Call once after DHCP lease is acquired.
/// Returns timestamp for caller to write to RTC.
pub async fn initialize_time(stack: &Stack<'static>) -> Result<Timestamp, SntpError> {
    sync_sntp(stack).await
}

/// Start periodic SNTP re-synchronization task
///
/// Should be spawned as a separate task. Resyncs every 15 minutes.
/// Returns timestamps for caller to write to RTC.
#[allow(dead_code)]
pub async fn start_resync_task(stack: &Stack<'static>) -> ! {
    resync_task(stack).await
}
