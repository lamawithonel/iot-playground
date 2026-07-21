//! Minimal register-bus write abstraction
//!
//! One write, one bus: the only operation any of this crate's I2C-
//! backed devices need today is "write these bytes to that
//! address"-- device-specific register maps and multi-byte command
//! framing are the driver's job, not the bus's.  Shaped so any
//! addressed, interrupt-driven I2C peripheral (the N6 toolhead's
//! amp-control bus, gate G4, is the first consumer) can satisfy it
//! without dragging a read half or bus-scan API into a trait no
//! current driver needs.

/// Point-to-point register/I2C bus write.
///
/// Rationale: every current consumer (amp volume/mute control over
/// I2C1; see
/// `docs/src/projects/ars-toolhead-sensor/pinout.md`) only ever
/// writes-- there is no read-back path in scope yet-- so the trait
/// stays to that one awaitable operation rather than pulling in a
/// full `embedded-hal-async::i2c::I2c` surface this crate would
/// then have to re-export or wrap.
pub trait I2cBus {
    /// Implementation-specific bus failure (e.g., NACK, arbitration
    /// loss, bus timeout).
    type Error: core::fmt::Debug;

    /// Write `bytes` to the device at `address`.
    ///
    /// `bytes` is the whole write payload-- any register-select
    /// byte is the caller's concern, not the bus's.  Completes once
    /// the bus has clocked out the whole write; an empty `bytes` is
    /// implementation-defined (some devices treat a zero-length
    /// write as a bus probe).
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{now_or_never, MockI2cBus};

    /// MAX9744 default I2C address: ADDR1/ADDR2 pulled high,
    /// SJ3/SJ4 open on the amp board (MAX9744.pdf Table 4, p.17;
    /// Adafruit schematic-- see
    /// `docs/src/projects/ars-toolhead-sensor/pinout.md`).
    const MAX9744_ADDRESS: u8 = 0x4B;

    /// Generic consumer proof: the shape an amp volume-set driver
    /// uses-- write the desired volume byte to the MAX9744's fixed
    /// I2C address.
    async fn set_volume<B: I2cBus>(bus: &mut B, volume: u8) -> Result<(), B::Error> {
        bus.write(MAX9744_ADDRESS, &[volume]).await
    }

    #[test]
    fn test_amp_driver_writes_expected_volume_register_at_0x4b() {
        let mut bus = MockI2cBus::<4, 4>::new();
        assert_eq!(now_or_never(set_volume(&mut bus, 40)), Some(Ok(())));
        assert_eq!(bus.write_count(), 1);
        assert_eq!(bus.write(0), Some((MAX9744_ADDRESS, &[40][..])));
    }

    #[test]
    #[ignore = "RED: gate G4 -- needs a real I2C1 bus scan on the \
                NUCLEO-N657X0-Q bench; pinout.md's open questions flag \
                SJ3/SJ4 physical jumper state on the amp board in hand as \
                unverified, so even the 0x4B address assumption needs \
                bench confirmation before this can pass"]
    fn test_g4_i2c_scan_finds_amp_at_0x4b() {
        todo!(
            "requires a real I2C1 bus scan on the NUCLEO-N657X0-Q bench; \
             see docs/src/projects/ars-toolhead-sensor/pinout.md open \
             questions"
        )
    }
}
