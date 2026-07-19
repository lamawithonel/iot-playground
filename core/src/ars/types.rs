//! Excitation descriptor and bin-plan arithmetic
//!
//! These types describe *what signal was played*, independent of
//! how a generator's oscillator coefficients were computed.  A
//! generator's `descriptor()` method returns one of these so the
//! capture record can never disagree with the signal that was
//! actually emitted (see design decision 8).

/// Discriminant for the kind of excitation signal played during a
/// capture
///
/// Values match the `exc.kind` byte in the `CAPTURE RECORD v1`
/// schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum ExcitationKind {
    /// Constant-frequency sine wave
    Sine = 0,
    /// Discrete frequency sweep, dwelling at each step
    SteppedSine = 1,
    /// Maximal-length binary sequence (LFSR-driven)
    Mls = 2,
}

impl ExcitationKind {
    /// Decode from the wire byte value
    ///
    /// Returns `None` for any value not defined above, so callers
    /// can reject unrecognized records instead of silently
    /// misinterpreting them.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Sine),
            1 => Some(Self::SteppedSine),
            2 => Some(Self::Mls),
            _ => None,
        }
    }

    /// Encode to the wire byte value
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Excitation descriptor embedded in a capture record
///
/// Mirrors the 24-byte `exc.*` block of `CAPTURE RECORD v1`
/// (offsets 40..64).  Field meaning varies by `kind`, per the
/// schema table:
///
/// - `Sine`: `f_start_dhz` is the played frequency; `f_stop_dhz`
///   mirrors it; `steps_or_order` and `seed` are `0`.
/// - `SteppedSine`: `f_start_dhz`/`f_stop_dhz` bound the sweep;
///   `steps_or_order` is the step count; `seed` is `0`.
/// - `Mls`: `steps_or_order` is the LFSR order; `seed` is the
///   effective (masked, substituted-if-zero) seed that ran;
///   `f_start_dhz`/`f_stop_dhz` are `0` (frequency content is
///   broadband, not a swept tone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ExcitationDescriptor {
    /// Which excitation kind was played
    pub kind: ExcitationKind,
    /// Reserved, always `0`
    pub flags: u8,
    /// Peak amplitude in Q15
    pub level_q15: i16,
    /// Sweep/tone start frequency in deci-Hz
    pub f_start_dhz: u32,
    /// Sweep stop frequency in deci-Hz
    pub f_stop_dhz: u32,
    /// Stepped sine: step count.  MLS: LFSR order.  Else `0`.
    pub steps_or_order: u16,
    /// Samples per dwell (stepped sine) or total samples emitted
    /// (sine, MLS)
    pub dwell: u32,
    /// Effective seed that ran.  `0` for `Sine`.
    pub seed: u32,
}

/// Bin-frequency bookkeeping for a stepped-sine sweep
///
/// Bin `i`'s frequency is `f_start + i * step`, where `step` is
/// the sweep range divided evenly across `steps - 1` intervals.
/// This is pure integer deci-Hz arithmetic-- no trigonometry-- so
/// it is safe to run on-device to label which dwell window
/// corresponds to which played frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinPlan {
    /// Sweep start frequency, deci-Hz
    pub f_start_dhz: u32,
    /// Sweep stop frequency, deci-Hz
    pub f_stop_dhz: u32,
    /// Number of discrete steps (bins) in the sweep
    pub steps: u16,
}

impl BinPlan {
    /// Build a bin plan from a `SteppedSine` excitation descriptor
    ///
    /// Returns `None` if `descriptor.kind` is not `SteppedSine`, if
    /// `steps_or_order` is `0` (no bins to plan), or if
    /// `f_stop_dhz < f_start_dhz` (descending sweeps are rejected
    /// because `step_dhz` computes `f_stop_dhz - f_start_dhz`, which
    /// would underflow).
    pub const fn from_descriptor(descriptor: &ExcitationDescriptor) -> Option<Self> {
        match descriptor.kind {
            ExcitationKind::SteppedSine
                if descriptor.steps_or_order > 0
                    && descriptor.f_stop_dhz >= descriptor.f_start_dhz =>
            {
                Some(Self {
                    f_start_dhz: descriptor.f_start_dhz,
                    f_stop_dhz: descriptor.f_stop_dhz,
                    steps: descriptor.steps_or_order,
                })
            }
            _ => None,
        }
    }

    /// Frequency step between adjacent bins, in deci-Hz
    ///
    /// `0` for a single-bin plan (`steps == 1`), which has no
    /// interval to step across.
    pub const fn step_dhz(&self) -> u32 {
        if self.steps <= 1 {
            0
        } else {
            (self.f_stop_dhz - self.f_start_dhz) / (self.steps as u32 - 1)
        }
    }

    /// Frequency of bin `i`, in deci-Hz
    ///
    /// Returns `None` if `i >= steps`.
    pub const fn freq_dhz(&self, i: u16) -> Option<u32> {
        if i >= self.steps {
            None
        } else {
            Some(self.f_start_dhz + i as u32 * self.step_dhz())
        }
    }

    /// Sample-index half-open range `[start, end)` occupied by bin
    /// `i`'s dwell, given `dwell` samples per bin
    ///
    /// Returns `None` if `i >= steps`.
    pub const fn dwell_range(&self, i: u16, dwell: u32) -> Option<(u32, u32)> {
        if i >= self.steps {
            None
        } else {
            let start = i as u32 * dwell;
            Some((start, start + dwell))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_roundtrip() {
        for kind in [
            ExcitationKind::Sine,
            ExcitationKind::SteppedSine,
            ExcitationKind::Mls,
        ] {
            assert_eq!(ExcitationKind::from_u8(kind.as_u8()), Some(kind));
        }
    }

    #[test]
    fn test_kind_unknown_byte_rejected() {
        assert_eq!(ExcitationKind::from_u8(255), None);
    }

    fn stepped_descriptor(f_start: u32, f_stop: u32, steps: u16) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::SteppedSine,
            flags: 0,
            level_q15: 16_000,
            f_start_dhz: f_start,
            f_stop_dhz: f_stop,
            steps_or_order: steps,
            dwell: 1024,
            seed: 0,
        }
    }

    #[test]
    fn test_bin_plan_freq_linear_steps() {
        // 100.0 Hz to 500.0 Hz (deci-Hz: 1000..5000) across 5 bins
        // -> step = (5000-1000)/(5-1) = 1000 deci-Hz = 100.0 Hz.
        let plan = BinPlan::from_descriptor(&stepped_descriptor(1000, 5000, 5)).unwrap();
        assert_eq!(plan.step_dhz(), 1000);
        assert_eq!(plan.freq_dhz(0), Some(1000));
        assert_eq!(plan.freq_dhz(1), Some(2000));
        assert_eq!(plan.freq_dhz(4), Some(5000));
        assert_eq!(plan.freq_dhz(5), None);
    }

    #[test]
    fn test_bin_plan_single_step_has_zero_span() {
        let plan = BinPlan::from_descriptor(&stepped_descriptor(2000, 2000, 1)).unwrap();
        assert_eq!(plan.step_dhz(), 0);
        assert_eq!(plan.freq_dhz(0), Some(2000));
    }

    #[test]
    fn test_bin_plan_rejects_descending_sweep() {
        // f_stop < f_start would underflow step_dhz's subtraction.
        assert!(BinPlan::from_descriptor(&stepped_descriptor(5000, 1000, 5)).is_none());
    }

    #[test]
    fn test_bin_plan_rejects_non_stepped_sine() {
        let sine = ExcitationDescriptor {
            kind: ExcitationKind::Sine,
            flags: 0,
            level_q15: 16_000,
            f_start_dhz: 12_000,
            f_stop_dhz: 12_000,
            steps_or_order: 0,
            dwell: 4096,
            seed: 0,
        };
        assert!(BinPlan::from_descriptor(&sine).is_none());
    }

    #[test]
    fn test_dwell_range_matches_bin_index() {
        let plan = BinPlan::from_descriptor(&stepped_descriptor(1000, 5000, 5)).unwrap();
        assert_eq!(plan.dwell_range(0, 256), Some((0, 256)));
        assert_eq!(plan.dwell_range(2, 256), Some((512, 768)));
        assert_eq!(plan.dwell_range(5, 256), None);
    }
}
