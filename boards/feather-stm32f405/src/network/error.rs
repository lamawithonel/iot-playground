#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client error types
//!
//! This module provides a flexible error hierarchy that allows network components
//! to define their own error types while maintaining a unified error interface.
//!
//! # Architecture
//!
//! - Base `NetworkError` enum provides common network errors
//! - Component-specific errors (MQTT, TLS, SNTP) are separate types
//! - `From` implementations allow automatic error conversion
//! - This decouples the base network module from specific components

use defmt::Format;

/// Network client operation errors
///
/// This enum contains common network errors that apply across multiple components,
/// plus generic variants for component-specific errors.
#[derive(Debug, Clone, Copy, Format)]
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
    /// TLS-specific error (see TlsError for details)
    Tls(TlsError),
    /// MQTT-specific error (see MqttError for details)
    Mqtt(MqttError),
    /// SNTP-specific error (see SntpError for details)
    Sntp(SntpError),
}

/// TLS operation errors
#[derive(Debug, Clone, Copy, Format)]
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
#[derive(Debug, Clone, Copy, Format)]
#[allow(dead_code)]
pub enum MqttError {
    /// MQTT connection failed
    ConnectionFailed,
    /// MQTT publish failed
    PublishFailed,
    /// MQTT protocol error
    ProtocolError,
    /// Buffer allocation failed
    BufferError,
}

/// SNTP operation errors
#[derive(Debug, Clone, Copy, Format)]
#[allow(dead_code)]
pub enum SntpError {
    /// Invalid stratum received
    InvalidStratum,
    /// Parse error
    ParseError,
}

// Automatic conversion from component errors to NetworkError
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

// Implement core::error::Error for no_std compatibility
impl core::error::Error for NetworkError {}
impl core::error::Error for TlsError {}
impl core::error::Error for MqttError {}
impl core::error::Error for SntpError {}

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
