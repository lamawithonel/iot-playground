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
//! The default feature set is still a deliberate `compile_error!`
//! scaffold guard.  Under the `g1-spike` feature the crate builds
//! and boots: the gate G0 RAM-boot flow was bench-verified
//! 2026-07-21 (see the board page's bring-up section for the load
//! flow of record).
//!
//! See `pins.rs` for the pin map of record in code form.

#![no_std]
#![no_main]

#[cfg(not(feature = "g1-spike"))]
compile_error!("scaffold only -- see boards/nucleo-n657x0/README.md");

// pins.rs is a documentation-only sketch of the pin map (see its
// module doc); it is intentionally not compiled during the G1
// spike since G1 only proves the dependency stack compiles, and
// pulling it in would trip `#![deny(warnings)]` dead-code lints
// on consts nothing in this minimal skeleton consumes yet.
#[cfg(not(feature = "g1-spike"))]
mod pins;

// ── Gate G1 compile spike ────────────────────────────────────────
// Minimal RTIC 2 skeleton to prove embassy-stm32 + rtic 2.x +
// rtic-monotonics + defmt/defmt-rtt/panic-probe compile for the
// Cortex-M55 (thumbv8m.main-none-eabihf target; there is no
// distinct thumbv8.1m.main rustc target-- ARMv8.1-M/Helium is
// selected via target-cpu/target-feature on top of v8-M
// mainline).  No peripherals beyond what compiling demands; not a
// real application.  See docs/src/projects/ars-toolhead-sensor/
// pinout.md gate G1.
#[cfg(feature = "g1-spike")]
#[rtic::app(device = embassy_stm32, dispatchers = [USART1])]
mod g1_spike {
    use defmt_rtt as _;
    use panic_probe as _;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        let _p = embassy_stm32::init(Default::default());

        defmt::info!("g0: nucleo-n657x0 alive");

        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
