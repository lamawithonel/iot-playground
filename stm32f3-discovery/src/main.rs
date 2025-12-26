#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;
use rtic::app;

#[app(device = stm32_metapac, peripherals = false, dispatchers = [COMP7])]
mod app {
    use defmt::info;
    use embassy_stm32::gpio::{Level, Output, Speed};
    use rtic_monotonics::stm32::prelude::*;

    stm32_tim2_monotonic!(Mono, 1_000_000);

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Output<'static>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let mut config = embassy_stm32::Config::default();
        {
            use embassy_stm32::rcc::*;
            config.rcc.hsi = true;
            config.rcc.sys = Sysclk::HSI;
            config.rcc.pll = None;
            config.rcc.ahb_pre = AHBPrescaler::DIV1;
            config.rcc.apb1_pre = APBPrescaler::DIV1;
            config.rcc.apb2_pre = APBPrescaler::DIV1;
        }
        let p = embassy_stm32::init(config);
        info!("Hello world!");

        let led = Output::new(p.PE9, Level::High, Speed::Low);
        info!("LED initialized on PE9 (LD3 red)");

        let timer_clock_hz = 8_000_000;
        Mono::start(timer_clock_hz);

        // Schedule the blinking task
        blink::spawn().ok();

        (Shared {}, Local { led })
    }

    #[task(local = [led])]
    async fn blink(cx: blink::Context) {
        info!("starting blink()");
        loop {
            info!("attempting to set high");
            cx.local.led.set_high();
            info!("set high");
            Mono::delay(125.millis()).await;
            info!("timer await returned");

            info!("attempting to set low");
            cx.local.led.set_low();
            info!("set low");
            Mono::delay(125.millis()).await;
            info!("timer await returned");
        }
    }
}
