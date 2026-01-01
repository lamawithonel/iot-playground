#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;
use rtic::app;

#[app(device = stm32_metapac, peripherals = false, dispatchers = [USART6])]
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
        // Configure timers to use the HSE clock source and max out frequencies.
        let mut config = embassy_stm32::Config::default();
        {
            use embassy_stm32::rcc::*;
            config.rcc.hse = Some(Hse {
                freq: embassy_stm32::time::Hertz(12_000_000),
                mode: HseMode::Oscillator,
            });
            config.rcc.pll_src = PllSource::HSE;
            config.rcc.pll = Some(Pll {
                prediv: PllPreDiv::DIV6,   // 12 MHz / 6 = 2 MHz
                mul: PllMul::MUL168,       // 2 MHz * 168 = 336 MHz
                divp: Some(PllPDiv::DIV2), // 336 MHz / 2 = 168 MHz (sysclk)
                divq: Some(PllQDiv::DIV7), // 336 Mhz / 7 = 48 MHz (USB, SDIO)
                divr: None,
            });
            config.rcc.sys = Sysclk::PLL1_P;
            config.rcc.ahb_pre = AHBPrescaler::DIV1; // 168 MHz
            config.rcc.apb1_pre = APBPrescaler::DIV4; // 42 MHz (max for APB1)
            config.rcc.apb2_pre = APBPrescaler::DIV2; // 84 MHz (max for APB2)
        }
        let p = embassy_stm32::init(config);
        info!("Hello world!");

        let led = Output::new(p.PC1, Level::High, Speed::Low);
        info!("LED initialized on PC1 (red)");

        // TIM2 is on APB1.  When APB1 prescaler != 1, timer clock = 2*APB1
        // APB1 = 42 MHz, so TIM2 clock = 84 MHz
        let timer_clock_hz = 84_000_000;
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
