#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client trait and base types
//!
//! A trait-based abstraction for network protocol clients.  Add a
//! new protocol by implementing `NetworkClient` without modifying
//! core infrastructure code (Open-Closed Principle).

use super::error::NetworkError;

/// Trait for network protocol clients
///
/// Implementors log and continue on error rather than panicking.
///
/// # Example Implementation
/// ```ignore
/// struct SntpClient { config: SntpConfig }
///
/// impl NetworkClient for SntpClient {
///     type Output = Timestamp;
///     async fn run(&mut self, stack: &Stack<'_>) -> Result<Self::Output, NetworkError> {
///         // Perform SNTP sync
///     }
/// }
/// ```
pub trait NetworkClient {
    /// Output type for successful client operation
    type Output;

    /// Run the client operation once
    ///
    /// Performs a single client operation (e.g., one SNTP sync
    /// request).  For periodic operations, the caller should invoke
    /// this method on a schedule.  The RNG supplies any per-request
    /// unpredictable values the protocol needs (e.g. the NTP
    /// transmit value matched against the reply).
    fn run<R: rand_core::RngCore>(
        &mut self,
        stack: &embassy_net::Stack<'static>,
        rng: &mut R,
    ) -> impl core::future::Future<Output = Result<Self::Output, NetworkError>>;
}
