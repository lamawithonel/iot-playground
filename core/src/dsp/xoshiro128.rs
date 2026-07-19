//! `xoshiro128++` pseudo-random number generator
//!
//! A small, public-domain, non-cryptographic PRNG (Blackman and
//! Vigna) used to seed noise floors and dither in synthetic signal
//! generators.  Deterministic and reproducible: the same seed
//! always produces the same output stream, which is required for
//! byte-exact golden vectors.

/// `xoshiro128++` generator state
///
/// Constructed from a single `u32` seed via [`Xoshiro128PlusPlus::new`],
/// which expands the seed into the four-word state using SplitMix32
/// so that small or zero seeds still produce well-mixed state (the
/// all-zero state is invalid for xoshiro and is never reachable
/// through this constructor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xoshiro128PlusPlus {
    s: [u32; 4],
}

impl Xoshiro128PlusPlus {
    /// Create a generator from a 32-bit seed
    ///
    /// Internally expands the seed with SplitMix32 to fill all
    /// four state words-- a raw seed placed directly into a single
    /// word (with the rest zeroed) would produce poor early output.
    pub const fn new(seed: u32) -> Self {
        let mut sm = seed;
        let mut s = [0u32; 4];
        let mut i = 0;
        while i < 4 {
            sm = sm.wrapping_add(0x9E37_79B9);
            let mut z = sm;
            z = (z ^ (z >> 16)).wrapping_mul(0x21F0_AAAD);
            z = (z ^ (z >> 15)).wrapping_mul(0x735A_2D97);
            z ^= z >> 15;
            s[i] = z;
            i += 1;
        }
        Self { s }
    }

    /// Generate the next pseudo-random `u32`
    pub fn next_u32(&mut self) -> u32 {
        let result = (self.s[0].wrapping_add(self.s[3]))
            .rotate_left(7)
            .wrapping_add(self.s[0]);

        let t = self.s[1] << 9;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(11);

        result
    }

    /// Generate the next pseudo-random sample scaled to
    /// `[-amplitude, amplitude]`
    ///
    /// Used to synthesize a bounded noise floor.  `amplitude` is
    /// the peak magnitude in the caller's natural sample units
    /// (e.g., Q15 counts).
    pub fn next_i16_bounded(&mut self, amplitude: i16) -> i16 {
        if amplitude <= 0 {
            return 0;
        }
        let span = (2 * amplitude as i32) + 1;
        let raw = (self.next_u32() >> 8) as i32; // 24 usable bits
        (raw % span) as i16 - amplitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_zero_is_not_all_zero_state() {
        let mut rng = Xoshiro128PlusPlus::new(0);
        // The all-zero xoshiro state is a fixed point (never
        // produces nonzero output); SplitMix32 expansion must
        // avoid it even for a zero seed.
        assert_ne!(rng.next_u32(), 0);
    }

    #[test]
    fn test_deterministic_stream() {
        let mut a = Xoshiro128PlusPlus::new(42);
        let mut b = Xoshiro128PlusPlus::new(42);
        for _ in 0..64 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn test_different_seeds_diverge() {
        let mut a = Xoshiro128PlusPlus::new(1);
        let mut b = Xoshiro128PlusPlus::new(2);
        let seq_a: heapless::Vec<u32, 16> = (0..16).map(|_| a.next_u32()).collect();
        let seq_b: heapless::Vec<u32, 16> = (0..16).map(|_| b.next_u32()).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn test_bounded_within_range() {
        let mut rng = Xoshiro128PlusPlus::new(7);
        for _ in 0..1000 {
            let v = rng.next_i16_bounded(104);
            assert!((-104..=104).contains(&v));
        }
    }

    #[test]
    fn test_bounded_zero_amplitude_is_silent() {
        let mut rng = Xoshiro128PlusPlus::new(7);
        for _ in 0..8 {
            assert_eq!(rng.next_i16_bounded(0), 0);
        }
    }
}
