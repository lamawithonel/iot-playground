//! Windowed ADC sample capture abstraction
//!
//! One analysis window at a time: the caller hands over a Q15
//! buffer, the implementation fills it with the next contiguous
//! block of converted samples.  Shaped so a timer-paced ADC + DMA
//! ring buffer (the N6 toolhead capture chain and the H7 DAC->ADC
//! loopback rig both use one) can satisfy it by copying out of the
//! ring when the half/full-transfer interrupt fires, while staying
//! free of any hardware types.

/// Windowed Q15 capture source.
///
/// Rationale: both consumers (H7 loopback rig, N6 toolhead app)
/// analyze fixed-length windows-- Goertzel bins and capture-record
/// payloads-- so the only operation needed is "fill one window",
/// plus the two facts analysis and record headers need: the real
/// sample rate (`fs_hz` in `CAPTURE RECORD v1`) and the largest
/// window the backing buffer can deliver.  Streaming callbacks,
/// start/stop control, and channel selection stay board-side.
pub trait AdcCapture {
    /// Implementation-specific capture failure (e.g., DMA overrun,
    /// unsupported window length).
    type Error: core::fmt::Debug;

    /// Largest window length, in samples, a single
    /// [`AdcCapture::capture`] call can fill.
    ///
    /// Bounded by the implementation's DMA/ring-buffer sizing;
    /// callers size their window buffers against this.
    const MAX_WINDOW_LEN: usize;

    /// Configured sample rate in Hz.
    ///
    /// The actually-achieved hardware rate, not the requested one--
    /// this value goes verbatim into capture record headers.
    fn sample_rate_hz(&self) -> u32;

    /// Fill `window` completely with the next capture window.
    ///
    /// Samples are signed Q15.  `window.len()` selects the window
    /// length; the samples within one window are guaranteed
    /// contiguous in time, but consecutive windows need not be.
    /// Implementations MUST fail (not truncate) when `window.len()`
    /// exceeds [`AdcCapture::MAX_WINDOW_LEN`].
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn capture(&mut self, window: &mut [i16]) -> Result<(), Self::Error>;
}

/// [`AdcCapture`] extension for trigger-synchronized window starts.
///
/// Rationale: phase-locked bench measurements (e.g., the H753ZI
/// DAC->ADC loopback rig's planned `audio_loopback` module; see
/// `docs/src/projects/ars-toolhead-sensor/hil-measurements.md`)
/// need to know not just *that* a window was captured but *when its
/// first sample landed relative to a hardware trigger edge*-- a
/// fact plain [`AdcCapture::capture`] has no vocabulary for.  This
/// stays a separate, optional extension rather than growing the
/// base trait, since most `AdcCapture` consumers (e.g., the N6
/// toolhead's free-running mic path) never trigger-synchronize at
/// all.
pub trait TriggeredCapture: AdcCapture {
    /// Arm on the next hardware trigger edge, then fill `window`
    /// with the resulting capture.
    ///
    /// Returns the trigger-to-first-sample latency actually
    /// observed, in samples at the rate [`AdcCapture::sample_rate_hz`]
    /// reports-- implementations report this so callers can check
    /// it against their own declared tolerance; it is not itself an
    /// error condition.
    #[allow(async_fn_in_trait)]
    async fn capture_after_trigger(&mut self, window: &mut [i16]) -> Result<u32, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        now_or_never, MockAdcCapture, MockAdcCaptureError, MockTriggeredCapture,
    };

    #[test]
    fn capture_replays_pattern_continuously_across_windows() {
        let pattern: [i16; 3] = [100, -200, 300];
        let mut adc = MockAdcCapture::new(&pattern, 48_000);

        let mut first = [0i16; 4];
        assert_eq!(now_or_never(adc.capture(&mut first)), Some(Ok(())));
        assert_eq!(first, [100, -200, 300, 100]);

        // The stream continues where the previous window ended.
        let mut second = [0i16; 2];
        assert_eq!(now_or_never(adc.capture(&mut second)), Some(Ok(())));
        assert_eq!(second, [-200, 300]);
        assert_eq!(adc.windows_captured(), 2);
    }

    #[test]
    fn sample_rate_is_reported_verbatim() {
        let pattern: [i16; 1] = [0];
        let adc = MockAdcCapture::new(&pattern, 44_100);
        assert_eq!(adc.sample_rate_hz(), 44_100);
    }

    #[test]
    fn oversized_window_is_rejected_not_truncated() {
        const OVERSIZED: usize = <MockAdcCapture<'static> as AdcCapture>::MAX_WINDOW_LEN + 1;
        let pattern: [i16; 2] = [1, -1];
        let mut adc = MockAdcCapture::new(&pattern, 48_000);
        let mut window = [0i16; OVERSIZED];
        assert_eq!(
            now_or_never(adc.capture(&mut window)),
            Some(Err(MockAdcCaptureError::WindowTooLong))
        );
        assert_eq!(adc.windows_captured(), 0);
    }

    #[test]
    fn injected_failure_recovers_after_one_call() {
        let pattern: [i16; 2] = [7, -7];
        let mut adc = MockAdcCapture::new(&pattern, 48_000);
        adc.fail_next(MockAdcCaptureError::Overrun);

        let mut window = [0i16; 2];
        assert_eq!(
            now_or_never(adc.capture(&mut window)),
            Some(Err(MockAdcCaptureError::Overrun))
        );
        // Recovered: the next window succeeds and the failed call
        // did not consume pattern samples.
        assert_eq!(now_or_never(adc.capture(&mut window)), Some(Ok(())));
        assert_eq!(window, [7, -7]);
    }

    /// Generic consumer proof: the shape a capture task uses to
    /// stay board-free.
    fn window_energy<A: AdcCapture>(adc: &mut A, window: &mut [i16]) -> Option<u64> {
        let captured = now_or_never(adc.capture(window))?;
        captured.ok()?;
        Some(
            window
                .iter()
                .map(|&s| (i64::from(s) * i64::from(s)) as u64)
                .sum(),
        )
    }

    #[test]
    fn generic_consumer_bound_is_usable() {
        let pattern: [i16; 2] = [3, -4];
        let mut adc = MockAdcCapture::new(&pattern, 48_000);
        let mut window = [0i16; 2];
        assert_eq!(window_energy(&mut adc, &mut window), Some(25));
    }

    #[test]
    fn test_triggered_capture_offset_is_within_declared_tolerance() {
        let pattern: [i16; 4] = [10, -10, 20, -20];
        let mut adc = MockTriggeredCapture::new(&pattern, 48_000, 3);
        let mut window = [0i16; 4];

        let offset = now_or_never(adc.capture_after_trigger(&mut window))
            .expect("mock future completes synchronously")
            .expect("mock capture does not fail");
        assert_eq!(window, pattern);

        // Proves the tolerance-check logic itself, independent of
        // any real Saleae-measured latency spec.
        let expected_offset_samples = 3;
        let declared_tolerance_samples = 1;
        assert!(
            offset.abs_diff(expected_offset_samples) <= declared_tolerance_samples,
            "trigger-to-first-sample offset {offset} exceeds the declared \
             {declared_tolerance_samples}-sample tolerance around \
             {expected_offset_samples}"
        );
    }

    #[test]
    #[ignore = "RED: needs the H753ZI audio_loopback module to exist at \
                all (currently only 'planned' in \
                boards/nucleo-h753zi/AGENTS.md) plus a Saleae capture \
                correlating the TIM-trigger/capture-strobe digital edges \
                to the analog DAC/ADC waveforms per hil-measurements.md"]
    fn test_phase_locked_capture_window_matches_saleae_trigger_edge() {
        todo!(
            "requires the H753ZI audio_loopback module plus a bench \
             Saleae capture; see \
             docs/src/projects/ars-toolhead-sensor/hil-measurements.md"
        )
    }
}
