//! ARS toolhead sensor-- NUCLEO-N657X0-Q board crate.
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
//! flow of record).  `g1-spike` now also carries the phase-1
//! implementation (the `app` module below)-- `button_task` and the
//! `AMP_MUTE_N`/`AMP_I2C` inits are real, interrupt-driven code;
//! `capture`/`sweep_engine` stay empty hardware-task shells (see
//! their own doc comments) pending ADC1/TIM1 landing in
//! embassy-stm32 for this chip.
//!
//! See `pins.rs` for the pin map of record in code form.

#![no_std]
#![no_main]
#![deny(warnings)]
#![deny(unsafe_code)]

#[cfg(not(feature = "g1-spike"))]
compile_error!("scaffold only -- see boards/nucleo-n657x0/README.md");

// Plain `&str` consts, no embassy deps-- compiles in every feature
// configuration.  See the module doc for which consts the `app`
// module below actually consumes.
mod pins;

// ── Gate G1 / phase-1 RTIC app ───────────────────────────────────
// Proves embassy-stm32 + rtic 2.x + rtic-monotonics +
// defmt/defmt-rtt/panic-probe compile and run for the Cortex-M55
// (thumbv8m.main-none-eabihf target; there is no distinct
// thumbv8.1m.main rustc target-- ARMv8.1-M/Helium is selected via
// target-cpu/target-feature on top of v8-M mainline), and hosts the
// phase-1 task topology.  `capture`/`sweep_engine` are dormant
// hardware-task shells (empty bodies, no transfer ever configured)
// because ADC1 and TIM1 have no embassy-stm32 0.6.0 peripheral
// singleton for `stm32n657x0` in this crate's locked dependency
// version-- see their doc comments below.  See
// docs/src/projects/ars-toolhead-sensor/pinout.md gates G1-G4.
#[cfg(feature = "g1-spike")]
#[rtic::app(device = embassy_stm32, dispatchers = [USART1, USART2])]
mod app {
    use core::sync::atomic::{AtomicU32, Ordering};

    use defmt_rtt as _;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, OutputOpenDrain, Pull, Speed};
    use embassy_stm32::i2c::I2c;
    use embassy_stm32::mode::Async;
    use embassy_stm32::peripherals;
    use embassy_stm32::time::Hertz;
    use embassy_stm32::Peri;
    use panic_probe as _;
    use rtic_monotonics::systick::prelude::*;

    use super::pins;

    systick_monotonic!(Mono, 1_000);

    /// Count of `USER_BTN` (PC13/EXTI13) falling-edge presses
    /// observed since boot.
    ///
    /// Incremented only by [`button_task`]; nothing reads it back
    /// yet in phase-1.  A future telemetry task is the intended
    /// consumer, per `docs/src/projects/ars-toolhead-sensor/
    /// README.md`'s planned `telemetry` task.
    static USER_BTN_EVENTS: AtomicU32 = AtomicU32::new(0);

    // USER_BTN (PC13/EXTI13) interrupt binding for embassy's async
    // EXTI driver.  A raw RTIC `#[task(binds = EXTI13)]` hardware
    // task cannot safely coexist with this: embassy's `ExtiInput`
    // owns pending-bit clearing and waker registration for the
    // vector internally, and duplicating that at the PAC level
    // would need `unsafe`, forbidden in this crate.  `button_task`
    // below is still fully interrupt-driven (no polling)-- it just
    // isn't a literal `#[task(binds = ...)]` hardware task.  Mirrors
    // pinout.md's own USER_BTN rationale
    // (`boards/feather-stm32f405/src/counting_exti.rs`).
    embassy_stm32::bind_interrupts!(struct ExtiIrqs {
        EXTI13 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI13>;
    });

    /// RTIC shared resources.
    ///
    /// Empty in phase-1: no state is contended across task
    /// priorities yet.
    #[shared]
    struct Shared {}

    /// RTIC local (task-exclusive) resources.
    #[local]
    struct Local {
        /// AMP_MUTE_N (PD0), open-drain GPIO output, driven low
        /// (muted) at boot.  Open-drain per pinout.md: the surveyed
        /// facts do not identify which rail the amp board's R15
        /// pull-up returns to, so the MCU must never actively drive
        /// the net high.  Owned by [`sweep_engine`]: it is the
        /// natural long-term owner since a sweep should stay muted
        /// between plays and unmuted only while one is active.  Pin
        /// source: pinout.md ("Amplifier mute control; drive LOW to
        /// mute").
        amp_mute_n: OutputOpenDrain<'static>,
        /// USER_BTN (PC13/EXTI13), async embassy EXTI input, owned
        /// by [`button_task`].  `Pull::Up` + falling-edge assumes
        /// idle-high/pressed-low wiring for the on-board B1 button;
        /// bench-unverified.
        user_btn: ExtiInput<'static, Async>,
        /// GPDMA1 channel 0, reserved for the ADC1 capture
        /// transfer.  Unconfigured: ADC1 has no embassy-stm32
        /// 0.6.0 peripheral singleton for `stm32n657x0` (no
        /// register-block metadata in `stm32-metapac` 21.0.0).
        /// See [`capture`].
        capture_dma_ch: Peri<'static, peripherals::GPDMA1_CH0>,
        /// GPDMA1 channel 1, reserved for the TIM1 duty-stream
        /// transfer.  Unconfigured: TIM1 has no embassy-stm32
        /// 0.6.0 peripheral singleton for `stm32n657x0` either
        /// (only TIM9, the time driver, is generated for this
        /// chip).  See [`sweep_engine`].
        sweep_dma_ch: Peri<'static, peripherals::GPDMA1_CH1>,
    }

    /// RTIC init.  Claims PD0/PC13/EXTI13/I2C1(PH9,PC1)/
    /// GPDMA1_CH0/GPDMA1_CH1, starts the SysTick monotonic, and
    /// spawns [`analysis`] and [`button_task`].
    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let p = embassy_stm32::init(Default::default());

        // Sysclk value is a placeholder: `embassy_stm32::init` is
        // called with `Default::default()` here, not an explicit
        // RCC config, so the true post-boot sysclk is unconfirmed.
        // Monotonic timing accuracy is not a phase-1 concern (no
        // on-device work tonight-- compile + clippy only).
        Mono::start(cx.core.SYST, 200_000_000);

        defmt::info!("g1: nucleo-n657x0 phase-1 alive");

        // AMP_MUTE_N (PD0): drive low at boot so the amp stays
        // muted before any real audio content exists.  No AF; GPIO
        // output, open-drain per pinout.md (the amp board's R15
        // pull-up rail is unidentified, so the MCU must not drive
        // the net high itself).
        defmt::info!("AMP_MUTE_N ({}): muted low", pins::AMP_MUTE_N);
        let amp_mute_n = OutputOpenDrain::new(p.PD0, Level::Low, Speed::Low);

        // USER_BTN (PC13/EXTI13): async ExtiInput-- see the
        // `ExtiIrqs` binding comment above for why this isn't a raw
        // RTIC hardware-task bind.
        defmt::info!("USER_BTN ({}): EXTI13 armed", pins::USER_BTN);
        let user_btn = ExtiInput::new(p.PC13, p.EXTI13, Pull::Up, ExtiIrqs);

        // AMP_I2C (I2C1, PH9 SCL / PC1 SDA): blocking init only--
        // no transactions attempted.  Gate G4 (MAX9744 present at
        // 0x4B) is unconfirmed, so real ownership/storage for
        // volume-control writes is deferred to G4; the instance is
        // constructed and dropped here purely to prove the
        // peripheral claims and clock enable compile and run.
        defmt::info!(
            "AMP_I2C: I2C1 SCL={} SDA={} (init-only)",
            pins::AMP_I2C_SCL,
            pins::AMP_I2C_SDA
        );
        let mut i2c_config = embassy_stm32::i2c::Config::default();
        i2c_config.frequency = Hertz(100_000);
        let _amp_i2c = I2c::new_blocking(p.I2C1, p.PH9, p.PC1, i2c_config);

        // GPDMA1_CH0/CH1 are claimed here and handed to `capture`/
        // `sweep_engine` below via `local`; neither has a transfer
        // configured yet (see their doc comments).
        analysis::spawn().ok();
        button_task::spawn().ok();

        (
            Shared {},
            Local {
                amp_mute_n,
                user_btn,
                capture_dma_ch: p.GPDMA1_CH0,
                sweep_dma_ch: p.GPDMA1_CH1,
            },
        )
    }

    /// ADC capture GPDMA hardware task-- bound to GPDMA1 channel 0
    /// completion, owns [`Local::capture_dma_ch`].
    ///
    /// Empty body: nothing configures a transfer on this channel,
    /// so this interrupt never fires in phase-1.  Blocked on gate
    /// G1 runtime (confirming GPDMA linked-list/circular-capture
    /// mode) and gate G2 (adaptation-amp capture quality), and more
    /// fundamentally on ADC1 itself: `stm32-metapac` 21.0.0 has
    /// `registers: None` for both the "ADC1" and "ADC12_COMMON"
    /// peripheral entries for this chip, so `embassy_stm32::adc`
    /// cannot target ADC1 here in any mode, not even blocking
    /// calibration-only init.  See pinout.md gates G1-G2.
    #[task(binds = GPDMA1_CHANNEL0, local = [capture_dma_ch], priority = 3)]
    fn capture(_cx: capture::Context) {}

    /// TIM1 duty-stream GPDMA hardware task-- bound to GPDMA1
    /// channel 1 completion, owns [`Local::sweep_dma_ch`] and
    /// [`Local::amp_mute_n`].
    ///
    /// Empty body, same dormant status as [`capture`]: TIM1 is
    /// entirely absent as a peripheral singleton for `stm32n657x0`
    /// in embassy-stm32 0.6.0 (only TIM9, already claimed as the
    /// time driver, is generated for this chip)-- there is no PWM
    /// API surface to build a carrier skeleton against yet.  See
    /// pinout.md gate G3.
    #[task(binds = GPDMA1_CHANNEL1, local = [sweep_dma_ch, amp_mute_n], priority = 3)]
    fn sweep_engine(_cx: sweep_engine::Context) {}

    /// USER_BTN (PC13/EXTI13) press counter.
    ///
    /// Awaits falling edges on the async `ExtiInput`
    /// ([`Local::user_btn`])-- interrupt-driven throughout, no
    /// polling.  Mirrors `feather-stm32f405`'s `counting_exti.rs`
    /// telemetry style, per pinout.md's USER_BTN rationale.
    #[task(local = [user_btn], priority = 1)]
    async fn button_task(cx: button_task::Context) {
        loop {
            cx.local.user_btn.wait_for_falling_edge().await;
            let n = USER_BTN_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
            defmt::info!("user_btn: press {}", n);
        }
    }

    /// FFT / feature-extraction task.  Spawned once from [`init`];
    /// empty in phase-1.
    ///
    /// The planned hand-off is an `rtic_sync::channel::Receiver<
    /// CaptureWindow, 2>` fed by [`capture`], mirroring
    /// `feather-stm32f405`'s `SENSOR_CHANNEL_CAP = 2` precedent.
    /// Not instantiated yet: `capture` has no real producer (see
    /// its doc comment), and holding an unused `Sender` trips
    /// `#![deny(warnings)]`.  Lands together with the `rtic-sync`
    /// dependency once gate G1 runtime and G2 close.
    #[task(priority = 2)]
    async fn analysis(_cx: analysis::Context) {}

    /// RTIC idle task-- WFI sleep when no task is runnable.
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
