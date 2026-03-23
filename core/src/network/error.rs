//! Network client error types
//!
//! Provides a flexible error hierarchy for network components.
//! Component-specific errors (`MqttError`, `TlsError`, `SntpError`)
//! convert automatically into the unified `NetworkError` via `From`.

/// Network client operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub enum NetworkError {
    /// DNS resolution failed
    DnsError,
    /// Socket bind/connect error
    SocketError,
    /// Request timeout
    Timeout,
    /// Invalid response from server
    InvalidResponse,
    /// Server error (generic)
    ServerError,
    /// All configured servers failed
    AllServersFailed,
    /// RTC not initialized
    RtcNotInitialized,
    /// RTC hardware error
    RtcHardwareError,
    /// TLS-specific error (see [`TlsError`] for details)
    Tls(TlsError),
    /// MQTT-specific error (see [`MqttError`] for details)
    Mqtt(MqttError),
    /// SNTP-specific error (see [`SntpError`] for details)
    Sntp(SntpError),
}

/// TLS operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub enum TlsError {
    /// TLS handshake failed
    HandshakeFailed,
    /// Certificate verification error
    CertificateError,
    /// TLS alert received from peer
    AlertReceived,
    /// Connection closed unexpectedly
    ConnectionClosed,
}

/// MQTT operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub enum MqttError {
    /// MQTT connection failed
    ConnectionFailed,
    /// MQTT publish failed
    PublishFailed,
    /// Broker disconnected (connection was established then lost)
    Disconnected,
    /// MQTT protocol error
    ProtocolError,
    /// Buffer allocation failed
    BufferError,
}

/// SNTP operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(dead_code)]
pub enum SntpError {
    /// Invalid stratum received
    InvalidStratum,
    /// Parse error
    ParseError,
}

impl From<TlsError> for NetworkError {
    fn from(err: TlsError) -> Self {
        NetworkError::Tls(err)
    }
}

impl From<MqttError> for NetworkError {
    fn from(err: MqttError) -> Self {
        NetworkError::Mqtt(err)
    }
}

impl From<SntpError> for NetworkError {
    fn from(err: SntpError) -> Self {
        NetworkError::Sntp(err)
    }
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
            Self::Tls(e) => write!(f, "TLS error: {}", e),
            Self::Mqtt(e) => write!(f, "MQTT error: {}", e),
            Self::Sntp(e) => write!(f, "SNTP error: {}", e),
        }
    }
}

impl core::fmt::Display for TlsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HandshakeFailed => write!(f, "handshake failed"),
            Self::CertificateError => write!(f, "certificate error"),
            Self::AlertReceived => write!(f, "alert received"),
            Self::ConnectionClosed => write!(f, "connection closed"),
        }
    }
}

impl core::fmt::Display for MqttError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "connection failed"),
            Self::PublishFailed => write!(f, "publish failed"),
            Self::Disconnected => write!(f, "broker disconnected"),
            Self::ProtocolError => write!(f, "protocol error"),
            Self::BufferError => write!(f, "buffer error"),
        }
    }
}

impl core::fmt::Display for SntpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidStratum => write!(f, "invalid stratum"),
            Self::ParseError => write!(f, "parse error"),
        }
    }
}

impl core::error::Error for NetworkError {}
impl core::error::Error for TlsError {}
impl core::error::Error for MqttError {}
impl core::error::Error for SntpError {}

impl From<crate::time::RtcError> for NetworkError {
    fn from(e: crate::time::RtcError) -> Self {
        match e {
            crate::time::RtcError::NotInitialized => NetworkError::RtcNotInitialized,
            crate::time::RtcError::HardwareError => NetworkError::RtcHardwareError,
        }
    }
}

#[cfg(feature = "embedded-io")]
impl embedded_io_async::Error for NetworkError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Self::SocketError | Self::Tls(TlsError::ConnectionClosed) => {
                embedded_io_async::ErrorKind::BrokenPipe
            }
            Self::Timeout => embedded_io_async::ErrorKind::TimedOut,
            Self::InvalidResponse => embedded_io_async::ErrorKind::InvalidData,
            _ => embedded_io_async::ErrorKind::Other,
        }
    }
}
