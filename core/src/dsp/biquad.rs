//! Fixed-point biquad filter (Direct Form II Transposed)
//!
//! A general 5-coefficient second-order section, used by
//! `ars::synth` to model resonant plant behavior.  Coefficients
//! are Q2.30 (they can reach magnitude 2.0, e.g. `a1` for a pole
//! near the unit circle); state (`z1`, `z2`) is kept in the
//! natural integer scale of the samples flowing through the
//! filter, per the crate-level fixed-point conventions.

/// A single biquad (second-order IIR) section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Biquad {
    b0: i32,
    b1: i32,
    b2: i32,
    a1: i32,
    a2: i32,
    z1: i64,
    z2: i64,
}

impl Biquad {
    /// Build a biquad from Q2.30 coefficients (`b0`, `b1`, `b2`,
    /// `a1`, `a2`), matching the standard transfer function
    ///
    /// ```text
    /// H(z) = (b0 + b1 z^-1 + b2 z^-2) / (1 + a1 z^-1 + a2 z^-2)
    /// ```
    ///
    /// State starts at zero (filter at rest).
    pub const fn new(b0: i32, b1: i32, b2: i32, a1: i32, a2: i32) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0,
            z2: 0,
        }
    }

    /// Reset filter state to zero (filter at rest), keeping
    /// coefficients
    pub fn reset(&mut self) {
        self.z1 = 0;
        self.z2 = 0;
    }

    /// Process one input sample, returning the filtered output
    ///
    /// `x` and the returned sample are in the same natural integer
    /// scale (e.g., Q15 audio counts); coefficients are Q2.30 and
    /// are rescaled internally.
    pub fn process(&mut self, x: i32) -> i32 {
        let xi = x as i64;
        let y = ((self.b0 as i64 * xi) >> 30) + self.z1;
        self.z1 = ((self.b1 as i64 * xi) >> 30) - ((self.a1 as i64 * y) >> 30) + self.z2;
        self.z2 = ((self.b2 as i64 * xi) >> 30) - ((self.a2 as i64 * y) >> 30);
        y as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q30: f64 = (1u64 << 30) as f64;

    fn to_q30(x: f64) -> i32 {
        (x * Q30).round() as i32
    }

    /// f64 reference implementation of the same Direct Form II
    /// Transposed recurrence, used only inside tests.
    struct RefBiquad {
        b0: f64,
        b1: f64,
        b2: f64,
        a1: f64,
        a2: f64,
        z1: f64,
        z2: f64,
    }

    impl RefBiquad {
        fn process(&mut self, x: f64) -> f64 {
            let y = self.b0 * x + self.z1;
            self.z1 = self.b1 * x - self.a1 * y + self.z2;
            self.z2 = self.b2 * x - self.a2 * y;
            y
        }
    }

    /// A simple two-pole resonator (no zeros) targeting a
    /// resonance at `freq_hz` with pole radius `r` (closer to 1.0
    /// == higher Q).  Mirrors the design used in
    /// `ars::synth::Plant`.
    fn resonator_coeffs(freq_hz: f64, fs_hz: f64, r: f64) -> (f64, f64, f64, f64, f64) {
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;
        let b0 = 1.0 - r;
        let a1 = -2.0 * r * theta.cos();
        let a2 = r * r;
        (b0, 0.0, 0.0, a1, a2)
    }

    #[test]
    fn test_matches_f64_reference_impulse_response() {
        let (b0, b1, b2, a1, a2) = resonator_coeffs(1200.0, 48_000.0, 0.995);

        let mut fixed = Biquad::new(to_q30(b0), to_q30(b1), to_q30(b2), to_q30(a1), to_q30(a2));
        let mut reference = RefBiquad {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        };

        // Drive both with a unit impulse scaled to Q15 full scale
        // so relative rounding error stays small.  This resonator
        // has a pole close to the unit circle (r = 0.995, chosen
        // for a narrow, high-Q peak); Q2.30 coefficient rounding
        // compounds through that near-unstable feedback loop more
        // than a gentle filter would, so the bound below is a
        // measured empirical envelope (observed max ~79 over 500
        // samples), not a bit-exact match (see decision 2 on
        // tolerance-based f64 comparisons).
        let impulse = 32_767.0_f64;
        for n in 0..500 {
            let x = if n == 0 { impulse } else { 0.0 };
            let got = fixed.process(x as i32) as f64;
            let want = reference.process(x);
            let err = (got - want).abs();
            assert!(err < 100.0, "sample {n}: got {got}, want {want:.3}");
        }
    }

    #[test]
    fn test_reset_clears_state() {
        let mut bq = Biquad::new(to_q30(0.5), 0, 0, to_q30(-0.5), to_q30(0.1));
        bq.process(32_000);
        bq.reset();
        assert_eq!(bq.z1, 0);
        assert_eq!(bq.z2, 0);
    }

    #[test]
    fn test_zero_input_is_silent_from_rest() {
        let mut bq = Biquad::new(
            to_q30(0.5),
            to_q30(0.2),
            to_q30(0.1),
            to_q30(-0.5),
            to_q30(0.1),
        );
        assert_eq!(bq.process(0), 0);
    }
}
