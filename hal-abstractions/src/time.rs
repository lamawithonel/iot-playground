//! Wall-clock timestamp vocabulary
//!
//! Canonical types shared by the [`crate::rtc::Rtc`] trait.
//!
//! ## Relationship to `iot_core::time`
//!
//! `iot-core` depends on `hal-abstractions` (not the reverse), so
//! `hal-abstractions` cannot import `iot_core::time::Timestamp`
//! without creating a dependency cycle.  These types are therefore
//! a deliberate mirror of `iot_core::time::{Timestamp, RtcError}`--
//! same fields, same semantics, kept in sync by hand.  Only the
//! subset needed by the `Rtc` trait is mirrored here; NTP parsing
//! and calendar math stay `core`-only.
//!
//! Unifying the two copies (moving the canonical definition here
//! with `core` re-exporting it) is a follow-up that touches
//! `core/` and is out of scope for this change.  Until that lands,
//! treat this module as the canonical copy and
//! `iot_core::time::{Timestamp, RtcError}` as the alias-to-be; the
//! two are kept in sync by hand with zero compile-time
//! enforcement, so silent drift between them is possible.
//!
//! **Before any board implements [`crate::rtc::Rtc`]**, file that
//! follow-up with `core/` in scope: delete `core`'s definitions,
//! add `pub use hal_abstractions::time::{RtcError, Timestamp};`
//! in `core/src/time/mod.rs`, and forward `core`'s `defmt` feature
//! (`defmt = ["dep:defmt", "hal-abstractions/defmt"]`).  A board's
//! `Rtc` re-export (e.g.
//! `boards/feather-stm32f405/src/time/rtc.rs`) then resolves to
//! this same nominal type with a zero-line diff on that side;
//! without it, an `Rtc` implementation must field-copy convert
//! between the two structurally identical but nominally distinct
//! types.

/// Timestamp with microsecond precision.
///
/// Mirrors `iot_core::time::Timestamp` field-for-field (see the
/// module docs for why this is a mirror, not a re-export).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Timestamp {
    /// Unix timestamp in seconds since epoch (1970-01-01 00:00:00
    /// UTC).
    pub unix_secs: u64,
    /// Microseconds component (0-999,999).
    pub micros: u32,
}

impl Timestamp {
    /// Create a new timestamp.
    ///
    /// If `micros` exceeds 999,999 the overflow is carried into
    /// `unix_secs` so the invariant `micros < 1_000_000` always
    /// holds.
    ///
    /// ```
    /// use hal_abstractions::time::Timestamp;
    ///
    /// let ts = Timestamp::new(100, 1_500_000);
    /// assert_eq!(ts.unix_secs, 101);
    /// assert_eq!(ts.micros, 500_000);
    /// ```
    pub const fn new(unix_secs: u64, micros: u32) -> Self {
        let extra_secs = (micros / 1_000_000) as u64;
        Self {
            unix_secs: unix_secs + extra_secs,
            micros: micros % 1_000_000,
        }
    }
}

/// RTC operation errors.
///
/// Mirrors `iot_core::time::RtcError` (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RtcError {
    /// RTC not initialized.
    NotInitialized,
    /// RTC hardware error.
    HardwareError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_normalizes_micros_overflow() {
        let ts = Timestamp::new(100, 1_500_000);
        assert_eq!(ts.unix_secs, 101);
        assert_eq!(ts.micros, 500_000);
    }

    #[test]
    fn new_normalizes_exact_boundary() {
        let ts = Timestamp::new(100, 1_000_000);
        assert_eq!(ts.unix_secs, 101);
        assert_eq!(ts.micros, 0);
    }

    #[test]
    fn new_keeps_in_range_micros_untouched() {
        let ts = Timestamp::new(100, 999_999);
        assert_eq!(ts.unix_secs, 100);
        assert_eq!(ts.micros, 999_999);
    }
}
