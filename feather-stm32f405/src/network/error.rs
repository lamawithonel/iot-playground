#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client error types

use defmt::Format;

/// Network client operation errors
#[derive(Debug, Clone, Copy, Format)]
#[allow(dead_code)] // Some variants not used in Phase 1
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
    /// TLS handshake failed
    TlsHandshakeFailed,
    /// TLS certificate verification error
    TlsCertificateError,
    /// TLS alert received from peer
    TlsAlertReceived,
    /// TLS connection closed unexpectedly
    TlsConnectionClosed,
}

impl embedded_io_async::Error for NetworkError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Self::SocketError | Self::TlsConnectionClosed => {
                embedded_io_async::ErrorKind::BrokenPipe
            }
            Self::Timeout => embedded_io_async::ErrorKind::TimedOut,
            Self::InvalidResponse => embedded_io_async::ErrorKind::InvalidData,
            _ => embedded_io_async::ErrorKind::Other,
        }
    }
}
