//! Shared golden-corpus fixture driver
//!
//! Included via `#[path]` by BOTH `core/tests/golden.rs` (the
//! regression test) and `tools/ars-synth` (the corpus generator and
//! checker), so the code that parses/renders fixture files and
//! drives `iot-core`'s fixed-point kernels is one module, not two
//! copies that could drift apart.
//!
//! Everything in this module is integer-only: fixture files carry
//! every fixed-point coefficient and initial condition as decimal
//! integers, so replaying a fixture needs no floating point and no
//! trigonometry.  The f64 signal *design* (choosing coefficients,
//! Schroeder phases, the ESS sweep) lives only in
//! `tools/ars-synth/src/fixtures.rs`.
//!
//! See `core/tests/golden/README.md` for the file format.

use iot_core::ars::generators::MlsGen;
use iot_core::ars::record::{
    CaptureHeader, CaptureRecord, CaptureView, EndPresence, LabelRecord, LabelSource, Payload,
};
use iot_core::ars::synth::{Plant, PlantVariant};
use iot_core::dsp::goertzel::GoertzelBin;
use iot_core::dsp::nco::Nco;

/// One parsed (or in-construction) golden fixture
///
/// A fixture is a leading comment block, ordered `key = value`
/// parameters, and ordered `[name]` sections of decimal integers.
/// Sections whose names start with `expected_` are outputs of
/// [`compute_expected`]; all other sections are inputs or
/// informational.
pub struct Fixture {
    /// Leading `#` comment lines, without the `# ` prefix
    pub comments: Vec<String>,
    /// Ordered `key = value` parameters (before the first section)
    pub params: Vec<(String, String)>,
    /// Ordered `[name]` sections of decimal integers
    pub sections: Vec<(String, Vec<i64>)>,
}

impl Fixture {
    /// Parse a fixture from its text form
    ///
    /// Comment (`#`) and blank lines are skipped; `key = value`
    /// lines are only accepted before the first `[section]` header.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut fixture = Self {
            comments: Vec::new(),
            params: Vec::new(),
            sections: Vec::new(),
        };
        for (idx, raw) in text.lines().enumerate() {
            let lineno = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                fixture.sections.push((name.to_string(), Vec::new()));
            } else if let Some((_, values)) = fixture.sections.last_mut() {
                for tok in line.split_whitespace() {
                    let v: i64 = tok
                        .parse()
                        .map_err(|_| format!("line {lineno}: bad integer {tok:?}"))?;
                    values.push(v);
                }
            } else {
                let (key, value) = line
                    .split_once('=')
                    .ok_or_else(|| format!("line {lineno}: expected `key = value`"))?;
                fixture
                    .params
                    .push((key.trim().to_string(), value.trim().to_string()));
            }
        }
        Ok(fixture)
    }

    /// Render the fixture to its canonical text form
    ///
    /// `gen` writes exactly this text and `check` compares against
    /// it byte-for-byte, so the rendering must stay deterministic.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for c in &self.comments {
            out.push_str("# ");
            out.push_str(c);
            out.push('\n');
        }
        for (key, value) in &self.params {
            out.push_str(key);
            out.push_str(" = ");
            out.push_str(value);
            out.push('\n');
        }
        for (name, values) in &self.sections {
            out.push('\n');
            out.push('[');
            out.push_str(name);
            out.push_str("]\n");
            render_ints(&mut out, values);
        }
        out
    }

    /// Look up a parameter's raw string value
    pub fn param_str(&self, key: &str) -> Result<&str, String> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .ok_or_else(|| format!("missing param {key:?}"))
    }

    /// Look up a parameter and parse it as a decimal integer
    pub fn param_i64(&self, key: &str) -> Result<i64, String> {
        let v = self.param_str(key)?;
        v.parse()
            .map_err(|_| format!("param {key:?}: bad integer {v:?}"))
    }

    /// Look up a section's integer array
    pub fn section(&self, name: &str) -> Result<&[i64], String> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
            .ok_or_else(|| format!("missing section [{name}]"))
    }
}

/// Append `values` to `out` as space-separated decimal integers,
/// wrapped before 80 columns
fn render_ints(out: &mut String, values: &[i64]) {
    let mut line = String::new();
    for v in values {
        let tok = v.to_string();
        if line.is_empty() {
            line = tok;
        } else if line.len() + 1 + tok.len() > 76 {
            out.push_str(&line);
            out.push('\n');
            line = tok;
        } else {
            line.push(' ');
            line.push_str(&tok);
        }
    }
    if !line.is_empty() {
        out.push_str(&line);
        out.push('\n');
    }
}

fn to_u8(v: i64, what: &str) -> Result<u8, String> {
    u8::try_from(v).map_err(|_| format!("{what}: {v} out of u8 range"))
}

fn to_u16(v: i64, what: &str) -> Result<u16, String> {
    u16::try_from(v).map_err(|_| format!("{what}: {v} out of u16 range"))
}

fn to_i16(v: i64, what: &str) -> Result<i16, String> {
    i16::try_from(v).map_err(|_| format!("{what}: {v} out of i16 range"))
}

fn to_i32(v: i64, what: &str) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("{what}: {v} out of i32 range"))
}

fn to_u32(v: i64, what: &str) -> Result<u32, String> {
    u32::try_from(v).map_err(|_| format!("{what}: {v} out of u32 range"))
}

fn to_u64(v: i64, what: &str) -> Result<u64, String> {
    u64::try_from(v).map_err(|_| format!("{what}: {v} out of u64 range"))
}

fn to_count(v: i64, what: &str) -> Result<usize, String> {
    usize::try_from(v).map_err(|_| format!("{what}: {v} out of usize range"))
}

/// Feed `samples` through one [`GoertzelBin`] per Q1.30 cosine
/// coefficient in `coeffs` and return the magnitudes
fn goertzel_mags(samples: &[i16], coeffs: &[i64]) -> Result<Vec<i64>, String> {
    let mut out = Vec::with_capacity(coeffs.len());
    for &c in coeffs {
        let mut bin = GoertzelBin::new(to_i32(c, "goertzel coeff_q30")?);
        for &s in samples {
            bin.process_sample(s);
        }
        out.push(i64::from(bin.magnitude()));
    }
    Ok(out)
}

fn plant_variant(v: i64) -> Result<PlantVariant, String> {
    match v {
        0 => Ok(PlantVariant::Absent),
        1 => Ok(PlantVariant::Present),
        _ => Err(format!(
            "plant_variant: {v} is not 0 (absent) or 1 (present)"
        )),
    }
}

/// Recompute every `expected_*` section of `fixture` by driving
/// `iot-core`'s fixed-point kernels with the fixture's integer
/// parameters and input sections
///
/// Returns the expected sections in canonical order for the
/// fixture's `kind`.  The generator appends these verbatim; the
/// regression test compares them against what is stored on disk.
pub fn compute_expected(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    match fixture.param_str("kind")? {
        "mls" => compute_mls(fixture),
        "nco" => compute_nco(fixture),
        "biquad" => compute_biquad(fixture),
        "multitone" => compute_multitone(fixture),
        "goertzel_tone" => compute_goertzel_tone(fixture),
        "ess_plant" => compute_ess_plant(fixture),
        "capture" => compute_capture(fixture),
        other => Err(format!("unknown fixture kind {other:?}")),
    }
}

fn compute_mls(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let order = to_u8(fixture.param_i64("order")?, "order")?;
    let seed = to_u32(fixture.param_i64("seed")?, "seed")?;
    let level = to_i16(fixture.param_i64("level_q15")?, "level_q15")?;
    let count = to_count(fixture.param_i64("count")?, "count")?;

    let mut generator = MlsGen::new(order, seed, level)
        .ok_or_else(|| format!("no curated taps for order {order}"))?;
    let mut samples = vec![0i16; count];
    generator.fill(&mut samples);
    let d = generator.descriptor();

    Ok(vec![
        (
            "expected_descriptor".to_string(),
            vec![
                i64::from(d.kind.as_u8()),
                i64::from(d.steps_or_order),
                i64::from(d.seed),
                i64::from(d.dwell),
                i64::from(d.level_q15),
            ],
        ),
        (
            "expected_samples_i16".to_string(),
            samples.iter().map(|&s| i64::from(s)).collect(),
        ),
    ])
}

fn nco_from_params(fixture: &Fixture) -> Result<Nco, String> {
    Ok(Nco::new(
        to_i32(fixture.param_i64("nco_coeff_q30")?, "nco_coeff_q30")?,
        to_i32(fixture.param_i64("nco_y1_init")?, "nco_y1_init")?,
        to_i32(fixture.param_i64("nco_y2_init")?, "nco_y2_init")?,
    ))
}

fn compute_nco(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let count = to_count(fixture.param_i64("count")?, "count")?;
    let mut nco = nco_from_params(fixture)?;
    let samples: Vec<i64> = (0..count).map(|_| i64::from(nco.next_i16())).collect();
    Ok(vec![("expected_samples_i16".to_string(), samples)])
}

fn compute_biquad(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let mut biquad = iot_core::dsp::biquad::Biquad::new(
        to_i32(fixture.param_i64("b0_q30")?, "b0_q30")?,
        to_i32(fixture.param_i64("b1_q30")?, "b1_q30")?,
        to_i32(fixture.param_i64("b2_q30")?, "b2_q30")?,
        to_i32(fixture.param_i64("a1_q30")?, "a1_q30")?,
        to_i32(fixture.param_i64("a2_q30")?, "a2_q30")?,
    );
    let count = to_count(fixture.param_i64("count")?, "count")?;
    let level = to_i32(fixture.param_i64("input_level")?, "input_level")?;
    let input = fixture.param_str("input")?;
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let x = match input {
            "impulse" => {
                if n == 0 {
                    level
                } else {
                    0
                }
            }
            "step" => level,
            other => return Err(format!("input: unknown drive {other:?}")),
        };
        out.push(i64::from(biquad.process(x)));
    }
    Ok(vec![("expected_output_i32".to_string(), out)])
}

/// Sum one [`Nco`] per tone (parallel `tone_*` sections) into a
/// saturated `i16` multitone sample stream
fn multitone_samples(fixture: &Fixture, count: usize) -> Result<Vec<i16>, String> {
    let coeffs = fixture.section("tone_coeff_q30")?;
    let y1s = fixture.section("tone_y1_init")?;
    let y2s = fixture.section("tone_y2_init")?;
    if coeffs.len() != y1s.len() || coeffs.len() != y2s.len() {
        return Err("tone_* sections have mismatched lengths".to_string());
    }
    let mut ncos = Vec::with_capacity(coeffs.len());
    for i in 0..coeffs.len() {
        ncos.push(Nco::new(
            to_i32(coeffs[i], "tone_coeff_q30")?,
            to_i32(y1s[i], "tone_y1_init")?,
            to_i32(y2s[i], "tone_y2_init")?,
        ));
    }
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let sum: i64 = ncos.iter_mut().map(Nco::next_sample).sum();
        samples.push(sum.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16);
    }
    Ok(samples)
}

fn compute_multitone(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let count = to_count(fixture.param_i64("count")?, "count")?;
    let samples = multitone_samples(fixture, count)?;
    let mags = goertzel_mags(&samples, fixture.section("goertzel_coeff_q30")?)?;
    Ok(vec![
        (
            "expected_samples_i16".to_string(),
            samples.iter().map(|&s| i64::from(s)).collect(),
        ),
        ("expected_goertzel_mag_u32".to_string(), mags),
    ])
}

fn compute_goertzel_tone(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let count = to_count(fixture.param_i64("count")?, "count")?;
    let mut nco = nco_from_params(fixture)?;
    let samples: Vec<i16> = (0..count).map(|_| nco.next_i16()).collect();
    let mags = goertzel_mags(&samples, fixture.section("goertzel_coeff_q30")?)?;
    Ok(vec![("expected_goertzel_mag_u32".to_string(), mags)])
}

fn compute_ess_plant(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    let variant = plant_variant(fixture.param_i64("plant_variant")?)?;
    let seed = to_u32(fixture.param_i64("plant_seed")?, "plant_seed")?;
    let input = fixture.section("input_samples_i16")?;
    let mut buf = Vec::with_capacity(input.len());
    for &v in input {
        buf.push(to_i16(v, "input_samples_i16")?);
    }
    let mut plant = Plant::new(variant, seed);
    plant.process_buf(&mut buf);
    let mags = goertzel_mags(&buf, fixture.section("goertzel_coeff_q30")?)?;
    Ok(vec![
        (
            "expected_plant_out_i16".to_string(),
            buf.iter().map(|&s| i64::from(s)).collect(),
        ),
        ("expected_goertzel_mag_u32".to_string(), mags),
    ])
}

fn compute_capture(fixture: &Fixture) -> Result<Vec<(String, Vec<i64>)>, String> {
    // Excitation -> plant -> raw capture payload.
    let order = to_u8(fixture.param_i64("exc_order")?, "exc_order")?;
    let seed = to_u32(fixture.param_i64("exc_seed")?, "exc_seed")?;
    let level = to_i16(fixture.param_i64("exc_level_q15")?, "exc_level_q15")?;
    let count = to_count(fixture.param_i64("count")?, "count")?;
    let mut generator = MlsGen::new(order, seed, level)
        .ok_or_else(|| format!("no curated taps for order {order}"))?;
    let mut samples = vec![0i16; count];
    generator.fill(&mut samples);
    let mut plant = Plant::new(
        plant_variant(fixture.param_i64("plant_variant")?)?,
        to_u32(fixture.param_i64("plant_seed")?, "plant_seed")?,
    );
    plant.process_buf(&mut samples);

    let record = CaptureRecord {
        header: CaptureHeader {
            bench_id: to_u8(fixture.param_i64("bench_id")?, "bench_id")?,
            mic_id: to_u8(fixture.param_i64("mic_id")?, "mic_id")?,
            channels: to_u8(fixture.param_i64("channels")?, "channels")?,
            boot_id: to_u32(fixture.param_i64("boot_id")?, "boot_id")?,
            capture_seq: to_u32(fixture.param_i64("capture_seq")?, "capture_seq")?,
            monotonic_us: to_u64(fixture.param_i64("monotonic_us")?, "monotonic_us")?,
            unix_micros: to_u64(fixture.param_i64("unix_micros")?, "unix_micros")?,
            fs_hz: to_u32(fixture.param_i64("fs_hz")?, "fs_hz")?,
            excitation: generator.descriptor(),
        },
        payload: Payload::RawI16(&samples),
    };
    let mut capture_bytes = vec![0u8; record.encoded_len()];
    record
        .to_bytes(&mut capture_bytes)
        .map_err(|e| format!("capture to_bytes failed: {e:?}"))?;
    CaptureView::parse(&capture_bytes).map_err(|e| format!("capture re-parse failed: {e:?}"))?;

    let label = LabelRecord {
        bench_id: record.header.bench_id,
        mic_id: record.header.mic_id,
        hot_end: EndPresence::from_u8(to_u8(fixture.param_i64("label_hot_end")?, "label_hot_end")?)
            .ok_or("label_hot_end: bad EndPresence")?,
        cold_end: EndPresence::from_u8(to_u8(
            fixture.param_i64("label_cold_end")?,
            "label_cold_end",
        )?)
        .ok_or("label_cold_end: bad EndPresence")?,
        boot_id: record.header.boot_id,
        capture_seq: record.header.capture_seq,
        unix_micros: to_u64(fixture.param_i64("label_unix_micros")?, "label_unix_micros")?,
        confidence_q15: to_u16(
            fixture.param_i64("label_confidence_q15")?,
            "label_confidence_q15",
        )?,
        source: LabelSource::from_u8(to_u8(fixture.param_i64("label_source")?, "label_source")?)
            .ok_or("label_source: bad LabelSource")?,
    };
    let label_bytes = label.to_bytes();
    LabelRecord::parse(&label_bytes).map_err(|e| format!("label re-parse failed: {e:?}"))?;

    Ok(vec![
        (
            "expected_capture_bytes_u8".to_string(),
            capture_bytes.iter().map(|&b| i64::from(b)).collect(),
        ),
        (
            "expected_label_bytes_u8".to_string(),
            label_bytes.iter().map(|&b| i64::from(b)).collect(),
        ),
    ])
}
