//! ARS toolhead sensor-- NUCLEO-N657X0-Q board crate (scaffold).
//!
//! Planned RTIC 2.x task topology, per
//! `docs/src/projects/ars-toolhead-sensor/README.md`:
//!
//! - `init`: clocks, pins, audio output path, ADC setup
//! - `sweep_engine`: swept-sine synthesis feeding the MAX9744
//! - `capture`: ADC capture windows synchronized to the sweep
//! - `analysis`: FFT, feature extraction, and classification
//! - `telemetry`: labels and features over the framework MQTT
//!   stack once ported
//! - `idle`: WFI sleep
//!
//! Framework rules apply: RTIC 2.x scheduling, Embassy HAL only
//! (no embassy-executor), `no_std`, no heap, defmt logging.
//!
//! Nothing here builds yet-- the toolchain and boot-chain
//! decisions land at the bring-up spike.

#![no_std]
#![no_main]

compile_error!("scaffold only -- see boards/nucleo-n657x0/README.md");
