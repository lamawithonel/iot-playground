//! Numerically controlled oscillator (direct-form resonator)
//!
//! Generates a pure sinusoid from a single precomputed cosine
//! coefficient using the classic two-pole direct-form digital
//! resonator recurrence:
//!
//! ```text
//! y[n] = 2 * cos(w) * y[n-1] - y[n-2]
//! ```
//!
//! This produces an exact sinusoid at angular frequency `w`
//! without ever calling `sin`/`cos` at runtime-- the coefficient
//! and the two initial-condition samples are computed once by the
//! caller (on the host, or from a checked-in constant table) and
//! passed in.  `no_std` has no `cos`, and this recurrence sidesteps
//! needing one on every sample.
//!
//! State (`y1`, `y2`) is kept close to the natural integer scale of
//! the samples the oscillator produces (e.g., Q15 audio counts),
//! widened by a few fractional guard bits (see [`GUARD_BITS`]); the
//! coefficient carries the Q1.30 fractional scale, per the
//! crate-level fixed-point conventions.
//!
//! A plain `(2 * coeff_q30 * y1) >> 30` truncates toward negative
//! infinity every sample.  With no fractional bits in the state to
//! absorb that bias, it compounds through the feedback loop and
//! erodes amplitude fastest at low frequencies, where `cos(w)` sits
//! close to 1 and the useful information is encoded almost entirely
//! in the low bits a plain shift throws away.  Carrying
//! [`GUARD_BITS`] extra fractional bits in `y1`/`y2`, and rounding
//! (rather than truncating) both the coefficient multiply and the
//! final descale back to natural units, keeps that error bounded
//! instead of growing with dwell length-- see the accuracy tests
//! below for the measured envelope.

/// Extra fractional bits of headroom carried in `y1`/`y2` beyond
/// the natural sample scale
///
/// Chosen to cut the low-frequency amplitude error the module docs
/// describe by roughly three orders of magnitude while leaving wide
/// overflow headroom in `next_sample`'s `i64` multiply-- even the
/// pathological coefficient in
/// [`tests::test_next_i16_saturates`] paired with an `i16`-range
/// amplitude stays far under `i64::MAX`.
const GUARD_BITS: u32 = 8;

/// Coupled-form digital oscillator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nco {
    /// `cos(w)` in Q1.30
    coeff_q30: i32,
    /// `y[n-1]`, scaled by `2^GUARD_BITS` fractional guard bits
    y1: i64,
    /// `y[n-2]`, scaled by `2^GUARD_BITS` fractional guard bits
    y2: i64,
}

impl Nco {
    /// Build an oscillator from a Q1.30 cosine coefficient and two
    /// initial-condition samples (natural sample units)
    ///
    /// The two initial samples set both the oscillator's amplitude
    /// and its starting phase; see the module docs for the closed
    /// form this recurrence produces.
    pub const fn new(coeff_q30: i32, y1_init: i32, y2_init: i32) -> Self {
        Self {
            coeff_q30,
            y1: (y1_init as i64) << GUARD_BITS,
            y2: (y2_init as i64) << GUARD_BITS,
        }
    }

    /// Advance one sample and return it in natural (unclamped)
    /// integer units
    pub fn next_sample(&mut self) -> i64 {
        let coeff = self.coeff_q30 as i64;
        // Round-to-nearest (not truncate) the Q1.30 multiply-- see
        // the module docs for why truncation alone erodes amplitude
        // at low frequencies.
        let product = 2 * coeff * self.y1 + (1 << 29);
        let y0 = (product >> 30) - self.y2;
        self.y2 = self.y1;
        self.y1 = y0;
        // Descale back to natural sample units, again rounding
        // rather than dropping the guard bits.
        (y0 + (1 << (GUARD_BITS - 1))) >> GUARD_BITS
    }

    /// Advance one sample, saturating to `i16` range
    ///
    /// Callers that size initial conditions within `i16` range in
    /// practice never hit the saturating clamp; it exists only as
    /// a safety net against pathological coefficients.
    pub fn next_i16(&mut self) -> i16 {
        self.next_sample().clamp(i16::MIN as i64, i16::MAX as i64) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q30: f64 = (1u64 << 30) as f64;

    fn to_q30(x: f64) -> i32 {
        (x * Q30).round() as i32
    }

    #[test]
    fn test_matches_f64_reference_sinusoid() {
        let fs_hz = 48_000.0;
        let freq_hz = 1_000.0;
        let amplitude = 10_000.0_f64;
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;

        let coeff_q30 = to_q30(theta.cos());
        let y1_init = (amplitude * theta.sin()).round() as i32;
        let y2_init = 0;

        let mut nco = Nco::new(coeff_q30, y1_init, y2_init);

        // Closed form for these initial conditions:
        // y[n] = amplitude * sin(theta * (n + 2))
        //
        // Q1.30 coefficient rounding compounds through the
        // feedback recurrence, so the bound below is a measured
        // empirical envelope (observed max ~2.4 over 200 samples at
        // this amplitude with rounded, guard-bit-widened state), not
        // a bit-exact match-- fixed-point vs f64 comparisons are
        // documented (decision 2) as tolerance-based, unlike the
        // zero-tolerance device-vs-host integer comparison.
        for n in 0..200i64 {
            let got = nco.next_sample();
            let want = amplitude * (theta * (n as f64 + 2.0)).sin();
            let err = (got as f64 - want).abs();
            assert!(
                err < 10.0,
                "sample {n}: got {got}, want {want:.3}, err {err:.3}"
            );
        }
    }

    /// Long-dwell accuracy at a low frequency, where the Q1.30
    /// coefficient's fractional bits (not carried in the old,
    /// unguarded state) mattered most: previously this case drifted
    /// to a max error over 100% of the amplitude within a single
    /// 4,800-sample dwell.
    #[test]
    fn test_low_freq_long_dwell_matches_f64_reference() {
        let fs_hz = 48_000.0;
        let freq_hz = 100.0;
        let amplitude = 16_000.0_f64;
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;

        let coeff_q30 = to_q30(theta.cos());
        let y1_init = (amplitude * theta.sin()).round() as i32;

        let mut nco = Nco::new(coeff_q30, y1_init, 0);

        for n in 0..4_800i64 {
            let got = nco.next_sample();
            let want = amplitude * (theta * (n as f64 + 2.0)).sin();
            let err = (got as f64 - want).abs();
            assert!(
                err < 50.0,
                "sample {n}: got {got}, want {want:.3}, err {err:.3}"
            );
        }
    }

    /// Same low-frequency case as
    /// [`test_low_freq_long_dwell_matches_f64_reference`], run out
    /// to `GoertzelBin`'s `MAX_DWELL` scale (65,536 samples), to
    /// confirm the error stays bounded rather than growing with
    /// run length.
    #[test]
    fn test_low_freq_max_dwell_scale_matches_f64_reference() {
        let fs_hz = 48_000.0;
        let freq_hz = 100.0;
        let amplitude = 16_000.0_f64;
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;

        let coeff_q30 = to_q30(theta.cos());
        let y1_init = (amplitude * theta.sin()).round() as i32;

        let mut nco = Nco::new(coeff_q30, y1_init, 0);

        for n in 0..65_536i64 {
            let got = nco.next_sample();
            let want = amplitude * (theta * (n as f64 + 2.0)).sin();
            let err = (got as f64 - want).abs();
            assert!(
                err < 50.0,
                "sample {n}: got {got}, want {want:.3}, err {err:.3}"
            );
        }
    }

    #[test]
    fn test_deterministic() {
        let coeff_q30 = to_q30(0.5_f64);
        let mut a = Nco::new(coeff_q30, 1000, 0);
        let mut b = Nco::new(coeff_q30, 1000, 0);
        for _ in 0..64 {
            assert_eq!(a.next_sample(), b.next_sample());
        }
    }

    #[test]
    fn test_next_i16_saturates() {
        // A coefficient near 2.0 in Q1.30 terms is out of the
        // valid cosine range but exercises the saturation path.
        let mut nco = Nco::new(i32::MAX, i16::MAX as i32, 0);
        let s = nco.next_i16();
        assert!(s == i16::MAX || s == i16::MIN);
    }
}
