//! Synthetic toolhead plant model
//!
//! A plausible stand-in for a real toolhead's acoustic resonance
//! response, used to unblock downstream pipeline work before
//! hardware captures exist.  The model is two parallel resonant
//! biquads (host-designed Q2.30 coefficients approximating
//! plausible toolhead resonances at roughly 1.2 kHz and 3.4 kHz)
//! plus a seeded noise floor at roughly -50 dBFS.  The
//! filament-present variant shifts both resonator centers down 4%
//! and raises their Q (moves the poles closer to the unit circle).
//!
//! Every generator here is a pure function of its constructor
//! arguments (fixed coefficients plus a caller-supplied noise
//! seed), so synthetic captures are exactly reproducible.
//!
//! # Fixed sample rate assumption
//!
//! The resonator coefficients below are designed for a single,
//! fixed assumed sample rate ([`PLANT_FS_HZ`], 48 kHz)-- deriving
//! coefficients for an arbitrary runtime `fs_hz` would require
//! runtime cosine, which `no_std` does not have (see the crate's
//! "coefficients are host-computed data" policy).  If the real
//! bench rig captures at a different rate, this constant-- and the
//! coefficients derived from it-- need updating together.
//!
//! # Excitation level and resonance clipping
//!
//! Each resonator has significant gain at its own center: roughly
//! 3.21x (absent variant) and, worse, 3.33x (present variant) at
//! the R1 center (see [`MAX_SAFE_LEVEL_Q15`]).  A sine or
//! stepped-sine dwell held at or near a resonance center at a
//! `level_q15` above that constant will saturate the `i16` output
//! in [`Plant::process`], clipping exactly in the frequency region
//! downstream classifiers care about most.  Callers building
//! excitation descriptors for resonance-centered dwells should keep
//! `level_q15` at or below [`MAX_SAFE_LEVEL_Q15`].

use crate::dsp::biquad::Biquad;
use crate::dsp::xoshiro128::Xoshiro128PlusPlus;

/// Sample rate the plant model's resonator coefficients were
/// designed for
///
/// See the module-level docs' "Fixed sample rate assumption".
pub const PLANT_FS_HZ: u32 = 48_000;

/// Noise floor peak amplitude, Q15 counts, approximating -50 dBFS
/// relative to full scale (`32767 * 10^(-50/20) ~= 103.6`)
pub const NOISE_FLOOR_Q15: i16 = 104;

/// Maximum recommended excitation level (`level_q15` in an
/// `ExcitationDescriptor`) for a dwell held at or near a resonance
/// center, without hitting the output clamp in [`Plant::process`]
///
/// The parallel resonators peak at combined gains of roughly 3.21x
/// (absent variant) and 3.33x (present variant, the worst case) at
/// their R1 centers (~1.2 kHz / ~1.152 kHz).  Naively dividing
/// full-scale headroom by that peak gain suggests a level near
/// 9,800; a resonance dwell's transient overshoots the ideal
/// steady-state gain slightly before settling, so this constant is
/// set with margin below that naive bound and verified empirically
/// (see `test_resonance_dwell_at_max_safe_level_does_not_clip`).
pub const MAX_SAFE_LEVEL_Q15: i16 = 9_500;

/// Whether filament is present in the synthetic plant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PlantVariant {
    /// No filament loaded-- resonators at their bare-toolhead
    /// centers and Q
    Absent,
    /// Filament loaded-- resonator centers shifted down 4% and Q
    /// raised
    Present,
}

/// Q2.30 biquad coefficients for one resonator, `(b0, b1, b2, a1, a2)`
type Coeffs = (i32, i32, i32, i32, i32);

/// Resonator 1 (~1.2 kHz), absent variant
const RESONATOR_1_ABSENT: Coeffs = (5_368_709, 0, 0, -2_110_439_338, 1_063_031_249);
/// Resonator 2 (~3.4 kHz), absent variant
const RESONATOR_2_ABSENT: Coeffs = (7_516_193, 0, 0, -1_924_719_129, 1_058_762_052);
/// Resonator 1 (~1.152 kHz, -4% shifted), present variant, raised Q
const RESONATOR_1_PRESENT: Coeffs = (2_147_484, 0, 0, -2_118_867_228, 1_069_451_152);
/// Resonator 2 (~3.264 kHz, -4% shifted), present variant, raised Q
const RESONATOR_2_PRESENT: Coeffs = (3_221_225, 0, 0, -1_948_574_377, 1_067_309_037);

fn resonator_coeffs(variant: PlantVariant) -> [Coeffs; 2] {
    match variant {
        PlantVariant::Absent => [RESONATOR_1_ABSENT, RESONATOR_2_ABSENT],
        PlantVariant::Present => [RESONATOR_1_PRESENT, RESONATOR_2_PRESENT],
    }
}

/// The synthetic plant: two parallel resonant biquads plus a
/// seeded noise floor
#[derive(Debug, Clone, Copy)]
pub struct Plant {
    resonators: [Biquad; 2],
    noise: Xoshiro128PlusPlus,
}

impl Plant {
    /// Build a plant for the given `variant`, seeded for
    /// reproducible noise
    pub fn new(variant: PlantVariant, seed: u32) -> Self {
        let coeffs = resonator_coeffs(variant);
        Self {
            resonators: [
                Biquad::new(
                    coeffs[0].0,
                    coeffs[0].1,
                    coeffs[0].2,
                    coeffs[0].3,
                    coeffs[0].4,
                ),
                Biquad::new(
                    coeffs[1].0,
                    coeffs[1].1,
                    coeffs[1].2,
                    coeffs[1].3,
                    coeffs[1].4,
                ),
            ],
            noise: Xoshiro128PlusPlus::new(seed),
        }
    }

    /// Process one excitation sample through the parallel
    /// resonators, add the noise floor, and saturate to `i16`
    ///
    /// A resonance-centered dwell driven above
    /// [`MAX_SAFE_LEVEL_Q15`] will hit this saturation-- see the
    /// module-level "Excitation level and resonance clipping" docs.
    pub fn process(&mut self, excitation: i16) -> i16 {
        let x = excitation as i32;
        let r1 = self.resonators[0].process(x);
        let r2 = self.resonators[1].process(x);
        let noise = self.noise.next_i16_bounded(NOISE_FLOOR_Q15) as i32;
        let sum = r1 as i64 + r2 as i64 + noise as i64;
        sum.clamp(i16::MIN as i64, i16::MAX as i64) as i16
    }

    /// Process a whole excitation buffer in place
    pub fn process_buf(&mut self, buf: &mut [i16]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_in_yields_only_noise_floor() {
        let mut plant = Plant::new(PlantVariant::Absent, 1);
        for _ in 0..200 {
            let out = plant.process(0);
            assert!((-NOISE_FLOOR_Q15..=NOISE_FLOOR_Q15).contains(&out));
        }
    }

    #[test]
    fn test_deterministic_for_same_seed() {
        let mut a = Plant::new(PlantVariant::Absent, 42);
        let mut b = Plant::new(PlantVariant::Absent, 42);
        let excitation: [i16; 16] = [
            1000, -500, 2000, 0, 500, -1000, 1500, -2000, 100, 200, 300, -400, 0, 0, 0, 0,
        ];
        for &x in excitation.iter() {
            assert_eq!(a.process(x), b.process(x));
        }
    }

    #[test]
    fn test_different_seeds_diverge_in_noise() {
        // Drive with silence so the only difference between runs
        // is the noise floor.
        let mut a = Plant::new(PlantVariant::Absent, 1);
        let mut b = Plant::new(PlantVariant::Absent, 2);
        let seq_a: heapless::Vec<i16, 32> = (0..32).map(|_| a.process(0)).collect();
        let seq_b: heapless::Vec<i16, 32> = (0..32).map(|_| b.process(0)).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn test_present_variant_resonates_more_than_absent_at_shifted_center() {
        // Drive both variants with an impulse and confirm the
        // present variant (higher Q, closer pole) has not fully
        // decayed to silence by the time the absent variant's
        // response has settled near zero.
        let mut absent = Plant::new(PlantVariant::Absent, 7);
        let mut present = Plant::new(PlantVariant::Present, 7);

        let mut absent_energy = 0i64;
        let mut present_energy = 0i64;
        for n in 0..2000 {
            let x = if n == 0 { i16::MAX } else { 0 };
            let a_out = absent.process(x) as i64;
            let p_out = present.process(x) as i64;
            if n > 1500 {
                absent_energy += a_out * a_out;
                present_energy += p_out * p_out;
            }
        }
        assert!(
            present_energy > absent_energy,
            "present variant should still be ringing after the absent variant has decayed: \
             present={present_energy}, absent={absent_energy}"
        );
    }

    #[test]
    fn test_resonance_dwell_at_max_safe_level_does_not_clip() {
        // Drive each plant variant with a sine dwell at its own R1
        // resonance center (see the module docs' "Excitation level
        // and resonance clipping"), at MAX_SAFE_LEVEL_Q15, long
        // enough to reach and pass through steady state, and
        // confirm no sample hits the i16 clamp in `process`.
        let cases = [
            (PlantVariant::Absent, 1_200.0_f64),
            (PlantVariant::Present, 1_152.0_f64),
        ];
        for (variant, freq_hz) in cases {
            let mut plant = Plant::new(variant, 3);
            for n in 0..6_000u32 {
                let phase =
                    2.0 * core::f64::consts::PI * freq_hz * f64::from(n) / f64::from(PLANT_FS_HZ);
                let x = (f64::from(MAX_SAFE_LEVEL_Q15) * phase.sin()).round() as i16;
                let out = plant.process(x);
                assert!(
                    out.unsigned_abs() < i16::MAX as u16,
                    "{variant:?} clipped at sample {n}: out={out}"
                );
            }
        }
    }

    #[test]
    fn test_process_buf_matches_process_sample_by_sample() {
        let mut a = Plant::new(PlantVariant::Absent, 9);
        let mut b = Plant::new(PlantVariant::Absent, 9);
        let mut buf = [1000i16, 0, -500, 2000, 0, 0, 0, 0];
        let expected: heapless::Vec<i16, 8> = buf.iter().map(|&x| a.process(x)).collect();
        b.process_buf(&mut buf);
        assert_eq!(&buf[..], &expected[..]);
    }
}
