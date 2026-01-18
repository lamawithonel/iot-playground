#![deny(unsafe_code)]
#![deny(warnings)]
//! Network client trait and base types
//!
//! This module provides a trait-based abstraction for network protocol clients.
//! New protocols can be added by implementing `NetworkClient` without modifying
//! core infrastructure code (Open-Closed Principle).

use super::error::NetworkError;

/// Trait for network protocol clients
///
/// Implementors handle their own errors gracefully (log and continue)
/// rather than panicking, enabling robust operation in embedded systems.
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
    /// This is an async method that performs a single client operation
    /// (e.g., one SNTP sync request). For periodic operations, the caller
    /// should invoke this method on a schedule.
    fn run(
        &mut self,
        stack: &embassy_net::Stack<'static>,
    ) -> impl core::future::Future<Output = Result<Self::Output, NetworkError>>;
}
