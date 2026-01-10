#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client error types

use defmt::Format;

/// Network client operation errors
#[derive(Debug, Clone, Copy, Format)]
pub enum NetworkError {
    /// DNS resolution failed
    DnsError,
    /// Socket bind/connect error
    SocketError,
    /// Request timeout
    Timeout,
    /// Invalid response from server
    InvalidResponse,
    /// Server error (e.g., invalid stratum for NTP)
    ServerError,
    /// All configured servers failed
    AllServersFailed,
    /// RTC not initialized
    RtcNotInitialized,
    /// RTC hardware error
    RtcHardwareError,
}
