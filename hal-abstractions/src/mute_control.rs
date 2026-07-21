//! Amplifier mute-line digital output abstraction
//!
//! The N6 toolhead's amp board inverts its MUTE line: the header
//! net MUTE_INV drives a transistor stage that pulls the MAX9744's
//! actual MUTE pin, so engaging mute means driving the MCU-facing
//! header pin LOW-- the opposite sense of the bare chip datasheet
//! (see `docs/src/projects/ars-toolhead-sensor/pinout.md`,
//! `AMP_MUTE_N`).  This trait hides that inversion behind a
//! semantic `mute`/`unmute` pair so callers never encode the
//! board's polarity quirk themselves.

/// Digital mute control for the amp's (possibly inverted) mute
/// line.
///
/// Rationale: the only operation any excitation-sweep or fault-
/// handling caller needs is "silence the amp" / "un-silence the
/// amp"-- which physical level that takes is a board wiring detail
/// the implementation owns, not the caller.
pub trait MuteControl {
    /// Implementation-specific GPIO failure.
    type Error: core::fmt::Debug;

    /// Engage mute.
    fn mute(&mut self) -> Result<(), Self::Error>;

    /// Disengage mute.
    fn unmute(&mut self) -> Result<(), Self::Error>;

    /// Whether mute is currently engaged.
    fn is_muted(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockMuteControl;

    #[test]
    fn test_mute_control_drives_low_to_engage_mute() {
        let mut mute = MockMuteControl::new();
        // Default-high pull-up (R15): unmuted, line high at reset.
        assert!(!mute.is_muted());
        assert!(!mute.line_driven_low());

        assert_eq!(mute.mute(), Ok(()));
        assert!(mute.is_muted());
        assert!(
            mute.line_driven_low(),
            "engaging mute must drive the AMP_MUTE_N line LOW-- the \
             Adafruit board's inverted MUTE_INV sense (pinout.md)"
        );

        assert_eq!(mute.unmute(), Ok(()));
        assert!(!mute.is_muted());
        assert!(!mute.line_driven_low());
    }

    #[test]
    #[ignore = "RED: gate G4-adjacent -- needs bench verification of which \
                rail pulls the MUTE_INV net (R15); pinout.md lists this as \
                an open EE-review question (3.3V vs the 5V-side \
                diode-shifted domain), and power-up pop/click behavior on \
                mute/unmute transitions is unmeasured"]
    fn test_amp_mute_polarity_confirmed_on_bench_gpio() {
        todo!(
            "requires driving the real PD0 GPIO against the amp board on \
             the NUCLEO-N657X0-Q bench and observing MUTE_INV/pop-click \
             behavior; see docs/src/projects/ars-toolhead-sensor/pinout.md \
             open questions"
        )
    }
}
