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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{now_or_never, MockAdcCapture, MockAdcCaptureError};

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
}
