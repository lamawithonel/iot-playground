//! NUCLEO-H753ZI bring-up: RTIC 2.x heartbeat blink.
//!
//! Phase 1 scaffold for the ARS prototyping / net trigger board
//! (see `AGENTS.md`).  Proves flash, RTT logging, TIM2 monotonic
//! scheduling, and the idle/WFI path before any real peripherals
//! are added.

#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _; // global logger
use panic_probe as _;
use rtic::app;
use rtic_monotonics::stm32::prelude::*;

stm32_tim2_monotonic!(Mono, 1_000_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [UART4, UART5, UART7])]
mod app {
    use super::*;
    use defmt::info;
    use embassy_stm32::gpio::{Level, Output, Speed};

    /// Resources shared across tasks (none yet in phase 1).
    #[shared]
    struct Shared {}

    /// Resources owned by a single task.
    #[local]
    struct Local {
        /// LD1 (green, PB0), active-high (UM2407 Rev 6 p.27 Sec 7.6.1).
        led: Output<'static>,
    }

    /// RTIC init: clocks, TIM2 monotonic, LED, and heartbeat spawn.
    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        info!("nucleo-h753zi: init");

        // Stock default: HSI 64 MHz, HSE untouched.  The board's HSE
        // is an 8 MHz ST-LINK MCO with no crystal populated (UM2407
        // Rev 6 p.25-26 Sec 7.5.1); any crystal-mode HSE config
        // hangs before the first log line.  Explicit HSE-bypass +
        // PLL1 clock config is a planned follow-up, not bring-up.
        let p = embassy_stm32::init(embassy_stm32::Config::default());

        Mono::start(64_000_000);

        let led = Output::new(p.PB0, Level::Low, Speed::Low);

        heartbeat::spawn().ok();

        (Shared {}, Local { led })
    }

    /// Heartbeat task: blinks LD1 (100 ms on / 900 ms off) and logs
    /// a running count once per period.
    #[task(priority = 1, local = [led, count: u32 = 0])]
    async fn heartbeat(cx: heartbeat::Context) -> ! {
        info!("heartbeat: task started");
        loop {
            cx.local.led.set_high();
            Mono::delay(100.millis()).await;
            cx.local.led.set_low();
            Mono::delay(900.millis()).await;

            info!("heartbeat: {}", *cx.local.count);
            *cx.local.count = cx.local.count.wrapping_add(1);
        }
    }

    /// RTIC idle task -- WFI sleep mode when no tasks are active.
    ///
    /// The DSB ensures all pending memory accesses complete before
    /// the CPU enters sleep, per standard ARM Cortex-M practice
    /// (mirrors `boards/feather-stm32f405/src/main.rs`).
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        info!("idle: entering WFI loop");
        loop {
            cortex_m::asm::dsb();
            cortex_m::asm::wfi();
        }
    }
}
