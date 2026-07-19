#![deny(unsafe_code)]
#![deny(warnings)]
//! Wait-counting wrapper for `ExtiInput`.
//!
//! Wraps an [`ExtiInput`] and increments an [`AtomicU32`] counter
//! each time a [`Wait`] method completes.  This counts completed
//! waits, not raw hardware interrupts; however, embassy-net-wiznet
//! only calls `wait_for_low()` when the W5500 INT line is
//! deasserted, so each completion corresponds to one interrupt
//! serviced.  The counter provides firmware-level telemetry proving
//! that packet reception is interrupt-driven (EXTI2), not
//! polling-based.

use core::convert::Infallible;
use core::sync::atomic::{AtomicU32, Ordering};

use embassy_stm32::exti::ExtiInput;
use embassy_stm32::mode::Async;
use embedded_hal::digital::ErrorType;
use embedded_hal_async::digital::Wait;

/// An [`ExtiInput`] that counts completed waits.
///
/// Every completed `wait_for_*` call increments the associated
/// atomic counter.  The counter is typically read and reset per
/// MQTT publish cycle to produce per-interval telemetry.
///
/// The counter reference is `'static` because RTIC tasks require
/// `'static` resources, and embassy-net-wiznet moves this wrapper
/// into a `'static` async task.  In `no_std` without an allocator,
/// the backing [`AtomicU32`] must be a module-level static.
pub struct CountingExtiInput<'a> {
    inner: ExtiInput<'a, Async>,
    counter: &'static AtomicU32,
}

impl<'a> CountingExtiInput<'a> {
    /// Wrap an `ExtiInput` with the given wait-completion counter.
    pub fn new(inner: ExtiInput<'a, Async>, counter: &'static AtomicU32) -> Self {
        Self { inner, counter }
    }
}

impl ErrorType for CountingExtiInput<'_> {
    type Error = Infallible;
}

impl Wait for CountingExtiInput<'_> {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        self.inner.wait_for_high().await;
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        self.inner.wait_for_low().await;
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        self.inner.wait_for_rising_edge().await;
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        self.inner.wait_for_falling_edge().await;
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        self.inner.wait_for_any_edge().await;
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
