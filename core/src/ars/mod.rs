//! Active acoustic resonance spectroscopy (ARS) domain types
//!
//! Schema types, synthetic signal generators, and the plant model
//! for the toolhead acoustic resonance spectroscopy project.  Gated
//! behind the `ars` feature so boards and projects that do not use
//! ARS do not pay for it.
//!
//! Wire records (`CAPTURE RECORD v1` / `LABEL RECORD v1`) are
//! serde-free: fixed little-endian layouts with hand-rolled
//! `to_bytes`/`parse` and a CRC32 trailer, because the byte layout
//! is a cross-language contract that host tooling parses directly.
//! See [`record`] for the schema and [`crc32`] for the checksum.
//!
//! Capture generation (sine, stepped sine, and MLS excitation) and
//! the synthetic plant model reuse the shared kernels in
//! [`crate::dsp`]-- no on-device trigonometry, no heap.

pub mod crc32;
pub mod generators;
pub mod record;
pub mod synth;
pub mod types;
