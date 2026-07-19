//! Fibonacci linear feedback shift register (LFSR)
//!
//! Generic maximal-length sequence (MLS) generator.  The feedback
//! tap positions are supplied by the caller as a bitmask, so the
//! kernel itself carries no domain knowledge-- [`xapp052_taps`]
//! supplies known-maximal taps (per Xilinx application note
//! XAPP052, "Efficient Shift Registers, LFSR Counters, and Long
//! Pseudo-Random Sequence Generators") for a curated set of
//! register orders.

/// A single feedback shift register, parameterized by order and
/// tap mask
///
/// `taps_mask` has bit `t - 1` set for every 1-indexed tap position
/// `t` that participates in the feedback XOR.  The register order
/// is `1..=32`; state and mask are tracked in a `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lfsr {
    taps_mask: u32,
    mask: u32,
    /// Masked, non-zero seed actually loaded at construction--
    /// exposed via [`Lfsr::effective_seed`] so callers can record
    /// what actually ran even when the requested seed was masked
    /// or substituted.
    initial_seed: u32,
    state: u32,
}

impl Lfsr {
    /// Build an LFSR of the given `order` (1..=32) and tap mask
    ///
    /// `seed` is masked to `order` bits; if the masked seed is
    /// zero (the all-zeros state, which the register can never
    /// leave), it is replaced with `1`.  Use
    /// [`Lfsr::effective_seed`] to read back what was actually
    /// loaded.
    pub const fn new_with_taps(order: u8, taps_mask: u32, seed: u32) -> Self {
        let mask = if order >= 32 {
            u32::MAX
        } else {
            (1u32 << order) - 1
        };
        let mut s = seed & mask;
        if s == 0 {
            s = 1;
        }
        Self {
            taps_mask: taps_mask & mask,
            mask,
            initial_seed: s,
            state: s,
        }
    }

    /// Build an LFSR of the given `order` using the curated
    /// [`xapp052_taps`] table
    ///
    /// Returns `None` if `order` has no entry in the table.
    pub fn new(order: u8, seed: u32) -> Option<Self> {
        let taps_mask = xapp052_taps(order)?;
        Some(Self::new_with_taps(order, taps_mask, seed))
    }

    /// The masked, non-zero seed actually loaded at construction
    pub const fn effective_seed(&self) -> u32 {
        self.initial_seed
    }

    /// Advance the register one step and return the feedback bit
    ///
    /// The feedback bit (XOR of all tapped state bits) is both the
    /// generator's pseudo-random output and the bit shifted into
    /// the register.
    pub fn next_bit(&mut self) -> bool {
        let fb = (self.state & self.taps_mask).count_ones() & 1;
        self.state = ((self.state << 1) | fb) & self.mask;
        fb != 0
    }
}

/// Known-maximal Fibonacci LFSR tap masks (XAPP052), keyed by
/// register order
///
/// Only a curated subset of orders is included-- enough to cover
/// the ARS MLS excitation use case.  Each entry is verified by
/// [`tests::test_all_taps_are_maximal_length`] to produce a full
/// period of `2^order - 1`.
pub fn xapp052_taps(order: u8) -> Option<u32> {
    Some(match order {
        4 => 0x0000_000C,  // taps 4, 3
        5 => 0x0000_0014,  // taps 5, 3
        7 => 0x0000_0060,  // taps 7, 6
        9 => 0x0000_0110,  // taps 9, 5
        10 => 0x0000_0240, // taps 10, 7
        11 => 0x0000_0500, // taps 11, 9
        15 => 0x0000_6000, // taps 15, 14
        16 => 0x0000_D008, // taps 16, 15, 13, 4
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every curated order must produce a full-length maximal
    /// sequence (period `2^order - 1`) from a fixed seed.  This is
    /// the safety net for the hand-transcribed XAPP052 tap masks.
    #[test]
    fn test_all_taps_are_maximal_length() {
        for order in [4u8, 5, 7, 9, 10, 11, 15, 16] {
            let mut lfsr = Lfsr::new(order, 1).unwrap();
            let expected_period = (1u32 << order) - 1;
            let start = lfsr.state;
            let mut period = 0u32;
            loop {
                lfsr.next_bit();
                period += 1;
                if lfsr.state == start {
                    break;
                }
                assert!(
                    period <= expected_period,
                    "order {order} did not return to start within expected period"
                );
            }
            assert_eq!(period, expected_period, "order {order} period mismatch");
        }
    }

    #[test]
    fn test_unknown_order_returns_none() {
        assert!(Lfsr::new(6, 1).is_none());
        assert!(Lfsr::new(17, 1).is_none());
    }

    #[test]
    fn test_zero_seed_is_substituted() {
        let lfsr = Lfsr::new(4, 0).unwrap();
        assert_eq!(lfsr.effective_seed(), 1);
    }

    #[test]
    fn test_seed_is_masked_to_order() {
        // order 4 -> mask 0xF; seed 0x1F should be masked to 0xF
        let lfsr = Lfsr::new(4, 0x1F).unwrap();
        assert_eq!(lfsr.effective_seed(), 0xF);
    }

    #[test]
    fn test_deterministic_stream() {
        let mut a = Lfsr::new(9, 123).unwrap();
        let mut b = Lfsr::new(9, 123).unwrap();
        for _ in 0..64 {
            assert_eq!(a.next_bit(), b.next_bit());
        }
    }
}
