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
    /// MQTT connection failed
    MqttConnectionFailed,
    /// MQTT publish failed
    MqttPublishFailed,
    /// MQTT protocol error
    MqttProtocolError,
    /// MQTT buffer allocation failed
    MqttBufferError,
}

impl core::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DnsError => write!(f, "DNS resolution failed"),
            Self::SocketError => write!(f, "Socket error"),
            Self::Timeout => write!(f, "Request timeout"),
            Self::InvalidResponse => write!(f, "Invalid response"),
            Self::ServerError => write!(f, "Server error"),
            Self::AllServersFailed => write!(f, "All servers failed"),
            Self::RtcNotInitialized => write!(f, "RTC not initialized"),
            Self::RtcHardwareError => write!(f, "RTC hardware error"),
            Self::TlsHandshakeFailed => write!(f, "TLS handshake failed"),
            Self::TlsCertificateError => write!(f, "TLS certificate error"),
            Self::TlsAlertReceived => write!(f, "TLS alert received"),
            Self::TlsConnectionClosed => write!(f, "TLS connection closed"),
            Self::MqttConnectionFailed => write!(f, "MQTT connection failed"),
            Self::MqttPublishFailed => write!(f, "MQTT publish failed"),
            Self::MqttProtocolError => write!(f, "MQTT protocol error"),
            Self::MqttBufferError => write!(f, "MQTT buffer error"),
        }
    }
}

// Implement core::error::Error for no_std compatibility
impl core::error::Error for NetworkError {}

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
