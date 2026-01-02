#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;
use rtic::app;

mod network;
mod tls;

#[app(device = stm32_metapac, peripherals = false, dispatchers = [USART6])]
mod app {
    use defmt::{info, warn};
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
        info!("Phase 1: Initializing STM32F405 with Ethernet + TLS");

        // Configure clocks: HSE @ 12MHz -> PLL -> 168MHz SYSCLK
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
                divq: Some(PllQDiv::DIV7), // 336 MHz / 7 = 48 MHz (USB, SDIO)
                divr: None,
            });
            config.rcc.sys = Sysclk::PLL1_P;
            config.rcc.ahb_pre = AHBPrescaler::DIV1; // 168 MHz
            config.rcc.apb1_pre = APBPrescaler::DIV4; // 42 MHz (max)
            config.rcc.apb2_pre = APBPrescaler::DIV2; // 84 MHz (max)
        }
        let p = embassy_stm32::init(config);
        info!("Clock configuration complete: 168 MHz SYSCLK");

        // Initialize LED (PC1 - red LED on Feather)
        let led = Output::new(p.PC1, Level::High, Speed::Low);
        info!("LED initialized on PC1");

        // Configure TIM2 monotonic timer
        // TIM2 is on APB1. When APB1 prescaler != 1, timer clock = 2*APB1
        // APB1 = 42 MHz, so TIM2 clock = 84 MHz
        let timer_clock_hz = 84_000_000;
        Mono::start(timer_clock_hz);
        info!("Timer monotonic started at {} Hz", timer_clock_hz);

        // Report memory configuration
        info!("Memory configuration:");
        info!("  Flash:    1024 KB @ 0x08000000");
        info!("  Main RAM:  128 KB @ 0x20000000 (DMA-capable, stack here)");
        info!("  CCM RAM:    64 KB @ 0x10000000 (CPU-only, zero-wait, for data)");

        // Schedule network and TLS initialization
        network_task::spawn().ok();
        heartbeat::spawn().ok();

        (Shared {}, Local { led })
    }

    /// Heartbeat task - blinks LED to show system is alive
    #[task(local = [led])]
    async fn heartbeat(cx: heartbeat::Context) {
        info!("Heartbeat task started");
        loop {
            cx.local.led.set_low();
            Mono::delay(100.millis()).await;
            cx.local.led.set_high();
            Mono::delay(900.millis()).await;
        }
    }

    /// Network initialization and management task
    #[task]
    async fn network_task(_cx: network_task::Context) {
        info!("Network task started");

        // TODO: Initialize SPI for W5500
        // TODO: Initialize W5500 Ethernet controller
        // TODO: Setup smoltcp interface
        // TODO: Start DHCP client

        warn!("Network initialization not yet implemented");
        warn!("Phase 1 goals:");
        warn!("  1. W5500 SPI communication");
        warn!("  2. smoltcp TCP/IP stack");
        warn!("  3. DHCP IP acquisition");
        warn!("  4. Basic TLS 1.3 handshake");

        loop {
            Mono::delay(5000.millis()).await;
            info!("Network task tick (not yet implemented)");
        }
    }
}
