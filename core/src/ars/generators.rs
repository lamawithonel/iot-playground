//! Excitation signal generators
//!
//! `SineGen`, `SteppedSineGen`, and `MlsGen` share a fill-buffer
//! interface (`fill(&mut self, &mut [i16]) -> usize`)-- the primary
//! generation API, because the RTIC sweep engine feeds DMA
//! half-buffers and Helium vectorization wants slices.  [`Excitation`]
//! wraps all three for runtime mode selection via a plain enum with
//! static dispatch (no `dyn`, no heap).
//!
//! None of these generators computes trigonometry at runtime:
//! oscillator coefficients (Q1.30 cosine values) are supplied by
//! the caller, precomputed on the host or from a checked-in table.
//! Every generator's `descriptor()` returns the [`ExcitationDescriptor`]
//! that matches exactly what it emitted, so the capture record can
//! never disagree with the signal that was actually played.

use crate::dsp::lfsr::Lfsr;
use crate::dsp::nco::Nco;

use super::types::{BinPlan, ExcitationDescriptor, ExcitationKind};

/// Continuous constant-frequency sine generator
#[derive(Debug, Clone, Copy)]
pub struct SineGen {
    nco: Nco,
    level_q15: i16,
    f_dhz: u32,
    samples_emitted: u32,
}

impl SineGen {
    /// Build a sine generator
    ///
    /// `coeff_q30` is `cos(2*pi*f/fs)` in Q1.30; `y1_init`/`y2_init`
    /// are the oscillator's initial-condition samples (natural Q15
    /// scale)-- see [`crate::dsp::nco::Nco::new`].  `f_dhz` is the
    /// played frequency in deci-Hz, recorded verbatim in the
    /// descriptor.
    pub const fn new(
        coeff_q30: i32,
        y1_init: i32,
        y2_init: i32,
        f_dhz: u32,
        level_q15: i16,
    ) -> Self {
        Self {
            nco: Nco::new(coeff_q30, y1_init, y2_init),
            level_q15,
            f_dhz,
            samples_emitted: 0,
        }
    }

    /// Fill `buf` with the next `buf.len()` samples
    ///
    /// Always fills the entire buffer (a continuous sine never
    /// runs out); returns `buf.len()`.
    pub fn fill(&mut self, buf: &mut [i16]) -> usize {
        for s in buf.iter_mut() {
            *s = self.nco.next_i16();
        }
        self.samples_emitted = self.samples_emitted.saturating_add(buf.len() as u32);
        buf.len()
    }

    /// The excitation descriptor for what this generator is
    /// emitting
    pub const fn descriptor(&self) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::Sine,
            flags: 0,
            level_q15: self.level_q15,
            f_start_dhz: self.f_dhz,
            f_stop_dhz: self.f_dhz,
            steps_or_order: 0,
            dwell: self.samples_emitted,
            seed: 0,
        }
    }
}

/// One step's precomputed oscillator setup for [`SteppedSineGen`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepCoeffs {
    /// `cos(2*pi*f/fs)` in Q1.30 for this step's frequency
    pub coeff_q30: i32,
    /// Oscillator initial condition `y[-1]` (natural Q15 scale)
    pub y1_init: i32,
    /// Oscillator initial condition `y[-2]` (natural Q15 scale)
    pub y2_init: i32,
}

/// Discrete frequency-sweep generator, dwelling at each step
///
/// Steps through a caller-supplied coefficient table, holding each
/// frequency for `dwell` samples before moving to the next.  The
/// coefficient table and the [`BinPlan`] must describe the same
/// number of steps-- see [`SteppedSineGen::new`].
#[derive(Debug, Clone, Copy)]
pub struct SteppedSineGen<'a> {
    coeffs: &'a [StepCoeffs],
    bin_plan: BinPlan,
    dwell: u32,
    level_q15: i16,
    step_idx: u16,
    step_remaining: u32,
    nco: Nco,
}

impl<'a> SteppedSineGen<'a> {
    /// Build a stepped-sine generator
    ///
    /// `coeffs.len()` must equal `bin_plan.steps`; if it does not,
    /// the sweep stops at whichever is shorter.
    pub fn new(coeffs: &'a [StepCoeffs], bin_plan: BinPlan, dwell: u32, level_q15: i16) -> Self {
        let mut gen = Self {
            coeffs,
            bin_plan,
            dwell,
            level_q15,
            step_idx: 0,
            step_remaining: 0,
            nco: Nco::new(0, 0, 0),
        };
        gen.load_step(0);
        gen
    }

    fn load_step(&mut self, idx: u16) {
        if let Some(c) = self.coeffs.get(idx as usize) {
            self.nco = Nco::new(c.coeff_q30, c.y1_init, c.y2_init);
            self.step_remaining = self.dwell;
        } else {
            self.step_remaining = 0;
        }
    }

    /// Whether the sweep has emitted its final step's last sample
    pub const fn is_finished(&self) -> bool {
        self.step_idx as usize >= self.coeffs.len() && self.step_remaining == 0
    }

    /// Fill `buf`, advancing through sweep steps as needed
    ///
    /// Returns the number of samples written, which is less than
    /// `buf.len()` once the sweep runs out of steps.  Once finished,
    /// every subsequent call returns `0` without touching internal
    /// state-- callers may keep polling (as the RTIC sweep engine
    /// does, one DMA half-buffer at a time) indefinitely.
    pub fn fill(&mut self, buf: &mut [i16]) -> usize {
        if self.step_idx as usize >= self.coeffs.len() {
            return 0;
        }
        let mut n = 0;
        while n < buf.len() {
            if self.step_remaining == 0 {
                if self.step_idx as usize >= self.coeffs.len() {
                    break;
                }
                self.step_idx += 1;
                self.load_step(self.step_idx);
                if self.step_remaining == 0 {
                    break;
                }
            }
            buf[n] = self.nco.next_i16();
            self.step_remaining -= 1;
            n += 1;
            // The final sample of the final step completes the
            // sweep immediately-- do not wait for a follow-up fill
            // call to discover the out-of-range next step, which
            // left `is_finished()` false until one extra call and
            // let `step_idx` climb unboundedly on every poll after
            // that.
            if self.step_remaining == 0 && self.step_idx as usize + 1 >= self.coeffs.len() {
                self.step_idx = self.coeffs.len() as u16;
            }
        }
        n
    }

    /// The excitation descriptor for this sweep
    pub const fn descriptor(&self) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::SteppedSine,
            flags: 0,
            level_q15: self.level_q15,
            f_start_dhz: self.bin_plan.f_start_dhz,
            f_stop_dhz: self.bin_plan.f_stop_dhz,
            steps_or_order: self.bin_plan.steps,
            dwell: self.dwell,
            seed: 0,
        }
    }
}

/// Maximal-length binary sequence (MLS) generator
///
/// Emits `+level_q15`/`-level_q15` samples driven by an LFSR
/// feedback bit-- broadband excitation with no swept tone.
#[derive(Debug, Clone, Copy)]
pub struct MlsGen {
    lfsr: Lfsr,
    level_q15: i16,
    order: u16,
    seed: u32,
    samples_emitted: u32,
}

impl MlsGen {
    /// Build an MLS generator for the curated LFSR `order`
    ///
    /// Returns `None` if `order` has no entry in
    /// [`crate::dsp::lfsr::xapp052_taps`].  The seed is masked to
    /// `order` bits and substituted if it would otherwise be zero
    /// (see [`Lfsr::new`]); [`MlsGen::descriptor`] always reports
    /// the seed that actually ran.
    pub fn new(order: u8, seed: u32, level_q15: i16) -> Option<Self> {
        let lfsr = Lfsr::new(order, seed)?;
        let effective_seed = lfsr.effective_seed();
        Some(Self {
            lfsr,
            level_q15,
            order: order as u16,
            seed: effective_seed,
            samples_emitted: 0,
        })
    }

    /// Fill `buf` with the next `buf.len()` MLS samples
    ///
    /// Always fills the entire buffer; returns `buf.len()`.
    pub fn fill(&mut self, buf: &mut [i16]) -> usize {
        for s in buf.iter_mut() {
            *s = if self.lfsr.next_bit() {
                self.level_q15
            } else {
                -self.level_q15
            };
        }
        self.samples_emitted = self.samples_emitted.saturating_add(buf.len() as u32);
        buf.len()
    }

    /// The excitation descriptor for this generator
    pub const fn descriptor(&self) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::Mls,
            flags: 0,
            level_q15: self.level_q15,
            f_start_dhz: 0,
            f_stop_dhz: 0,
            steps_or_order: self.order,
            dwell: self.samples_emitted,
            seed: self.seed,
        }
    }
}

/// Runtime excitation mode selection, statically dispatched
///
/// No `dyn`, no heap: this is a plain enum wrapping each concrete
/// generator, matching the design's static-dispatch requirement.
#[derive(Debug, Clone, Copy)]
pub enum Excitation<'a> {
    /// Constant-frequency sine
    Sine(SineGen),
    /// Discrete frequency sweep
    SteppedSine(SteppedSineGen<'a>),
    /// Maximal-length binary sequence
    Mls(MlsGen),
}

impl<'a> Excitation<'a> {
    /// Fill `buf` with the next samples from whichever generator
    /// is active
    pub fn fill(&mut self, buf: &mut [i16]) -> usize {
        match self {
            Self::Sine(g) => g.fill(buf),
            Self::SteppedSine(g) => g.fill(buf),
            Self::Mls(g) => g.fill(buf),
        }
    }

    /// The excitation descriptor for whichever generator is active
    pub const fn descriptor(&self) -> ExcitationDescriptor {
        match self {
            Self::Sine(g) => g.descriptor(),
            Self::SteppedSine(g) => g.descriptor(),
            Self::Mls(g) => g.descriptor(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q30: f64 = (1u64 << 30) as f64;

    fn to_q30(x: f64) -> i32 {
        (x * Q30).round() as i32
    }

    fn sine_coeffs(freq_hz: f64, fs_hz: f64, amplitude: f64) -> (i32, i32, i32) {
        let theta = 2.0 * core::f64::consts::PI * freq_hz / fs_hz;
        (
            to_q30(theta.cos()),
            (amplitude * theta.sin()).round() as i32,
            0,
        )
    }

    #[test]
    fn test_sine_gen_fill_always_fills_whole_buffer() {
        let (coeff, y1, y2) = sine_coeffs(1_200.0, 48_000.0, 16_000.0);
        let mut gen = SineGen::new(coeff, y1, y2, 12_000, 16_000);
        let mut buf = [0i16; 37];
        assert_eq!(gen.fill(&mut buf), 37);
    }

    #[test]
    fn test_sine_gen_block_size_invariance() {
        // Filling in one large call must equal filling across
        // several smaller calls-- required so the RTIC sweep
        // engine's DMA half-buffer size never changes behavior.
        let (coeff, y1, y2) = sine_coeffs(1_200.0, 48_000.0, 16_000.0);
        let mut whole = SineGen::new(coeff, y1, y2, 12_000, 16_000);
        let mut split = SineGen::new(coeff, y1, y2, 12_000, 16_000);

        let mut whole_buf = [0i16; 100];
        whole.fill(&mut whole_buf);

        let mut split_buf = [0i16; 100];
        split.fill(&mut split_buf[0..30]);
        split.fill(&mut split_buf[30..70]);
        split.fill(&mut split_buf[70..100]);

        assert_eq!(whole_buf, split_buf);
    }

    #[test]
    fn test_sine_gen_descriptor() {
        let (coeff, y1, y2) = sine_coeffs(1_200.0, 48_000.0, 16_000.0);
        let mut gen = SineGen::new(coeff, y1, y2, 12_000, 16_000);
        let mut buf = [0i16; 50];
        gen.fill(&mut buf);
        let d = gen.descriptor();
        assert_eq!(d.kind, ExcitationKind::Sine);
        assert_eq!(d.f_start_dhz, 12_000);
        assert_eq!(d.f_stop_dhz, 12_000);
        assert_eq!(d.steps_or_order, 0);
        assert_eq!(d.dwell, 50);
        assert_eq!(d.seed, 0);
    }

    fn stepped_descriptor(f_start: u32, f_stop: u32, steps: u16) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::SteppedSine,
            flags: 0,
            level_q15: 16_000,
            f_start_dhz: f_start,
            f_stop_dhz: f_stop,
            steps_or_order: steps,
            dwell: 8,
            seed: 0,
        }
    }

    fn make_step_table(
        bin_plan: &BinPlan,
        fs_hz: f64,
        amplitude: f64,
    ) -> heapless::Vec<StepCoeffs, 4> {
        let mut table = heapless::Vec::new();
        for i in 0..bin_plan.steps {
            let f_dhz = bin_plan.freq_dhz(i).unwrap();
            let (coeff, y1, y2) = sine_coeffs(f_dhz as f64 / 10.0, fs_hz, amplitude);
            table
                .push(StepCoeffs {
                    coeff_q30: coeff,
                    y1_init: y1,
                    y2_init: y2,
                })
                .unwrap();
        }
        table
    }

    #[test]
    fn test_stepped_sine_advances_and_terminates() {
        let descriptor = stepped_descriptor(10_000, 40_000, 4);
        let bin_plan = BinPlan::from_descriptor(&descriptor).unwrap();
        let coeffs = make_step_table(&bin_plan, 48_000.0, 16_000.0);

        let mut gen = SteppedSineGen::new(&coeffs, bin_plan, 8, 16_000);
        let mut buf = [0i16; 100];
        let written = gen.fill(&mut buf);

        // 4 steps * 8 samples/step = 32 total samples available.
        assert_eq!(written, 32);
        assert!(gen.is_finished());
    }

    #[test]
    fn test_stepped_sine_block_size_invariance() {
        let descriptor = stepped_descriptor(10_000, 40_000, 4);
        let bin_plan = BinPlan::from_descriptor(&descriptor).unwrap();
        let coeffs = make_step_table(&bin_plan, 48_000.0, 16_000.0);

        let mut whole = SteppedSineGen::new(&coeffs, bin_plan, 8, 16_000);
        let mut whole_buf = [0i16; 32];
        whole.fill(&mut whole_buf);

        let mut split = SteppedSineGen::new(&coeffs, bin_plan, 8, 16_000);
        let mut split_buf = [0i16; 32];
        split.fill(&mut split_buf[0..5]);
        split.fill(&mut split_buf[5..20]);
        split.fill(&mut split_buf[20..32]);

        assert_eq!(whole_buf, split_buf);
    }

    #[test]
    fn test_stepped_sine_exact_fit_then_poll() {
        let descriptor = stepped_descriptor(10_000, 40_000, 4);
        let bin_plan = BinPlan::from_descriptor(&descriptor).unwrap();
        let coeffs = make_step_table(&bin_plan, 48_000.0, 16_000.0);

        let mut gen = SteppedSineGen::new(&coeffs, bin_plan, 8, 16_000);
        let mut buf = [0i16; 32];

        // Exact fit: the buffer length equals the sweep's total
        // sample count (4 steps * 8 samples/step)-- the DMA
        // half-buffer case this API is designed for.  Finished state
        // must be visible right away, not after one extra poll.
        let written = gen.fill(&mut buf);
        assert_eq!(written, 32);
        assert!(gen.is_finished());

        // Every fill call after completion (as the RTIC sweep
        // engine keeps feeding DMA half-buffers) must return 0 and
        // leave the generator finished, well past the range of the
        // internal u16 step counter.
        for _ in 0..(u32::from(u16::MAX) + 10) {
            assert_eq!(gen.fill(&mut buf), 0);
            assert!(gen.is_finished());
        }
    }

    #[test]
    fn test_stepped_sine_descriptor() {
        let descriptor = stepped_descriptor(10_000, 40_000, 4);
        let bin_plan = BinPlan::from_descriptor(&descriptor).unwrap();
        let coeffs = make_step_table(&bin_plan, 48_000.0, 16_000.0);
        let gen = SteppedSineGen::new(&coeffs, bin_plan, 8, 16_000);
        let d = gen.descriptor();
        assert_eq!(d.kind, ExcitationKind::SteppedSine);
        assert_eq!(d.f_start_dhz, 10_000);
        assert_eq!(d.f_stop_dhz, 40_000);
        assert_eq!(d.steps_or_order, 4);
        assert_eq!(d.dwell, 8);
    }

    #[test]
    fn test_mls_gen_output_is_bipolar() {
        let mut gen = MlsGen::new(9, 123, 16_000).unwrap();
        let mut buf = [0i16; 64];
        gen.fill(&mut buf);
        for &s in buf.iter() {
            assert!(s == 16_000 || s == -16_000);
        }
    }

    #[test]
    fn test_mls_gen_block_size_invariance() {
        let mut whole = MlsGen::new(9, 123, 16_000).unwrap();
        let mut whole_buf = [0i16; 40];
        whole.fill(&mut whole_buf);

        let mut split = MlsGen::new(9, 123, 16_000).unwrap();
        let mut split_buf = [0i16; 40];
        split.fill(&mut split_buf[0..7]);
        split.fill(&mut split_buf[7..40]);

        assert_eq!(whole_buf, split_buf);
    }

    #[test]
    fn test_mls_gen_descriptor_reports_effective_seed() {
        // order 4 masks the seed to 4 bits: 0x1F & 0xF == 0xF.
        let gen = MlsGen::new(4, 0x1F, 16_000).unwrap();
        let d = gen.descriptor();
        assert_eq!(d.kind, ExcitationKind::Mls);
        assert_eq!(d.steps_or_order, 4);
        assert_eq!(d.seed, 0xF);
    }

    #[test]
    fn test_mls_gen_unknown_order_returns_none() {
        assert!(MlsGen::new(6, 1, 16_000).is_none());
    }

    #[test]
    fn test_excitation_enum_dispatches_to_active_variant() {
        let gen = MlsGen::new(9, 1, 16_000).unwrap();
        let mut exc = Excitation::Mls(gen);
        let mut buf = [0i16; 16];
        assert_eq!(exc.fill(&mut buf), 16);
        assert_eq!(exc.descriptor().kind, ExcitationKind::Mls);
    }
}
