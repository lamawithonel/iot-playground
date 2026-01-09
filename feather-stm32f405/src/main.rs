#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _; // global logger
use panic_probe as _;
use rtic::app;
use rtic_monotonics::stm32::prelude::*;

mod ccmram;
mod eth;
mod net;
mod network;
mod time;

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
    use rtic_sync::make_channel;

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

    /// Bundle of peripherals needed for W5500 network initialization
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

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Output<'static>,
    }

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        info!("IoT Playground starting...");

        // Configure embassy-stm32 with proper clock sources
        // Adafruit Feather STM32F405 has:
        // - 12 MHz HSE crystal for main system clock
        // - 32.768 kHz LSE crystal on PC14/PC15 for RTC
        let mut config = embassy_stm32::Config::default();

        // Configure HSE: 12 MHz external crystal
        config.rcc.hse = Some(Hse {
            freq: Hertz(12_000_000),
            mode: HseMode::Oscillator,
        });

        // Configure LSE: 32.768 kHz crystal for RTC
        config.rcc.ls = LsConfig {
            rtc: embassy_stm32::rcc::RtcClockSource::LSE,
            lsi: false,
            lse: Some(LseConfig {
                frequency: Hertz(32_768),
                mode: LseMode::Oscillator(embassy_stm32::rcc::LseDrive::MediumHigh),
            }),
        };

        // Initialize embassy-stm32 HAL with clock config
        let p = embassy_stm32::init(config);

        info!("System initialized with HSE (12MHz) and LSE (32.768kHz)");

        // Initialize TIM2 monotonic timer at 1 MHz
        // TIM2 is on APB1. When APB1 prescaler != 1, timer clock = 2*APB1
        // With default config: APB1 = 42 MHz, so TIM2 clock = 84 MHz
        let timer_clock_hz = 84_000_000;
        Mono::start(timer_clock_hz);
        info!("TIM2 monotonic timer initialized at 1 MHz");

        // Initialize internal RTC with LSE clock
        let rtc_config = RtcConfig::default();
        let rtc = Rtc::new(p.RTC, rtc_config);
        info!("Internal RTC initialized with LSE (32.768kHz, ±20-50ppm accuracy)");

        // Initialize time module with RTC
        time::initialize_rtc(rtc);

        // Initialize Feather STM32F405 heartbeat LED (PC1)
        let led = Output::new(p.PC1, Level::High, Speed::Low);

        // Bundle peripherals for network task
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

        // Create network message channel using rtic-sync
        let (net_sender, net_receiver) = make_channel!(net::NetworkMessage, 8);

        heartbeat::spawn().ok();
        network_task::spawn(net_periph, net_receiver).ok();

        // Start SNTP resync task
        sntp_resync::spawn(net_sender).ok();

        (Shared {}, Local { led })
    }

    /// Heartbeat task - blinks LED to show system is alive
    /// Priority 1: Low priority background task
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

    /// Network Actor Task - Modular RTIC-First Design
    ///
    /// Priority 1: Low priority background task
    ///
    /// This task coordinates the network layer initialization using the new modular structure:
    /// - eth module: Ethernet hardware (W5500)
    /// - net module: IP layer (DHCP, DNS, Stack)
    ///
    /// The Stack is kept here since it's !Send and can't cross task boundaries.
    #[task(priority = 1)]
    async fn network_task(
        _cx: network_task::Context,
        periph: NetworkPeripherals,
        mut receiver: rtic_sync::channel::Receiver<'static, net::NetworkMessage, 8>,
    ) -> ! {
        info!("Network task started");

        // --- A. Setup SPI and GPIO ---
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

        // --- B. Initialize Ethernet layer (eth module) ---
        let eth_periph = eth::EthPeripherals {
            spi,
            cs,
            reset,
            int,
        };
        let mac_addr = [0x02, 0x00, 0x00, 0x12, 0x34, 0x56];
        let (device, w5500_runner) = eth::init_w5500(eth_periph, mac_addr).await;

        // --- C. Initialize IP network layer ---
        let (stack, mut net_runner) = {
            use embassy_net::{Config, StackResources};
            use static_cell::StaticCell;

            let config = Config::dhcpv4(Default::default());
            let seed = 0x1234_5678_u64;

            static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
            embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed)
        };
        info!("Network stack initialized with DHCP");

        // --- D. Run network runners and application logic concurrently ---
        // The runners MUST be running for DHCP and network operations to work.
        // We structure this as three concurrent futures:
        // 1. W5500 hardware driver
        // 2. Embassy-net TCP/IP stack
        // 3. Application logic (DHCP wait, SNTP, message handling)

        let app_logic = async {
            // Wait for network to come up (DHCP) using net module
            net::wait_for_config(&stack).await;

            // Initialize SNTP time synchronization
            info!("Initializing SNTP time synchronization with RTC (LSE)...");
            match time::sntp::sync_sntp(&stack).await {
                Ok(ts) => {
                    info!(
                        "SNTP sync successful: {}.{:06} UTC (written to internal RTC)",
                        ts.unix_secs, ts.micros
                    );
                }
                Err(e) => {
                    warn!("SNTP initialization failed: {:?}", e);
                }
            }

            info!("Network initialization complete - processing messages");

            // Main message loop
            loop {
                match receiver.recv().await {
                    Ok(msg) => match msg {
                        net::NetworkMessage::SntpSync => {
                            info!("SNTP sync requested");
                            match time::sntp::sync_sntp(&stack).await {
                                Ok(ts) => {
                                    info!(
                                        "SNTP sync successful: {}.{:06} UTC",
                                        ts.unix_secs, ts.micros
                                    );
                                }
                                Err(e) => {
                                    warn!("SNTP sync failed: {:?}", e);
                                }
                            }
                        }
                    },
                    Err(_) => {
                        warn!("Network message channel closed");
                        Mono::delay(1.secs()).await;
                    }
                }
            }
        };

        // Run all three futures concurrently - they never return
        join3(w5500_runner.run(), net_runner.run(), app_logic).await;
    }

    /// SNTP periodic resync task (RTIC-First approach)
    ///
    /// Priority 2: Medium priority
    ///
    /// ## Interrupt-Driven Scheduling
    ///
    /// This task uses `Mono::delay(15.minutes()).await` for periodic scheduling.
    /// Unlike Embassy's polling timers, this allows the MCU to enter WFI sleep
    /// because the delay is implemented using the TIM2 hardware timer interrupt.
    ///
    /// ## Architecture
    ///
    /// - Sends SNTP sync request to network task via message channel
    /// - Network task owns the Stack (!Send) and performs actual sync
    /// - 15-minute interval per SR-NET-007 requirement
    #[task(priority = 3)]
    async fn sntp_resync(
        _cx: sntp_resync::Context,
        mut sender: rtic_sync::channel::Sender<'static, net::NetworkMessage, 8>,
    ) -> ! {
        loop {
            // Wait 15 minutes between syncs
            Mono::delay(15.minutes()).await;

            info!("SNTP resync task triggered");

            // Send SNTP sync request to network task
            if sender.send(net::NetworkMessage::SntpSync).await.is_err() {
                warn!("Failed to send SNTP sync request");
            }
        }
    }

    /// RTIC idle task - enables MCU sleep mode (WFI)
    ///
    /// Priority 0 (lowest): Runs when no other tasks are active
    /// Allows the MCU to enter Wait-For-Interrupt (WFI) mode to save power
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        info!("Idle task started - entering WFI loop");
        loop {
            // Wait For Interrupt - MCU enters sleep mode
            // Will wake up on any interrupt (timer, network, etc.)
            cortex_m::asm::wfi();
        }
    }
}
