//! Fixture definitions: f64 signal design -> integer fixtures
//!
//! This is the ONLY place floating point is used.  Each builder
//! designs coefficients, initial conditions, phases, or input
//! samples in f64, rounds them to the fixed-point integers the
//! on-device kernels consume, and stores those integers in the
//! fixture.  The expected outputs are then computed by
//! `driver::compute_expected`, which drives `iot-core`'s kernels
//! with the stored integers only-- so the checked-in corpus never
//! depends on this module's f64 math at replay time.
//!
//! Excitation families and analysis features follow section 8 of
//! `docs/src/projects/ars-toolhead-sensor/research-prior-art.md`:
//! MLS (orders 9-11) as the primary excitation, a Schroeder-phase
//! multitone at the candidate bins plus flanking sentinels, and a
//! Farina exponential sine sweep, analyzed with Goertzel bin
//! magnitudes.

use std::f64::consts::PI;

use crate::driver::{compute_expected, Fixture};

/// Sample rate every fixture is designed for, matching
/// `iot_core::ars::synth::PLANT_FS_HZ`
const FS_HZ: f64 = 48_000.0;

fn to_q30(x: f64) -> i64 {
    (x * f64::from(1i32 << 30)).round() as i64
}

fn theta(f_hz: f64) -> f64 {
    2.0 * PI * f_hz / FS_HZ
}

/// Q1.30 Goertzel/NCO cosine coefficient for a tone at `f_hz`
fn cos_coeff_q30(f_hz: f64) -> i64 {
    to_q30(theta(f_hz).cos())
}

/// NCO initial conditions so the first output sample is
/// `amp * sin(phase)`: `y[-1] = amp*sin(phase - theta)` and
/// `y[-2] = amp*sin(phase - 2*theta)`
fn tone_inits(f_hz: f64, amp: f64, phase: f64) -> (i64, i64) {
    let th = theta(f_hz);
    (
        (amp * (phase - th).sin()).round() as i64,
        (amp * (phase - 2.0 * th).sin()).round() as i64,
    )
}

fn param(key: &str, value: impl ToString) -> (String, String) {
    (key.to_string(), value.to_string())
}

fn section(name: &str, values: Vec<i64>) -> (String, Vec<i64>) {
    (name.to_string(), values)
}

fn header(comments: &[&str], params: Vec<(String, String)>) -> Fixture {
    let mut all = vec![
        "Golden fixture for the ARS fixed-point DSP pipeline.".to_string(),
        "Regenerate: see tools/ars-synth/AGENTS.md; format: core/tests/golden/README.md"
            .to_string(),
    ];
    all.extend(comments.iter().map(|c| c.to_string()));
    let mut full_params = vec![param("schema", "ars-golden-v1")];
    full_params.extend(params);
    Fixture {
        comments: all,
        params: full_params,
        sections: Vec::new(),
    }
}

fn mls(order: u8, seed: u32, level_q15: i16, count: usize) -> Fixture {
    header(
        &["MLS excitation (LFSR, XAPP052 taps): raw sample stream + descriptor."],
        vec![
            param("kind", "mls"),
            param("fs_hz", 48_000),
            param("order", order),
            param("seed", seed),
            param("level_q15", level_q15),
            param("count", count),
        ],
    )
}

fn nco_sine() -> Fixture {
    let freq_hz = 1_200.0;
    let amp = 10_000.0;
    let (y1, y2) = tone_inits(freq_hz, amp, 0.0);
    header(
        &["NCO resonator sine, 1200.0 Hz at 48 kHz, amplitude 10000 Q15 counts."],
        vec![
            param("kind", "nco"),
            param("fs_hz", 48_000),
            param("freq_dhz", 12_000),
            param("nco_coeff_q30", cos_coeff_q30(freq_hz)),
            param("nco_y1_init", y1),
            param("nco_y2_init", y2),
            param("count", 256),
        ],
    )
}

/// Two-pole resonator coefficients (no zeros) at `f_hz` with pole
/// radius `r`, mirroring the topology used by `ars::synth::Plant`
fn resonator_coeffs(f_hz: f64, r: f64) -> (i64, i64, i64, i64, i64) {
    let th = theta(f_hz);
    (
        to_q30(1.0 - r),
        0,
        0,
        to_q30(-2.0 * r * th.cos()),
        to_q30(r * r),
    )
}

fn biquad(comment: &str, f_hz: f64, r: f64, input: &str, input_level: i32) -> Fixture {
    let (b0, b1, b2, a1, a2) = resonator_coeffs(f_hz, r);
    header(
        &[comment],
        vec![
            param("kind", "biquad"),
            param("fs_hz", 48_000),
            param("b0_q30", b0),
            param("b1_q30", b1),
            param("b2_q30", b2),
            param("a1_q30", a1),
            param("a2_q30", a2),
            param("input", input),
            param("input_level", input_level),
            param("count", 256),
        ],
    )
}

/// Schroeder-phase multitone at the candidate resonance bins
/// (1200/3400 Hz) plus flanking sentinel bins (1150/3450 Hz), with
/// Goertzel analysis at all four tone bins plus one off-bin
fn multitone() -> Fixture {
    let count = 4_800usize; // 0.1 s at 48 kHz -> 10 Hz bin spacing
    let tone_freqs = [1_150.0, 1_200.0, 3_400.0, 3_450.0];
    let amp = 6_000.0;
    let k_total = tone_freqs.len() as f64;

    let mut freqs_dhz = Vec::new();
    let mut phases_urad = Vec::new();
    let mut coeffs = Vec::new();
    let mut y1s = Vec::new();
    let mut y2s = Vec::new();
    for (i, &f) in tone_freqs.iter().enumerate() {
        // Schroeder phases: phi_k = -pi * k * (k - 1) / K, k = 1..K.
        let k = (i + 1) as f64;
        let phase = -PI * k * (k - 1.0) / k_total;
        let (y1, y2) = tone_inits(f, amp, phase);
        freqs_dhz.push((f * 10.0).round() as i64);
        phases_urad.push((phase * 1.0e6).round() as i64);
        coeffs.push(cos_coeff_q30(f));
        y1s.push(y1);
        y2s.push(y2);
    }

    let goertzel_freqs = [1_150.0, 1_200.0, 3_400.0, 3_450.0, 2_300.0];
    let mut fixture = header(
        &[
            "Schroeder-phase multitone: candidate bins 1200/3400 Hz plus",
            "flanking sentinels 1150/3450 Hz; Goertzel at all four tone",
            "bins plus a 2300 Hz off-bin.  Integer-period tones over the",
            "4800-sample dwell (10 Hz bin spacing) for leakage-free bins.",
        ],
        vec![
            param("kind", "multitone"),
            param("fs_hz", 48_000),
            param("tone_amp", 6_000),
            param("count", count),
        ],
    );
    fixture.sections = vec![
        section("tone_freq_dhz", freqs_dhz),
        section("tone_phase_urad", phases_urad),
        section("tone_coeff_q30", coeffs),
        section("tone_y1_init", y1s),
        section("tone_y2_init", y2s),
        section(
            "goertzel_coeff_q30",
            goertzel_freqs.iter().map(|&f| cos_coeff_q30(f)).collect(),
        ),
    ];
    fixture
}

/// A single slightly off-grid 1203 Hz tone analyzed by three
/// straddling Goertzel bins (1190/1200/1210 Hz) plus a distant
/// off-bin-- the raw material for the interpolated-peak (parabolic)
/// feature of record.  Off-grid on purpose, so every straddle bin
/// carries a non-trivial magnitude
fn goertzel_3bin() -> Fixture {
    let freq_hz = 1_203.0;
    let amp = 10_000.0;
    let (y1, y2) = tone_inits(freq_hz, amp, 0.0);
    let goertzel_freqs = [1_190.0, 1_200.0, 1_210.0, 2_400.0];
    let mut fixture = header(
        &[
            "Goertzel 3-bin peak straddle: NCO tone at 1203 Hz (off-grid",
            "on purpose), detectors at 1190/1200/1210 Hz (peak straddle",
            "for parabolic interpolation) plus a 2400 Hz off-bin.",
        ],
        vec![
            param("kind", "goertzel_tone"),
            param("fs_hz", 48_000),
            param("freq_dhz", 12_030),
            param("nco_coeff_q30", cos_coeff_q30(freq_hz)),
            param("nco_y1_init", y1),
            param("nco_y2_init", y2),
            param("count", 4_800),
        ],
    );
    fixture.sections = vec![
        section("goertzel_freq_dhz", vec![11_900, 12_000, 12_100, 24_000]),
        section(
            "goertzel_coeff_q30",
            goertzel_freqs.iter().map(|&f| cos_coeff_q30(f)).collect(),
        ),
    ];
    fixture
}

/// Farina exponential sine sweep samples, `level * sin(K * (exp(n *
/// ln(f2/f1) / n_total) - 1))` with `K = w1 * T / ln(f2/f1)`
fn ess_samples(f1_hz: f64, f2_hz: f64, count: usize, level: f64) -> Vec<i64> {
    let ratio_ln = (f2_hz / f1_hz).ln();
    let t_total = count as f64 / FS_HZ;
    let k = 2.0 * PI * f1_hz * t_total / ratio_ln;
    (0..count)
        .map(|n| {
            let t = n as f64 / FS_HZ;
            let phase = k * ((t * ratio_ln / t_total).exp() - 1.0);
            (level * phase.sin()).round() as i64
        })
        .collect()
}

/// Farina ESS (500 Hz -> 6 kHz) driven through the synthetic plant
/// (absent variant), with Goertzel analysis at the two resonance
/// centers
fn ess_plant() -> Fixture {
    let count = 4_800usize;
    let mut fixture = header(
        &[
            "Farina exponential sine sweep, 500 Hz -> 6 kHz over 4800",
            "samples, level 9000 Q15 counts (below MAX_SAFE_LEVEL_Q15),",
            "through Plant(Absent, seed 77); Goertzel at the plant's",
            "1200/3400 Hz resonance centers.  The sweep samples are",
            "stored literally so replay never needs trigonometry.",
        ],
        vec![
            param("kind", "ess_plant"),
            param("fs_hz", 48_000),
            param("f1_dhz", 5_000),
            param("f2_dhz", 60_000),
            param("level_q15", 9_000),
            param("plant_variant", 0),
            param("plant_seed", 77),
            param("count", count),
        ],
    );
    fixture.sections = vec![
        section(
            "input_samples_i16",
            ess_samples(500.0, 6_000.0, count, 9_000.0),
        ),
        section("goertzel_freq_dhz", vec![12_000, 34_000]),
        section(
            "goertzel_coeff_q30",
            vec![cos_coeff_q30(1_200.0), cos_coeff_q30(3_400.0)],
        ),
    ];
    fixture
}

/// Full record images: MLS order 9 through Plant(Present) into a
/// raw-payload `CAPTURE RECORD v1` plus a matching `LABEL RECORD
/// v1`, CRC trailers included
fn capture() -> Fixture {
    header(
        &[
            "Full ARSC capture + ARSL label record images (bytes, CRC",
            "included): MLS order 9 excitation through Plant(Present,",
            "seed 99), raw i16 payload.  Timestamps are fixed constants",
            "(never wall-clock).",
        ],
        vec![
            param("kind", "capture"),
            param("fs_hz", 48_000),
            param("exc_order", 9),
            param("exc_seed", 293),
            param("exc_level_q15", 8_000),
            param("count", 511),
            param("plant_variant", 1),
            param("plant_seed", 99),
            param("bench_id", 1),
            param("mic_id", 1),
            param("channels", 1),
            param("boot_id", 0xC0FF_EE01u32),
            param("capture_seq", 7),
            param("monotonic_us", 1_000_000),
            param("unix_micros", 1_750_000_000_000_000u64),
            param("label_hot_end", 1),
            param("label_cold_end", 0),
            param("label_unix_micros", 1_750_000_005_000_000u64),
            param("label_confidence_q15", 30_000),
            param("label_source", 0),
        ],
    )
}

/// Build the complete corpus: every fixture, with its `expected_*`
/// sections computed by the shared kernel-replay driver
pub fn build_all() -> Vec<(&'static str, Fixture)> {
    let mut fixtures = vec![
        ("mls_order09.txt", mls(9, 293, 16_000, 511)),
        ("mls_order10.txt", mls(10, 613, 16_000, 1_023)),
        ("mls_order11.txt", mls(11, 1_201, 16_000, 2_047)),
        ("nco_sine_1200hz.txt", nco_sine()),
        (
            "biquad_impulse_1200hz.txt",
            biquad(
                "Resonator biquad (1200 Hz, r = 0.995) impulse response.",
                1_200.0,
                0.995,
                "impulse",
                32_767,
            ),
        ),
        (
            "biquad_step_3400hz.txt",
            biquad(
                "Resonator biquad (3400 Hz, r = 0.990) step response.",
                3_400.0,
                0.990,
                "step",
                16_384,
            ),
        ),
        ("multitone_schroeder.txt", multitone()),
        ("goertzel_3bin_1203hz.txt", goertzel_3bin()),
        ("ess_sweep_plant.txt", ess_plant()),
        ("capture_mls_plant.txt", capture()),
    ];
    for (name, fixture) in &mut fixtures {
        let expected = compute_expected(fixture)
            .unwrap_or_else(|e| panic!("fixture {name}: expected-output replay failed: {e}"));
        fixture.sections.extend(expected);
    }
    fixtures
}
