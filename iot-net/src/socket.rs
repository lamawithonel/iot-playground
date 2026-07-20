#![deny(unsafe_code)]
#![deny(warnings)]
//! Async TCP socket wrapper for embedded-tls integration
//!
//! An async wrapper around `embassy_net::tcp::TcpSocket` that
//! implements the `embedded-io-async` traits required by `embedded-tls`.

use embassy_net::tcp::TcpSocket;
use embassy_net::{IpEndpoint, Stack};
use embedded_io_async::{ErrorType, Read, Write};

use super::error::NetworkError;

/// Async TCP socket wrapper implementing embedded-io-async traits
///
/// This wrapper provides the `Read` and `Write` traits from `embedded-io-async`
/// that `embedded-tls` requires for TLS operations.
///
/// # Example
///
/// ```ignore
/// // ignore: needs a live embassy-net `Stack` to bind; this crate
/// // has no host doc-tests (embassy-net does not build for host).
/// let mut socket = AsyncTcpSocket::new(stack, rx_buffer, tx_buffer);
/// socket.connect(endpoint).await?;
/// // Now socket can be used with embedded-tls
/// ```
pub struct AsyncTcpSocket<'a> {
    socket: TcpSocket<'a>,
}

impl<'a> AsyncTcpSocket<'a> {
    /// Create a new async TCP socket
    ///
    /// # Arguments
    ///
    /// * `stack` - Embassy network stack
    /// * `rx_buffer` - Buffer for receiving data (typically 4-8 KB)
    /// * `tx_buffer` - Buffer for transmitting data (typically 4-8 KB)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // ignore: needs a live embassy-net `Stack` to bind; this crate
    /// // has no host doc-tests (embassy-net does not build for host).
    /// let mut rx_buffer = [0u8; 4096];
    /// let mut tx_buffer = [0u8; 4096];
    /// let socket = AsyncTcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    /// ```
    pub fn new(stack: Stack<'a>, rx_buffer: &'a mut [u8], tx_buffer: &'a mut [u8]) -> Self {
        Self {
            socket: TcpSocket::new(stack, rx_buffer, tx_buffer),
        }
    }

    /// Connect to a remote endpoint
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Remote IP endpoint to connect to
    ///
    /// # Errors
    ///
    /// Returns `NetworkError::SocketError` if connection fails
    pub async fn connect(&mut self, endpoint: IpEndpoint) -> Result<(), NetworkError> {
        self.socket
            .connect(endpoint)
            .await
            .map_err(|_| NetworkError::SocketError)
    }
}

/// Error type for embedded-io-async traits
///
/// Uses `NetworkError` as the error type, for consistency with the
/// rest of the crate.
impl ErrorType for AsyncTcpSocket<'_> {
    type Error = NetworkError;
}

/// Async read implementation for embedded-tls
///
/// This allows `embedded-tls` to read data from the TCP socket.
impl Read for AsyncTcpSocket<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.socket
            .read(buf)
            .await
            .map_err(|_| NetworkError::SocketError)
    }
}

/// Async write implementation for embedded-tls
///
/// This allows `embedded-tls` to write data to the TCP socket.
impl Write for AsyncTcpSocket<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.socket
            .write(buf)
            .await
            .map_err(|_| NetworkError::SocketError)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.socket
            .flush()
            .await
            .map_err(|_| NetworkError::SocketError)
    }
}
