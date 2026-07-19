//! Excitation output sink abstraction
//!
//! The core generators (`SineGen`, `SteppedSineGen`, `MlsGen`)
//! produce Q15 sample blocks via their `fill` interface; this trait
//! is the other half of that loop-- it sinks a filled block to
//! whatever hardware drives the exciter.  On the N6 toolhead that
//! is TIM1_CH1 PWM duty updates streamed by DMA through an RC
//! low-pass; on the H7 loopback rig it is the on-chip DAC.  Either
//! way the caller owns signal generation and the sink owns pacing.

/// Q15 excitation output sink.
///
/// Rationale: the sweep engine's inner loop is
/// `generator.fill(&mut block)` followed by "emit that block", and
/// the emit half is the only board-specific part-- so the trait is
/// exactly one awaitable block write.  `write` returning is the
/// backpressure signal: it completes when the hardware has accepted
/// the block (e.g., the previous DMA half-buffer drained), so the
/// fill/write loop paces itself off the output rate with no
/// busy-wait, per the interrupt-driven design rules.
pub trait ExcitationSink {
    /// Implementation-specific output failure (e.g., DMA underrun).
    type Error: core::fmt::Debug;

    /// Emit the next block of signed Q15 excitation samples.
    ///
    /// Samples are consumed in order, continuing the previously
    /// written block.  Completes once the sink has accepted the
    /// whole block; an empty block is a no-op.
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn write(&mut self, samples: &[i16]) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{now_or_never, MockExcitation, MockExcitationError};

    #[test]
    fn write_records_samples_in_order_across_blocks() {
        let mut sink = MockExcitation::<8>::new();
        assert_eq!(now_or_never(sink.write(&[1, -2, 3])), Some(Ok(())));
        assert_eq!(now_or_never(sink.write(&[-4, 5])), Some(Ok(())));
        assert_eq!(sink.emitted(), &[1, -2, 3, -4, 5]);
    }

    #[test]
    fn empty_write_is_noop() {
        let mut sink = MockExcitation::<4>::new();
        assert_eq!(now_or_never(sink.write(&[])), Some(Ok(())));
        assert_eq!(sink.emitted(), &[] as &[i16]);
    }

    #[test]
    fn overflowing_block_is_rejected_whole() {
        let mut sink = MockExcitation::<4>::new();
        assert_eq!(now_or_never(sink.write(&[1, 2, 3])), Some(Ok(())));
        // 3 + 2 > 4: the whole block is rejected, nothing partial.
        assert_eq!(
            now_or_never(sink.write(&[4, 5])),
            Some(Err(MockExcitationError::CapacityExceeded))
        );
        assert_eq!(sink.emitted(), &[1, 2, 3]);
    }

    /// Generic consumer proof: the fill/write loop shape the sweep
    /// engine uses, with a stand-in generator.
    #[test]
    fn composes_with_a_fill_style_generator() {
        // Stand-in for `core`'s generator `fill` contract: writes a
        // ramp, returns the count written.
        fn fill_ramp(start: i16, block: &mut [i16]) -> usize {
            for (i, slot) in block.iter_mut().enumerate() {
                *slot = start + i as i16;
            }
            block.len()
        }

        let mut sink = MockExcitation::<6>::new();
        let mut block = [0i16; 3];
        for start in [0i16, 3] {
            let n = fill_ramp(start, &mut block);
            assert_eq!(now_or_never(sink.write(&block[..n])), Some(Ok(())));
        }
        assert_eq!(sink.emitted(), &[0, 1, 2, 3, 4, 5]);
    }
}
