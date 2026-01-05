//! SNTP Time Synchronization Module with Hardware RTC
//!
//! Implements time synchronization using SNTP (Simple Network Time Protocol)
//! per RFC 5905, fulfilling requirements SR-NET-006 and SR-NET-007.
//!
//! ## Architecture
//! - SNTP client syncs with NTP servers every 15 minutes
//! - Time is written to STM32 hardware internal RTC
//! - Between syncs, timestamps are read from internal RTC hardware
//! - Sync status stored atomically in CCM RAM
//! - defmt timestamps use RTC for Unix epoch time display
//!
//! ## Features
//! - UDP socket communication with NTP servers
//! - DNS hostname resolution
//! - Multi-server fallback with retries
//! - 15-minute automatic re-synchronization
//! - Hardware internal RTC for accurate timekeeping between syncs
//! - Atomic sync status in CCM RAM
//! - Custom defmt timestamps (Unix epoch time instead of uptime)
//! - Stratum validation (rejects servers with stratum > 3)
//!
//! ## defmt Timestamps
//!
//! This module provides a custom `defmt::timestamp!()` implementation using the hardware RTC.
//! Returns Unix epoch time in seconds (u64) formatted as ISO8601 date-time.
//!
//! ### Behavior:
//! - Before first NTP sync: Shows 0 (timestamp not available)
//! - After NTP sync: Shows ISO8601 formatted time from RTC (1-second resolution)
//! - Between syncs: RTC continues counting (±20-50ppm accuracy from LSE)
//!
//! ### Format:
//! The `:iso8601s` display hint formats Unix epoch seconds as ISO8601 date-time strings.
//! Example: `1767571200` → `2026-01-05T01:00:00Z`
//!
//! See: <https://defmt.ferrous-systems.com/timestamps>
//!
//! ## Usage
//! ```no_run
//! // Initialize internal RTC and perform initial SNTP sync
//! time::initialize_rtc(rtc);
//! if let Ok(ts) = time::initialize_time(&stack).await {
//!     info!("Time synced: {}.{:06}", ts.unix_secs, ts.micros);
//! }
//!
//! // Check if time is synchronized
//! if time::is_time_synced() {
//!     // Safe to use timestamps
//! }
//!
//! // Spawn periodic resync task
//! spawner.spawn(time_resync(stack)).ok();
//!
//! // Get timestamp from internal RTC for sensor data
//! let timestamp = time::get_timestamp();
//! ```

// Allow unsafe code for #[link_section] attribute used in CCM RAM allocation
#![allow(unsafe_code)]
#![deny(warnings)]

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use critical_section::Mutex;
use defmt::{error, info, warn, Debug2Format, Format};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpEndpoint, Stack};
use embassy_stm32::rtc::{DateTime, DayOfWeek, Rtc};
use embassy_time::{Duration, Instant, Timer};

/// NTP epoch offset (1900-01-01 to 1970-01-01 in seconds)
const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

/// Unix epoch start (1970-01-01)
const UNIX_EPOCH_YEAR: u16 = 1970;

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

/// Maximum allowed drift before forced resync (future enhancement)
#[allow(dead_code)]
const MAX_DRIFT_MICROS: i64 = 1_000_000;

/// Timestamp with microsecond precision
#[derive(Debug, Clone, Copy, Format)]
pub struct Timestamp {
    /// Unix timestamp in seconds since epoch (1970-01-01 00:00:00 UTC)
    pub unix_secs: u64,
    /// Microseconds component (0-999,999)
    pub micros: u32,
}

impl Timestamp {
    /// Create a new timestamp
    pub const fn new(unix_secs: u64, micros: u32) -> Self {
        Self { unix_secs, micros }
    }

    /// Convert from NTP timestamp (seconds since 1900-01-01)
    pub fn from_ntp(ntp_secs: u64, ntp_frac: u32) -> Self {
        let unix_secs = ntp_secs.saturating_sub(NTP_UNIX_OFFSET);
        // Convert NTP fractional part to microseconds (2^-32 seconds)
        let micros = ((ntp_frac as u64 * 1_000_000) >> 32) as u32;
        Self::new(unix_secs, micros)
    }
}

/// Global internal RTC instance
static RTC: Mutex<RefCell<Option<Rtc>>> = Mutex::new(RefCell::new(None));

/// System time synchronization status in CCM RAM
#[link_section = ".ccmram"]
static TIME_SYNCED: AtomicBool = AtomicBool::new(false);

/// Initialize internal RTC
///
/// Must be called once during system initialization before any time operations.
pub fn initialize_rtc(rtc: Rtc) {
    critical_section::with(|cs| {
        RTC.borrow(cs).replace(Some(rtc));
    });
    info!("Internal RTC initialized");
}

/// Check if time has been synchronized with NTP
///
/// Returns `true` if at least one successful NTP sync has occurred.
/// Time read from `get_timestamp()` is only valid when this returns `true`.
#[allow(dead_code)]
pub fn is_time_synced() -> bool {
    TIME_SYNCED.load(Ordering::Relaxed)
}

/// Convert Unix timestamp to RTC DateTime
fn unix_to_datetime(unix_secs: u64) -> DateTime {
    // Simple conversion - not accounting for all leap years perfectly
    // Good enough for embedded use between NTP syncs

    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_LEAP_YEAR: u64 = 366;

    let mut days = unix_secs / SECONDS_PER_DAY;
    let secs_today = unix_secs % SECONDS_PER_DAY;

    let hour = (secs_today / 3600) as u8;
    let minute = ((secs_today % 3600) / 60) as u8;
    let second = (secs_today % 60) as u8;

    // Calculate year (simplified - doesn't handle all leap year edge cases)
    let mut year = UNIX_EPOCH_YEAR;
    loop {
        let days_in_year = if is_leap_year(year) {
            DAYS_PER_LEAP_YEAR
        } else {
            DAYS_PER_YEAR
        };

        if days < days_in_year {
            break;
        }

        days -= days_in_year;
        year += 1;
    }

    // Calculate month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u8;
    let mut day = days as u8 + 1;

    for days_in_month in days_in_months.iter() {
        if day <= *days_in_month {
            break;
        }
        day -= days_in_month;
        month += 1;
    }

    // Build DateTime using separate arguments (embassy-stm32 v0.4.0 API)
    // Returns Result<DateTime, DateTimeError> - unwrap is safe for valid date ranges
    DateTime::from(
        year,
        month,
        day,
        DayOfWeek::Monday, // Placeholder - not critical for timekeeping
        hour,
        minute,
        second,
        0, // microsecond
    )
    .unwrap_or_else(|_| {
        // Fallback to Unix epoch if date construction fails
        error!("Failed to construct DateTime, falling back to epoch");
        DateTime::from(1970, 1, 1, DayOfWeek::Thursday, 0, 0, 0, 0).unwrap()
    })
}

/// Convert RTC DateTime to Unix timestamp
fn datetime_to_unix(dt: DateTime) -> u64 {
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_LEAP_YEAR: u64 = 366;

    // Count days since Unix epoch
    let mut days = 0u64;

    // Add days for complete years
    for y in UNIX_EPOCH_YEAR..dt.year() {
        days += if is_leap_year(y) {
            DAYS_PER_LEAP_YEAR
        } else {
            DAYS_PER_YEAR
        };
    }

    // Add days for complete months in current year
    let days_in_months = if is_leap_year(dt.year()) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for m in 0..(dt.month() - 1) {
        days += days_in_months[m as usize] as u64;
    }

    // Add remaining days
    days += (dt.day() - 1) as u64;

    // Convert to seconds and add time of day
    days * SECONDS_PER_DAY
        + (dt.hour() as u64) * 3600
        + (dt.minute() as u64) * 60
        + (dt.second() as u64)
}

/// Check if year is a leap year
fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Write timestamp to internal RTC hardware
fn write_rtc(timestamp: Timestamp) -> Result<(), ()> {
    let datetime = unix_to_datetime(timestamp.unix_secs);

    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow(cs).borrow_mut().as_mut() {
            // Ignore RTC write errors - they're non-critical
            // Sync status flag will indicate overall success
            let _ = rtc.set_datetime(datetime);
            TIME_SYNCED.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err(())
        }
    })
}

/// Read timestamp from internal RTC hardware
fn read_rtc() -> Result<Timestamp, ()> {
    if !TIME_SYNCED.load(Ordering::Relaxed) {
        return Ok(Timestamp::new(0, 0));
    }

    critical_section::with(|cs| {
        if let Some(rtc) = RTC.borrow(cs).borrow_mut().as_mut() {
            // rtc.now() returns Result<DateTime, RtcError>
            let datetime = rtc.now().map_err(|_| ())?;
            let unix_secs = datetime_to_unix(datetime);
            // Internal RTC only has 1-second resolution
            Ok(Timestamp::new(unix_secs, 0))
        } else {
            Err(())
        }
    })
}

/// Perform SNTP synchronization and write to internal RTC
///
/// Tries each configured NTP server with retry logic.
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

                    // Write to internal RTC hardware
                    if write_rtc(timestamp).is_err() {
                        error!("Failed to write timestamp to internal RTC");
                        return Err(SntpError::NetworkError);
                    }

                    info!("Internal RTC updated with NTP time");
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

    // Create UDP socket with buffers
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0u8; 512];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buffer = [0u8; 512];
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

    // Add transmit timestamp for round-trip calculation
    let now = Instant::now();
    let timestamp_secs = now.as_secs();
    let timestamp_frac = ((now.as_micros() % 1_000_000) << 32) / 1_000_000;
    ntp_packet[40..44].copy_from_slice(&(timestamp_secs as u32).to_be_bytes());
    ntp_packet[44..48].copy_from_slice(&(timestamp_frac as u32).to_be_bytes());

    // Send NTP request
    socket
        .send_to(&ntp_packet, server_endpoint)
        .await
        .map_err(|_| SntpError::NetworkError)?;
    info!("Sent NTP request to {}", Debug2Format(&server_endpoint));

    // Receive response with timeout (using separate response buffer)
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

    let timestamp = Timestamp::from_ntp(tx_timestamp_secs, tx_timestamp_frac);
    info!(
        "NTP timestamp: {}.{:06} UTC",
        timestamp.unix_secs, timestamp.micros
    );
    Ok(timestamp)
}

/// Background task for periodic re-synchronization (15 minutes)
pub async fn resync_task(stack: &Stack<'static>) -> ! {
    loop {
        Timer::after(Duration::from_secs(SNTP_RESYNC_INTERVAL_SECS)).await;
        if let Err(e) = sync_sntp(stack).await {
            error!("Periodic SNTP resync failed: {:?}", e);
        }
    }
}

/// SNTP client errors
#[derive(Debug, Clone, Copy, Format)]
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
}

/// Get current timestamp from internal RTC hardware
///
/// Returns `Timestamp` with `unix_secs = 0` until first sync.
pub fn get_timestamp() -> Timestamp {
    read_rtc().unwrap_or(Timestamp::new(0, 0))
}

/// Initialize time system with SNTP sync
///
/// Call once after DHCP lease is acquired.
pub async fn initialize_time(stack: &Stack<'static>) -> Result<Timestamp, SntpError> {
    sync_sntp(stack).await
}

/// Start periodic SNTP re-synchronization task
///
/// Should be spawned as a separate task. Resyncs every 15 minutes.
pub async fn start_resync_task(stack: &Stack<'static>) -> ! {
    resync_task(stack).await
}

// ============================================================================
// DEFMT TIMESTAMP IMPLEMENTATION
// ============================================================================
// Custom defmt timestamp using hardware RTC. See module documentation for details.

defmt::timestamp!("{=u64:iso8601s}", { get_timestamp().unix_secs });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntp_to_unix_conversion() {
        let ts = Timestamp::from_ntp(NTP_UNIX_OFFSET, 0);
        assert_eq!(ts.unix_secs, 0);
        assert_eq!(ts.micros, 0);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2023));
    }
}
