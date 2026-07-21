//! HIL (hardware-in-the-loop) bench-acceptance gates for the ARS
//! toolhead audio path
//!
//! Each `#[ignore]`d test here corresponds to one spike gate from
//! `docs/src/projects/ars-toolhead-sensor/pinout.md`'s "Spike
//! Gates" section-- criteria that cannot be evaluated from pure
//! logic alone because they need a real bench capture (a Saleae
//! Logic MSO 2x100 export, per
//! `docs/src/projects/ars-toolhead-sensor/hil-measurements.md`), a
//! numeric acceptance threshold the source docs do not yet state,
//! or both.  They are real `#[test]` functions, not deleted or
//! commented out, so a missing prerequisite shows up as a visible
//! ignored-count warning in `cargo test` output rather than
//! silently disappearing; run with `cargo test -- --ignored` once a
//! gate's prerequisites land, and drop its `#[ignore]` at that
//! point.  This `#[ignore]` usage is user-approved (RED policy,
//! 2026-07-21)-- an exception to `testing_gates.md`'s general "do
//! not use `#[ignore]` to bypass failures" rule.
//!
//! Separate from the golden-vector corpus (`golden.rs`): golden
//! covers bit-exact kernel replay against a generated fixture
//! corpus, while this file covers bench-acceptance criteria against
//! real hardware captures.

#![cfg(feature = "ars")]
#![deny(warnings)]
#![deny(unsafe_code)]

/// Gate G2 (`pinout.md` Spike Gates)-- A0 adaptation-amplifier
/// mic-path capture quality: noise floor and flatness across the
/// sweep band, measured from a real bench mic capture rather than
/// the synthetic `Plant` (see `crate::ars::quality`'s passable-now
/// coverage for the synthetic-plant half of this gate).
///
/// Blocked on two prerequisites: (1) a bench mic capture fixture (a
/// Saleae analog export, per `hil-measurements.md`) checked in
/// under `core/tests/`, and (2) a numeric dB acceptance threshold--
/// G2's overturn criterion in `pinout.md` is qualitative ("if the
/// on-board stage degrades the audio band"), with no dB figure
/// stated anywhere in the docs, so this test cannot invent one
/// without EE sign-off.
#[test]
#[ignore = "RED: gate G2 -- needs bench mic capture fixture (Saleae analog export) and an \
            EE-specified dB threshold; pinout.md's G2 overturn criterion ('degrades the audio \
            band') is qualitative, no number is stated anywhere in the docs"]
fn test_g2_real_mic_capture_meets_flatness_and_noise_floor_bound() {
    todo!(
        "RED: gate G2 -- once a bench mic capture fixture exists, load it, run it through \
         crate::ars::quality::noise_floor_q15 and crate::ars::quality::band_flatness_db, and \
         assert against an EE-specified dB threshold once one is committed to the docs"
    );
}

/// Gate G3 (`pinout.md` Spike Gates)-- PWM-audio fidelity: SNR/THD
/// of the TIM1_CH1 PWM path plus RC filter into the MAX9744,
/// measured against sweep-analysis requirements (see
/// `crate::ars::quality`'s passable-now coverage for the THD math
/// itself, validated against a synthetic PWM-quantization model
/// rather than a real capture).
///
/// Blocked on two prerequisites: (1) Saleae PWM-carrier-residue and
/// loopback-sine capture fixtures per `hil-measurements.md`, and
/// (2) a committed numeric SNR/THD target.  No target is stated for
/// the actual TIM1_CH1-plus-RC path anywhere in the docs-- the only
/// sourced SNR/THD figures (112 dB SNR / -93 dB THD+N,
/// `spike-audio-board.md`) describe the Adafruit PCM5102 fallback
/// DAC candidate, not an acceptance bar for the currently
/// provisional PWM path, and must not be substituted without EE
/// sign-off.
#[test]
#[ignore = "RED: gate G3 -- needs Saleae PWM-carrier-residue + loopback-sine captures per \
            hil-measurements.md, and a committed numeric SNR/THD target.  No target is stated \
            for the actual TIM1_CH1+RC path anywhere in the docs -- the only sourced SNR/THD \
            figures (112 dB SNR / -93 dB THD+N in spike-audio-board.md) describe the PCM5102 \
            fallback DAC candidate, not an acceptance bar for the current PWM path, and must \
            not be substituted without EE sign-off"]
fn test_g3_bench_pwm_path_meets_snr_thd_targets() {
    todo!(
        "RED: gate G3 -- once PWM-carrier-residue and loopback-sine capture fixtures exist, \
         load them, run them through crate::ars::quality's THD/SNR analysis, and assert \
         against an EE-specified SNR/THD target once one is committed to the docs"
    );
}

/// Gate G5 (`pinout.md` Spike Gates)-- end-to-end sweep excursion
/// safety: drive levels at bench-derived band limits must keep the
/// Dayton Audio EX25VT2-4 exciter within safe excursion (see
/// `crate::ars::types::BinPlan::from_descriptor_within_safe_band`
/// for the passable-now sweep-*construction* guard rail this gate
/// builds on-- that only bounds which frequencies a sweep can
/// request, not how hard the exciter is driven).
///
/// Likely permanently blocked, not just pending a fixture:
/// `hardware.md` confirms the vendor supplies no `Xmax` or
/// usable-range figure at all for the EX25VT2-4, so this is a
/// physical/mechanical safety measurement that pure logic can never
/// assert on in principle-- there is no vendor constant to fall
/// back to.  Expect this test to remain `#[ignore]`d even after G2
/// and G3 clear, until a bench-measured excursion limit is manually
/// captured and hand-entered as a new constant.
#[test]
#[ignore = "RED: gate G5 -- needs a bench-measured EX25VT2-4 excursion limit; hardware.md \
            confirms the vendor supplies no Xmax or usable-range figure at all, so this is a \
            physical/mechanical safety measurement, not a value core can ever assert on from \
            pure logic -- expect this test to stay #[ignore] even after other RED gates clear, \
            until a measured constant is hand-entered"]
fn test_g5_end_to_end_sweep_stays_within_measured_excursion_limit() {
    todo!(
        "RED: gate G5 -- once a bench-measured EX25VT2-4 excursion limit is hand-entered as a \
         constant, assert that every drive level an end-to-end sweep can produce (per \
         crate::ars::types::BinPlan::from_descriptor_within_safe_band's frequency band and \
         crate::ars::synth::MAX_SAFE_LEVEL_Q15's level bound) stays under it"
    );
}
