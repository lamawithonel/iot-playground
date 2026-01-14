#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _; // global logger
use panic_probe as _;
use rtic::app;
use rtic_monotonics::stm32::prelude::*;

mod ccmram;
mod device_id;
mod eth;
mod network;
mod time;
mod tls_buffers;

stm32_tim2_monotonic!(Mono, 1_000_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [USART1, USART2, USART3])]
mod app {
    use super::*;
    use defmt::{info, warn};
    use embassy_futures::join::join3;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, Output, Pull, Speed};
    use embassy_stm32::peripherals;
    use embassy_stm32::rcc::{Hse, HseMode, LsConfig, LseConfig, LseMode};
    use embassy_stm32::rtc::{Rtc, RtcConfig};
    use embassy_stm32::spi::{self, Spi};
    use embassy_stm32::time::Hertz;

    use network::{manager, NetworkClient, SntpClient};

    type SpiPeripheral = embassy_stm32::Peri<'static, peripherals::SPI2>;
    type PinPB13 = embassy_stm32::Peri<'static, peripherals::PB13>;
    type PinPB15 = embassy_stm32::Peri<'static, peripherals::PB15>;
    type PinPB14 = embassy_stm32::Peri<'static, peripherals::PB14>;
    type PinPC6 = embassy_stm32::Peri<'static, peripherals::PC6>;
    type PinPC3 = embassy_stm32::Peri<'static, peripherals::PC3>;
    type PinPC2 = embassy_stm32::Peri<'static, peripherals::PC2>;
    type ExtiChannel = embassy_stm32::Peri<'static, peripherals::EXTI2>;
    type DmaTx = embassy_stm32::Peri<'static, peripherals::DMA1_CH4>;
    type DmaRx = embassy_stm32::Peri<'static, peripherals::DMA1_CH3>;

    struct NetworkPeripherals {
        spi: SpiPeripheral,
        sck: PinPB13,
        mosi: PinPB15,
        miso: PinPB14,
        cs: PinPC6,
        reset: PinPC3,
        int: PinPC2,
        exti: ExtiChannel,
        dma_tx: DmaTx,
        dma_rx: DmaRx,
    }

    // RNG interrupt binding for hardware random number generator
    embassy_stm32::bind_interrupts!(struct RngIrqs {
        RNG => embassy_stm32::rng::InterruptHandler<peripherals::RNG>;
    });

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Output<'static>,
    }

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        info!("IoT Playground starting...");

        // Adafruit Feather STM32F405: 12 MHz HSE, 32.768 kHz LSE (PC14/PC15)
        let mut config = embassy_stm32::Config::default();
        config.rcc.hse = Some(Hse {
            freq: Hertz(12_000_000),
            mode: HseMode::Oscillator,
        });

        // Configure PLL for system clock and RNG (48MHz required for RNG)
        // HSE (12 MHz) / PREDIV(6) = 2 MHz (PLL input)
        // 2 MHz * MUL(168) = 336 MHz (VCO)
        // VCO / DIVP(4) = 84 MHz (SYSCLK)
        // VCO / DIVQ(7) = 48 MHz (USB/RNG clock) ✓
        config.rcc.pll_src = embassy_stm32::rcc::PllSource::HSE;
        config.rcc.pll = Some(embassy_stm32::rcc::Pll {
            prediv: embassy_stm32::rcc::PllPreDiv::DIV6, // 12 MHz / 6 = 2 MHz
            mul: embassy_stm32::rcc::PllMul::MUL168,     // 2 MHz * 168 = 336 MHz (VCO)
            divp: Some(embassy_stm32::rcc::PllPDiv::DIV4), // 336 MHz / 4 = 84 MHz (SYSCLK)
            divq: Some(embassy_stm32::rcc::PllQDiv::DIV7), // 336 MHz / 7 = 48 MHz (RNG)
            divr: None,
        });
        config.rcc.sys = embassy_stm32::rcc::Sysclk::PLL1_P;
        config.rcc.ahb_pre = embassy_stm32::rcc::AHBPrescaler::DIV1; // 84 MHz
        config.rcc.apb1_pre = embassy_stm32::rcc::APBPrescaler::DIV2; // 42 MHz
        config.rcc.apb2_pre = embassy_stm32::rcc::APBPrescaler::DIV1; // 84 MHz

        config.rcc.ls = LsConfig {
            rtc: embassy_stm32::rcc::RtcClockSource::LSE,
            lsi: false,
            lse: Some(LseConfig {
                frequency: Hertz(32_768),
                mode: LseMode::Oscillator(embassy_stm32::rcc::LseDrive::MediumHigh),
            }),
        };

        let p = embassy_stm32::init(config);

        info!("System initialized with HSE (12MHz) and LSE (32.768kHz)");
        info!("PLL configured: SYSCLK=84MHz, PLLQ=48MHz for RNG");

        // TIM2 on APB1: timer clock = 2*APB1 when prescaler != 1
        // Default: APB1 = 42 MHz, TIM2 = 84 MHz
        let timer_clock_hz = 84_000_000;
        Mono::start(timer_clock_hz);
        info!("TIM2 monotonic timer initialized at 1 MHz");

        let rtc_config = RtcConfig::default();
        let rtc = Rtc::new(p.RTC, rtc_config);
        info!("Internal RTC initialized with LSE (32.768kHz, ±20-50ppm accuracy)");

        time::initialize_rtc(rtc);

        let led = Output::new(p.PC1, Level::High, Speed::Low);

        let net_periph = NetworkPeripherals {
            spi: p.SPI2,
            sck: p.PB13,
            mosi: p.PB15,
            miso: p.PB14,
            cs: p.PC6,
            reset: p.PC3,
            int: p.PC2,
            exti: p.EXTI2,
            dma_tx: p.DMA1_CH4,
            dma_rx: p.DMA1_CH3,
        };

        heartbeat::spawn().ok();
        network_task::spawn(net_periph, p.RNG).ok();

        (Shared {}, Local { led })
    }

    /// Heartbeat task
    #[task(priority = 1, local = [led])]
    async fn heartbeat(cx: heartbeat::Context) {
        info!("Heartbeat task started");
        loop {
            cx.local.led.set_high();
            Mono::delay(100.millis()).await;
            cx.local.led.set_low();
            Mono::delay(4900.millis()).await;
        }
    }

    /// Network task - orchestrates network stack and protocol clients
    ///
    /// Stack is !Send and must remain within this task.
    #[task(priority = 1)]
    async fn network_task(
        _cx: network_task::Context,
        periph: NetworkPeripherals,
        rng_periph: embassy_stm32::Peri<'static, peripherals::RNG>,
    ) -> ! {
        use embassy_net::{Config, StackResources};
        use static_cell::StaticCell;

        info!("Network task started");

        // Setup ethernet peripherals
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000); // 10 MHz for W5500

        let spi = Spi::new(
            periph.spi,
            periph.sck,
            periph.mosi,
            periph.miso,
            periph.dma_tx,
            periph.dma_rx,
            spi_config,
        );

        let cs = Output::new(periph.cs, Level::High, Speed::VeryHigh);
        let reset = Output::new(periph.reset, Level::High, Speed::Low);
        let int = ExtiInput::new(periph.int, periph.exti, Pull::Up);

        let eth_periph = eth::EthPeripherals {
            spi,
            cs,
            reset,
            int,
        };

        let mac_addr = [0x02, 0x00, 0x00, 0x12, 0x34, 0x56];
        let (device, w5500_runner) = eth::init_w5500(eth_periph, mac_addr).await;

        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let (stack, mut net_runner) = embassy_net::new(
            device,
            Config::dhcpv4(Default::default()),
            RESOURCES.init(StackResources::new()),
            0x1234_5678_u64,
        );
        info!("Network stack initialized with DHCP");

        let app_logic = async {
            manager::wait_for_config(&stack).await;
            run_clients(&stack, rng_periph).await;
        };

        join3(w5500_runner.run(), net_runner.run(), app_logic).await;
    }

    async fn run_clients(
        stack: &embassy_net::Stack<'static>,
        rng_periph: embassy_stm32::Peri<'static, peripherals::RNG>,
    ) -> ! {
        use embassy_stm32::rng::Rng;
        let mut sntp = SntpClient::new();

        // Initial SNTP sync
        info!("Initializing SNTP time synchronization with RTC (LSE)...");
        match sntp.run(stack).await {
            Ok(ts) => info!(
                "SNTP sync successful: {}.{:06} UTC (written to internal RTC)",
                ts.unix_secs, ts.micros
            ),
            Err(e) => warn!("SNTP initialization failed: {:?}", e),
        }

        // TLS 1.3 handshake test (Phase 1)
        info!("Testing TLS 1.3 handshake with 192.168.1.1:8883...");

        // Initialize hardware RNG just before TLS handshake
        info!("Initializing hardware RNG for TLS...");
        let mut rng = Rng::new(rng_periph, RngIrqs);
        info!("Hardware RNG initialized");

        let tls_config = network::tls::TlsClientConfig {
            server_name: "192.168.1.1",
            server_port: 8883,
            verify_server: false, // Phase 1: skip verification
        };
        let tls_client = network::tls::TlsClient::new(tls_config);
        match tls_client.test_handshake(stack, &mut rng).await {
            Ok(()) => info!("TLS 1.3 handshake test PASSED ✓"),
            Err(e) => warn!("TLS 1.3 handshake test FAILED: {:?}", e),
        }

        // Phase 2: MQTT Connection
        info!("Establishing MQTT connection over TLS 1.3...");
        let mqtt_config = network::MqttConfig {
            broker_host: "192.168.1.1",
            broker_port: 8883,
            keep_alive_secs: 60,
            clean_start: true,
        };
        let mut mqtt_client = network::MqttClient::new(mqtt_config);
        match mqtt_client.connect(stack, &mut rng).await {
            Ok(()) => info!("MQTT connection established ✓"),
            Err(e) => warn!("MQTT connection failed: {:?}", e),
        }

        info!("Network initialization complete - entering periodic sync loop");

        // Periodic resync using RTIC monotonic timer
        // Note: MQTT connection establishment is one-shot in current implementation
        // Future enhancement: maintain persistent connection with publish loop
        loop {
            Mono::delay(15.minutes()).await;
            info!("SNTP resync triggered");
            match sntp.run(stack).await {
                Ok(ts) => info!(
                    "SNTP sync successful: {}.{:06} UTC",
                    ts.unix_secs, ts.micros
                ),
                Err(e) => warn!("SNTP sync failed: {:?}", e),
            }
        }
    }

    /// RTIC idle task - WFI sleep mode when no tasks active
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        info!("Idle task started - entering WFI loop");
        loop {
            cortex_m::asm::wfi();
        }
    }
}
