//! Mic-path and audio-path quality metrics
//!
//! Small, pure integer helpers over already-computed magnitudes and
//! sample buffers-- no capture, no I/O.  These feed the bench-
//! acceptance gates described in
//! `docs/src/projects/ars-toolhead-sensor/pinout.md`'s "Spike
//! Gates" section (G2, mic-path noise floor and flatness; G3,
//! PWM-audio SNR/THD): the "passable-now" halves of those gates
//! run against the synthetic [`super::synth::Plant`] and the
//! existing [`crate::dsp::goertzel::GoertzelBin`] kernel; the
//! bench-hardware halves are the `#[ignore]`d specs in
//! `core/tests/hil_gates.rs`.
//!
//! `no_std` has no floating-point transcendentals (no `log10`,
//! `sqrt` needs a crate this workspace does not depend on) and this
//! crate's `dsp` kernels are integer-only by policy (see
//! [`crate::dsp`] module docs), so [`band_flatness_db`] computes its
//! decibel figure with a small fixed-point `log2` approximation
//! rather than pulling in a new dependency.

/// Peak amplitude across a buffer of samples, in Q15 counts
///
/// A crude but honest noise-floor estimator: [`super::synth::Plant`]'s
/// synthetic noise floor is documented (see
/// `super::synth::NOISE_FLOOR_Q15`) as a *peak* amplitude, uniformly
/// distributed over `[-NOISE_FLOOR_Q15, NOISE_FLOOR_Q15]`, not an
/// RMS figure-- so the matching estimator here is the sample
/// buffer's own peak magnitude, not an RMS/energy computation.
pub fn noise_floor_q15(samples: &[i16]) -> u32 {
    samples
        .iter()
        .map(|&s| u32::from(s.unsigned_abs()))
        .max()
        .unwrap_or(0)
}

/// Fixed-point `log2(x)` for a Q16.16 input, returned in Q16.16
/// (i.e., the true value times `65536`, rounded toward zero)
///
/// `x_q16` must be positive (checked with `debug_assert!` only--
/// see the "no panics in release" note below).  Splits `x` into its
/// integer octave (the bit position of the leading one) and
/// linearly interpolates the mantissa within that octave
/// (`log2(2^n * m) ~= n + (m - 1)` for `m` in `[1.0, 2.0)`)-- a
/// standard, low-cost `log2` approximation whose worst-case error
/// (at the octave midpoint, `m = 1.5`) is about `0.086` bits, i.e.
/// roughly half a dB once [`band_flatness_db`] rescales it.  That is
/// far more precision than the metric needs: no gate in the source
/// docs states a numeric dB threshold to compare against (see
/// `core/tests/hil_gates.rs`), so exactness here would be spurious.
///
/// No panics in release builds: an `x_q16` of `0` falls through to
/// `leading_zeros() == 64`, giving `msb = -1` and a very negative
/// (not wrapped or trapped) result-- callers that can pass zero
/// should check for it first, as [`band_flatness_db`] does.
fn log2_q16(x_q16: u64) -> i32 {
    debug_assert!(x_q16 > 0, "log2_q16 requires a positive input");
    // Bit position of the leading one, i.e. floor(log2(x_q16)).
    let msb = 63 - x_q16.leading_zeros() as i32;
    // Normalize the mantissa into [65536, 131072), i.e. [1.0, 2.0)
    // in Q16.16.
    let mantissa_q16 = if msb >= 16 {
        x_q16 >> (msb - 16)
    } else {
        x_q16 << (16 - msb)
    };
    // log2(x) = (msb - 16) + (mantissa - 1.0), both terms in Q16.16.
    (msb - 16) * 65_536 + (mantissa_q16 as i64 - 65_536) as i32
}

/// Peak-to-trough ratio across a bank of frequency-bin magnitudes,
/// in decibels
///
/// Takes plain magnitudes (e.g. the `.magnitude()` output of a bank
/// of [`crate::dsp::goertzel::GoertzelBin`] detectors tuned across a
/// sweep band)-- reusing the existing Goertzel kernel rather than
/// adding a new DSP primitive.  `0` means every bin reported the
/// same magnitude (an ideally flat response); larger values mean a
/// wider spread, e.g. a resonator dominating one bin over the rest.
///
/// Returns `0` for fewer than two bins (nothing to compare) and
/// `i32::MAX` if the quietest bin reported zero magnitude next to a
/// nonzero one (an infinite ratio-- a dead/silent bin).
pub fn band_flatness_db(bin_mags: &[u32]) -> i32 {
    if bin_mags.len() < 2 {
        return 0;
    }
    let max = bin_mags.iter().copied().max().unwrap_or(0);
    let min = bin_mags.iter().copied().min().unwrap_or(0);
    if min == 0 {
        return if max == 0 { 0 } else { i32::MAX };
    }
    let ratio_q16 = (u64::from(max) << 16) / u64::from(min);
    let log2_ratio_q16 = i64::from(log2_q16(ratio_q16));
    // dB = log2(ratio) * 20/log2(10); 6_020_600 / 1_000_000
    // approximates 20/log2(10) = 6.0205999...  Both operands stay
    // well within i64 (log2_ratio_q16 is bounded by a u32/u32 ratio
    // widened to Q16.16, nowhere near i64::MAX / 6_020_600).
    let db_q16 = log2_ratio_q16 * 6_020_600 / 1_000_000;
    ((db_q16 + 32_768) >> 16) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ars::generators::MlsGen;
    use crate::ars::synth::{Plant, PlantVariant, NOISE_FLOOR_Q15, PLANT_FS_HZ};
    use crate::dsp::biquad::Biquad;
    use crate::dsp::goertzel::GoertzelBin;
    use crate::dsp::nco::Nco;

    const Q30: f64 = (1u64 << 30) as f64;

    fn to_q30(x: f64) -> i32 {
        (x * Q30).round() as i32
    }

    fn goertzel_bank_mags(bins_hz: &[f64], fs_hz: f64, samples: &[i16]) -> heapless::Vec<u32, 8> {
        let mut mags = heapless::Vec::new();
        for &f in bins_hz {
            let theta = 2.0 * core::f64::consts::PI * f / fs_hz;
            let mut bin = GoertzelBin::new(to_q30(theta.cos()));
            for &s in samples {
                bin.process_sample(s);
            }
            mags.push(bin.magnitude()).unwrap();
        }
        mags
    }

    // -- noise_floor_q15 -----------------------------------------

    #[test]
    fn test_noise_floor_q15_recovers_near_documented_floor_on_silence_in() {
        // Silence-in run against the synthetic plant (Absent
        // variant): every output sample is pure noise floor, so the
        // peak-amplitude estimator should land close to
        // NOISE_FLOOR_Q15 (see synth.rs's own
        // test_silence_in_yields_only_noise_floor for the matching
        // per-sample bound).
        let mut plant = Plant::new(PlantVariant::Absent, 11);
        let mut samples = [0i16; 4_000];
        plant.process_buf(&mut samples);

        let floor = noise_floor_q15(&samples);
        let documented = u32::try_from(NOISE_FLOOR_Q15).unwrap();
        assert!(
            floor <= documented,
            "noise_floor_q15 {floor} exceeds the documented peak {documented}"
        );
        // With 4,000 uniform draws over [-104, 104], the sample
        // maximum is overwhelmingly likely to land within 20% of
        // the true bound; a much lower reading would mean the
        // estimator or the plant's noise floor drifted.
        assert!(
            floor >= documented * 8 / 10,
            "noise_floor_q15 {floor} is not near the documented peak {documented}"
        );
    }

    #[test]
    fn test_noise_floor_q15_is_zero_for_true_silence() {
        assert_eq!(noise_floor_q15(&[0, 0, 0]), 0);
    }

    #[test]
    fn test_noise_floor_q15_empty_buffer_is_zero() {
        assert_eq!(noise_floor_q15(&[]), 0);
    }

    #[test]
    fn test_noise_floor_q15_picks_the_largest_magnitude() {
        assert_eq!(noise_floor_q15(&[10, -50, 30, -20]), 50);
        assert_eq!(
            noise_floor_q15(&[i16::MIN]),
            u32::try_from(i16::MAX).unwrap() + 1
        );
    }

    // -- log2_q16 --------------------------------------------------

    #[test]
    fn test_log2_q16_matches_f64_reference() {
        for x in [1.0_f64, 1.5, 2.0, 3.2, 10.0, 100.0, 1000.0, 65536.0] {
            let x_q16 = (x * 65_536.0).round() as u64;
            let got = f64::from(log2_q16(x_q16)) / 65_536.0;
            let want = x.log2();
            let err = (got - want).abs();
            assert!(err < 0.1, "log2({x}): got {got}, want {want}, err {err}");
        }
    }

    #[test]
    fn test_log2_q16_of_one_is_zero() {
        assert_eq!(log2_q16(65_536), 0);
    }

    // -- band_flatness_db -------------------------------------------

    #[test]
    fn test_band_flatness_db_needs_at_least_two_bins() {
        assert_eq!(band_flatness_db(&[]), 0);
        assert_eq!(band_flatness_db(&[12_345]), 0);
    }

    #[test]
    fn test_band_flatness_db_zero_for_identical_magnitudes() {
        assert_eq!(band_flatness_db(&[500, 500, 500, 500]), 0);
    }

    #[test]
    fn test_band_flatness_db_matches_known_ratio() {
        // A 2x peak-to-trough ratio is exactly 20*log10(2) ~= 6.02
        // dB-- close enough to the crate's own well-known Q15
        // noise-floor dBFS estimate (see synth.rs) to sanity-check
        // by hand.
        let db = band_flatness_db(&[1_000, 2_000]);
        assert!(
            (5..=7).contains(&db),
            "expected ~6 dB for a 2x ratio, got {db}"
        );
    }

    #[test]
    fn test_band_flatness_db_infinite_for_dead_bin() {
        assert_eq!(band_flatness_db(&[0, 500]), i32::MAX);
    }

    #[test]
    fn test_band_flatness_db_zero_for_all_silent_bins() {
        assert_eq!(band_flatness_db(&[0, 0, 0]), 0);
    }

    #[test]
    fn test_band_flatness_db_near_zero_on_flat_spectrum_stand_in() {
        // Drive a broadband MLS excitation straight into a bank of
        // GoertzelBin detectors (no resonant plant in the loop) as
        // a flat-response stand-in, and confirm the bank reports a
        // small peak-to-trough spread.
        //
        // An order-9 MLS sequence repeats every 511 samples, and
        // its power spectrum is flat (the well-known m-sequence
        // property) only at that sequence's own harmonic grid,
        // k * fs/511-- so this dwells exactly one full period and
        // tunes every bin to an exact harmonic of it.  Off-grid
        // bins (or a dwell that is not a whole number of periods)
        // see ordinary DFT spectral leakage instead, which is not
        // what this test means by "flat" (see
        // dsp/goertzel.rs's own tests for the same exact-integer-
        // cycles convention).
        const PERIOD: usize = 511; // 2^9 - 1
        let mut mls = MlsGen::new(9, 123, 12_000).unwrap();
        let mut excitation = [0i16; PERIOD];
        mls.fill(&mut excitation);

        let fs_hz = f64::from(PLANT_FS_HZ);
        let bin_step_hz = fs_hz / PERIOD as f64;
        let ks = [11.0, 16.0, 21.0, 27.0, 32.0]; // ~1.0-3.0 kHz on the 511-bin grid
        let bins_hz: heapless::Vec<f64, 8> = ks.iter().map(|k| k * bin_step_hz).collect();

        let mags = goertzel_bank_mags(&bins_hz, fs_hz, &excitation);

        let flatness = band_flatness_db(&mags);
        assert!(
            flatness < 6,
            "expected a near-flat spectrum, got {flatness} dB: {mags:?}"
        );
    }

    #[test]
    fn test_band_flatness_db_large_once_resonator_injected() {
        // Same broadband excitation, this time driven through the
        // synthetic Plant (Absent variant) first, so its ~1.2 kHz
        // and ~3.4 kHz resonators dominate their bins over an
        // off-resonance bin.
        let mut mls = MlsGen::new(9, 123, 12_000).unwrap();
        let mut excitation = [0i16; 8_192];
        mls.fill(&mut excitation);

        let mut plant = Plant::new(PlantVariant::Absent, 5);
        let mut resonated = excitation;
        plant.process_buf(&mut resonated);

        let bins_hz = [200.0, 1_200.0, 3_400.0];
        let mags = goertzel_bank_mags(&bins_hz, f64::from(PLANT_FS_HZ), &resonated);

        let flatness = band_flatness_db(&mags);
        assert!(
            flatness > 6,
            "expected a large peak-to-trough spread once the resonators dominate, \
             got {flatness} dB: {mags:?}"
        );
    }

    // -- thd_percent (test-only) and its synthetic PWM model -------

    /// THD estimate from a fundamental magnitude and its harmonic
    /// magnitudes: `100 * sqrt(sum(harmonic^2)) / fundamental`
    ///
    /// Test-only: mirrors `goertzel.rs`'s host-test-only f64
    /// `reference_magnitude` pattern.  `no_std` has no `sqrt` for
    /// `f64` without a new dependency, and this is a bench-analysis
    /// figure, not a value any production kernel needs at runtime.
    fn thd_percent(fundamental_mag: u32, harmonic_mags: &[u32]) -> f64 {
        let sum_sq: f64 = harmonic_mags.iter().map(|&h| f64::from(h).powi(2)).sum();
        100.0 * sum_sq.sqrt() / f64::from(fundamental_mag)
    }

    /// f64-only Goertzel magnitude, for the reference pipeline below
    /// (mirrors `goertzel.rs`'s `reference_magnitude`, but over
    /// `f64` samples instead of `i16`, since the reference pipeline
    /// never rounds down to integer samples)
    fn reference_goertzel_mag(samples: &[f64], coeff: f64) -> f64 {
        let mut s1 = 0.0_f64;
        let mut s2 = 0.0_f64;
        for &x in samples {
            let s0 = x + 2.0 * coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        (s1 * s1 + s2 * s2 - 2.0 * coeff * s1 * s2).sqrt()
    }

    #[test]
    fn test_thd_kernel_matches_synthetic_pwm_quantization_model() {
        // Model of the TIM1_CH1 PWM-audio path (pinout.md: "a 200
        // kHz carrier gives ~1000 steps, ~10-bit resolution"
        // against the 200 MHz timer kernel clock) followed by the
        // external RC low-pass (pinout.md: R6-R9/C22-C23): quantize
        // a sine tone's amplitude to ~1000 discrete steps, then
        // smooth with a one-pole lowpass.  Confirms the harmonic
        // content measured through iot-core's own fixed-point
        // kernels (Nco tone, Biquad lowpass, GoertzelBin detection)
        // matches an independent f64 reference computation of the
        // same model-- this checks the THD math itself against a
        // known-content synthetic signal, not real PWM hardware
        // (see `hil_gates.rs`'s gate G3 for the bench-hardware
        // half).
        const DWELL: usize = 4_096;
        let fs_hz = f64::from(PLANT_FS_HZ);
        let freq_hz = 1_000.0_f64;
        let amplitude = 16_000.0_f64;
        let pwm_steps = 1_000.0_f64;
        let alpha = 0.3_f64; // one-pole lowpass smoothing factor

        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;
        let step = (2.0 * amplitude) / pwm_steps;

        let harmonics_hz = [
            freq_hz,
            2.0 * freq_hz,
            3.0 * freq_hz,
            4.0 * freq_hz,
            5.0 * freq_hz,
        ];
        let bin_thetas: [f64; 5] = harmonics_hz.map(|f| 2.0 * core::f64::consts::PI * f / fs_hz);

        // -- Fixed-point pipeline: Nco tone -> PWM-step quantize ->
        // Biquad one-pole lowpass, into one sample buffer.
        let coeff_q30 = to_q30(theta.cos());
        let y1_init = (amplitude * theta.sin()).round() as i32;
        let mut nco = Nco::new(coeff_q30, y1_init, 0);
        let mut lowpass = Biquad::new(to_q30(1.0 - alpha), 0, 0, to_q30(-alpha), 0);
        let mut fixed_samples = [0i16; DWELL];
        for s in fixed_samples.iter_mut() {
            let tone = nco.next_sample() as f64;
            let quantized = (tone / step).round() * step;
            let filtered = lowpass.process(quantized.round() as i32);
            *s = filtered.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        }

        // -- f64 reference pipeline: the same quantize + one-pole
        // lowpass, computed independently in float with its own
        // phase (Goertzel magnitude is phase-insensitive, so the
        // Nco's few-sample initial-condition offset need not match
        // this reference tone's phase exactly).
        let mut ref_lp_state = 0.0_f64;
        let mut ref_samples = [0.0_f64; DWELL];
        for (n, s) in ref_samples.iter_mut().enumerate() {
            let tone = amplitude * (theta * n as f64).sin();
            let quantized = (tone / step).round() * step;
            ref_lp_state = alpha * ref_lp_state + (1.0 - alpha) * quantized;
            *s = ref_lp_state;
        }

        // -- Fundamental + 2nd-5th harmonic magnitudes, both
        // pipelines, then the THD each implies.
        let mut fixed_mags = [0u32; 5];
        let mut ref_mags = [0.0_f64; 5];
        for (i, &w) in bin_thetas.iter().enumerate() {
            let mut bin = GoertzelBin::new(to_q30(w.cos()));
            for &s in fixed_samples.iter() {
                bin.process_sample(s);
            }
            fixed_mags[i] = bin.magnitude();
            ref_mags[i] = reference_goertzel_mag(&ref_samples, w.cos());
        }

        let fixed_thd = thd_percent(fixed_mags[0], &fixed_mags[1..]);
        let ref_sum_sq: f64 = ref_mags[1..].iter().map(|m| m.powi(2)).sum();
        let ref_thd = 100.0 * ref_sum_sq.sqrt() / ref_mags[0];

        assert!(
            fixed_thd > 0.0,
            "expected nonzero THD from PWM quantization, got {fixed_thd}"
        );
        let err = (fixed_thd - ref_thd).abs();
        assert!(
            err < 1.0,
            "fixed-point THD {fixed_thd:.4}% vs f64 reference {ref_thd:.4}%, err {err:.4}pp"
        );
    }
}
