//! Reusable fixed-point DSP kernels
//!
//! Always compiled (no feature gate) so any board or project can
//! reuse these primitives without pulling in ARS-specific schema
//! types.  Every kernel is integer-only-- no `f32`/`f64`, no
//! runtime trigonometry, and no external DSP crates.  Trig-derived
//! coefficients (cosine, sine) are computed once on the host and
//! passed in by the caller as fixed-point constants.
//!
//! # Fixed-point conventions
//!
//! - **Q1.30**: a signed fraction in `[-1.0, 1.0)`, stored in
//!   `i32` with 30 fractional bits.  Used for coefficients whose
//!   magnitude never reaches or exceeds 1.0 (cosine/sine values).
//! - **Q2.30**: a signed fraction in `[-2.0, 2.0)`, stored in
//!   `i32` with 30 fractional bits.  Used for biquad `a1`/`a2`
//!   coefficients, which can reach magnitude 2.0 for poles near
//!   the unit circle.
//! - Filter and oscillator *state* (`i64` accumulators) is kept in
//!   the same natural integer scale as the samples flowing through
//!   the kernel (e.g., Q15 audio counts)-- it is not itself
//!   Q30-scaled.  Only coefficients carry the Q30 fractional
//!   scale, and every coefficient multiply is followed by an
//!   arithmetic right shift back to the state's natural scale.

pub mod biquad;
pub mod goertzel;
pub mod lfsr;
pub mod nco;
pub mod xoshiro128;
