//! Fixed-point Goertzel single-bin magnitude detector
//!
//! Detects the magnitude of one frequency bin across a dwell of
//! `N` samples using the standard Goertzel second-order recurrence.
//! Only the cosine of the bin's angular frequency is needed (no
//! sine, no complex arithmetic per sample)-- consistent with the
//! crate's "coefficients are host-computed data" policy.
//!
//! Per-sample state (`s1`, `s2`) stays within `i64`-- bounded by
//! roughly `N^2 * X / 2` for a dwell of `N` samples at amplitude
//! `X` (about 2^46 at the full 65,536-sample dwell and full-scale
//! `i16` input), well inside the 63-bit signed range.  The
//! per-sample recurrence multiplies the Q1.30 coefficient (up to
//! 2^30 in magnitude) by `s1`, though, and that product alone can
//! need on the order of 76 bits-- more than `i64` provides-- so it
//! is computed in `i128` and only the shifted-down result (back
//! within the `s1`/`s2` bound above) is narrowed to `i64`.
//! Doubling the cosine coefficient is done by shifting one bit
//! further right (`>> 29` instead of precomputing `2 * cos` and
//! shifting by `>> 30`), because `2 * cos(w)` reaches exactly
//! `2.0` at DC (`w = 0`), which overflows the Q1.30 `i32`
//! representation by one part in 2^31.  Finalizing the magnitude
//! squares the accumulated state, which needs more headroom than
//! `i64` provides after a full 65,536-sample dwell, so that step
//! is also done in `i128`.

/// Maximum supported dwell length in samples
///
/// Bounds the per-sample state growth so the once-per-dwell
/// `i128` finalize step cannot overflow.
pub const MAX_DWELL: u32 = 65_536;

/// A single Goertzel frequency-bin detector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoertzelBin {
    /// `cos(w)` in Q1.30 (not pre-doubled-- see module docs)
    coeff_q30: i32,
    s1: i64,
    s2: i64,
    samples_processed: u32,
}

impl GoertzelBin {
    /// Build a detector for the bin with cosine coefficient
    /// `coeff_q30` (Q1.30)
    ///
    /// Panics in debug builds if used past [`MAX_DWELL`] samples
    /// (checked in [`GoertzelBin::process_sample`]).
    pub const fn new(coeff_q30: i32) -> Self {
        Self {
            coeff_q30,
            s1: 0,
            s2: 0,
            samples_processed: 0,
        }
    }

    /// Feed one input sample into the recurrence
    ///
    /// # Panics
    ///
    /// Debug builds panic if called more than [`MAX_DWELL`] times
    /// on the same detector-- the once-per-dwell finalize step
    /// assumes this bound to stay within `i128` headroom.
    pub fn process_sample(&mut self, x: i16) {
        debug_assert!(
            self.samples_processed < MAX_DWELL,
            "GoertzelBin dwell exceeds MAX_DWELL"
        );
        // 2 * cos(w) * s1, computed as (coeff * s1) >> 29 rather
        // than (2 * coeff) >> 30, to avoid the DC overflow noted
        // in the module docs.  The product alone can exceed `i64`
        // (coeff up to 2^30 times |s1| up to ~2^46 needs ~76
        // bits), so it is computed in `i128`; the shifted-down
        // result narrows back to `i64` within the documented
        // `s1`/`s2` bound.
        let coeff = self.coeff_q30 as i128;
        let doubled_cos_s1 = ((coeff * self.s1 as i128) >> 29) as i64;
        let s0 = x as i64 + doubled_cos_s1 - self.s2;
        self.s2 = self.s1;
        self.s1 = s0;
        self.samples_processed += 1;
    }

    /// Number of samples processed so far
    pub const fn samples_processed(&self) -> u32 {
        self.samples_processed
    }

    /// Compute the bin magnitude from the accumulated state
    ///
    /// Uses the standard Goertzel power identity
    /// `P = s1^2 + s2^2 - 2*cos(w)*s1*s2`, evaluated in `i128` to
    /// avoid overflow, then takes an integer square root.
    pub fn magnitude(&self) -> u32 {
        let s1 = self.s1 as i128;
        let s2 = self.s2 as i128;
        let coeff2 = (self.coeff_q30 as i128) << 1; // 2*cos(w), Q1.30, safe in i128
        let cross = (coeff2 * s1 * s2) >> 30;
        let power = s1 * s1 + s2 * s2 - cross;
        // Rounding can drive a near-zero true power slightly
        // negative; clamp rather than let it underflow.
        let power_u = power.max(0) as u128;
        let power_u64 = power_u.min(u64::MAX as u128) as u64;
        u64::isqrt(power_u64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q30: f64 = (1u64 << 30) as f64;

    fn to_q30(x: f64) -> i32 {
        (x * Q30).round() as i32
    }

    /// f64 reference Goertzel magnitude over the same samples,
    /// used only inside tests.
    fn reference_magnitude(samples: &[i16], coeff: f64) -> f64 {
        let mut s1 = 0.0_f64;
        let mut s2 = 0.0_f64;
        for &x in samples {
            let s0 = x as f64 + 2.0 * coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - 2.0 * coeff * s1 * s2).sqrt()
    }

    #[test]
    fn test_matches_f64_reference_on_exact_bin_tone() {
        let fs_hz = 48_000.0;
        let dwell = 256u32;
        // Bin frequency chosen so the dwell contains an exact
        // integer number of cycles (classic Goertzel usage).
        let bin = 16.0;
        let freq_hz = bin * fs_hz / dwell as f64;
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;
        let amplitude = 10_000.0_f64;

        let samples: heapless::Vec<i16, 256> = (0..dwell)
            .map(|n| (amplitude * (theta * n as f64).sin()).round() as i16)
            .collect();

        let coeff_q30 = to_q30(theta.cos());
        let mut bin_detector = GoertzelBin::new(coeff_q30);
        for &s in samples.iter() {
            bin_detector.process_sample(s);
        }

        let got = bin_detector.magnitude() as f64;
        let want = reference_magnitude(&samples, theta.cos());
        let rel_err = (got - want).abs() / want;
        assert!(
            rel_err < 0.01,
            "got {got}, want {want}, rel_err {rel_err:.5}"
        );
    }

    #[test]
    fn test_silence_has_zero_magnitude() {
        let coeff_q30 = to_q30(0.5);
        let mut bin = GoertzelBin::new(coeff_q30);
        for _ in 0..64 {
            bin.process_sample(0);
        }
        assert_eq!(bin.magnitude(), 0);
    }

    #[test]
    fn test_dc_bin_does_not_overflow() {
        // w = 0 -> cos(w) = 1.0, the exact case the doubling
        // trick exists for.
        let coeff_q30 = to_q30(1.0);
        let mut bin = GoertzelBin::new(coeff_q30);
        for _ in 0..1000 {
            bin.process_sample(1000);
        }
        // Should not panic and should report a large, finite
        // magnitude proportional to the DC level times dwell.
        let mag = bin.magnitude();
        assert!(mag > 0);
    }

    #[test]
    fn test_dc_full_scale_at_max_dwell_does_not_overflow() {
        // Full-scale DC input run for the maximum supported
        // dwell-- the worst case for state growth.  Regression
        // test: the per-sample multiply used to overflow `i64`
        // far below MAX_DWELL (around sample 725 for this exact
        // input), long before the once-per-dwell `i128` finalize
        // step the module docs used to claim was the only
        // widening needed.
        let coeff_q30 = to_q30(1.0);
        let mut bin = GoertzelBin::new(coeff_q30);
        for _ in 0..MAX_DWELL {
            bin.process_sample(i16::MAX);
        }
        assert!(bin.magnitude() > 0);
    }

    #[test]
    fn test_bin1_full_scale_at_max_dwell_does_not_overflow() {
        // Full-scale tone on the lowest non-DC bin (one full
        // cycle across the whole dwell), run for the full
        // MAX_DWELL length-- state grows almost as fast as the DC
        // case above, through the sine term instead of a constant
        // input.
        let dwell = MAX_DWELL;
        let theta = 2.0 * core::f64::consts::PI / dwell as f64;
        let amplitude = i16::MAX as f64;

        let coeff_q30 = to_q30(theta.cos());
        let mut bin = GoertzelBin::new(coeff_q30);
        for n in 0..dwell {
            let x = (amplitude * (theta * n as f64).sin()).round() as i16;
            bin.process_sample(x);
        }
        assert!(bin.magnitude() > 0);
    }

    #[test]
    fn test_nyquist_bin_full_scale_does_not_overflow() {
        // w = pi -> cos(w) = -1.0, the mirror image of the DC
        // doubling case: state alternates sign every sample
        // instead of accumulating with one sign, but grows just
        // as fast.
        let coeff_q30 = to_q30(-1.0);
        let mut bin = GoertzelBin::new(coeff_q30);
        for n in 0..MAX_DWELL {
            let x = if n % 2 == 0 { i16::MAX } else { i16::MIN };
            bin.process_sample(x);
        }
        assert!(bin.magnitude() > 0);
    }

    #[test]
    fn test_off_bin_rejects_tone() {
        // A detector tuned away from the input tone's bin should
        // report a magnitude much smaller than one tuned to it--
        // this is what makes Goertzel a selective single-bin
        // detector rather than a broadband level meter.
        let dwell = 256u32;
        let theta_on = 2.0 * core::f64::consts::PI * 16.0 / dwell as f64;
        let theta_off = 2.0 * core::f64::consts::PI * 20.0 / dwell as f64;
        let amplitude = 10_000.0_f64;

        let mut on_bin = GoertzelBin::new(to_q30(theta_on.cos()));
        let mut off_bin = GoertzelBin::new(to_q30(theta_off.cos()));
        for n in 0..dwell {
            let x = (amplitude * (theta_on * n as f64).sin()).round() as i16;
            on_bin.process_sample(x);
            off_bin.process_sample(x);
        }

        let on_mag = on_bin.magnitude();
        let off_mag = off_bin.magnitude();
        assert!(
            off_mag < on_mag / 100,
            "off-bin magnitude {off_mag} not much smaller than on-bin {on_mag}"
        );
    }

    #[test]
    fn test_samples_processed_tracks_calls() {
        let mut bin = GoertzelBin::new(0);
        for _ in 0..10 {
            bin.process_sample(1);
        }
        assert_eq!(bin.samples_processed(), 10);
    }
}
