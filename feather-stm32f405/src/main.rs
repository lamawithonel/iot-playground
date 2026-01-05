#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use defmt_rtt as _; // global logger
use heapless::String;
use panic_probe as _;
use rtic::app;
use rtic_monotonics::systick::prelude::*;

mod ccmram;
mod network;
mod time;

systick_monotonic!(Mono, 1_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [USART1, USART2, USART3])]
mod app {
    use super::*;
    use defmt::info;
    use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as SpiDeviceBus;
    use embassy_futures::join::join4;
    use embassy_net::StackResources;
    use embassy_net_wiznet::chip::W5500;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, Output, Pull, Speed};
    use embassy_stm32::mode::Async;
    use embassy_stm32::peripherals;
    use embassy_stm32::rcc::{Hse, HseMode, LsConfig, LseConfig, LseMode};
    use embassy_stm32::rtc::{Rtc, RtcConfig};
    use embassy_stm32::spi::{self, Spi};
    use embassy_stm32::time::Hertz;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_time::Timer;
    use static_cell::StaticCell;

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
    fn init(cx: init::Context) -> (Shared, Local) {
        info!("IoT Playground starting...");

        // Initialize RTIC monotonic
        Mono::start(cx.core.SYST, 168_000_000);

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

        heartbeat::spawn().ok();
        network_task::spawn(net_periph).ok();
        frame_logger::spawn().ok();

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

    /// Network Actor Task - Init-Inside-Task Pattern
    ///
    /// Priority 1: Low priority background task
    /// All network resources are constructed INSIDE this task
    /// This solves the !Send problem because nothing crosses task boundaries
    #[task(priority = 1)]
    async fn network_task(_cx: network_task::Context, periph: NetworkPeripherals) -> ! {
        info!("Network task started - initializing W5500...");

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
        let mut reset = Output::new(periph.reset, Level::High, Speed::Low);
        let int = ExtiInput::new(periph.int, periph.exti, Pull::Up);

        // Hardware reset
        info!("Performing W5500 hardware reset...");
        reset.set_low();
        Timer::after_millis(1).await;
        reset.set_high();
        Timer::after_millis(2).await;

        // --- B. Create SPI Device (async version with Mutex wrapper) ---
        type SpiBusType = embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Spi<'static, Async>>;
        static SPI_BUS: StaticCell<SpiBusType> = StaticCell::new();
        let spi_bus = SPI_BUS.init(embassy_sync::mutex::Mutex::new(spi));
        let spi_device = SpiDeviceBus::new(spi_bus, cs);

        // --- C. Initialize W5500 Driver ---
        let mac_addr = [0x02, 0x00, 0x00, 0x12, 0x34, 0x56];

        info!(
            "MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5]
        );

        // State for W5500 driver
        static STATE: StaticCell<embassy_net_wiznet::State<8, 8>> = StaticCell::new();
        let state = STATE.init(embassy_net_wiznet::State::<8, 8>::new());

        // Create W5500 device and runner
        // NOTE: This returns (Device, Runner) - both are !Send
        // The last parameter is the RESET PIN (OutputPin), not the chip type
        // Chip type W5500 is inferred from context
        let (device, runner): (
            embassy_net_wiznet::Device<'_>,
            embassy_net_wiznet::Runner<'_, W5500, _, _, _>,
        ) = embassy_net_wiznet::new(mac_addr, state, spi_device, int, reset)
            .await
            .unwrap();

        info!("W5500 initialized");

        // --- D. Initialize embassy-net Stack ---
        let config = embassy_net::Config::dhcpv4(Default::default());
        let seed = 0x1234_5678_u64; // TODO: Use proper RNG

        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let (stack, mut runner2) =
            embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

        info!("Network stack initialized");

        // --- E. Run Everything Concurrently ---
        // We need to run:
        // 1. W5500 runner (handles SPI/IRQ)
        // 2. Stack runner (handles TCP/IP state machine)
        // 3. Application logic
        // 4. SNTP resync task (after initial sync)

        let app_logic = async {
            let receiver = network::network_receiver();

            // Wait for network to come up
            stack.wait_config_up().await;
            info!("Network is UP!");

            // Log IP address
            if let Some(config) = stack.config_v4() {
                let ip = config.address.address();
                let octets = ip.octets();
                info!(
                    "IP: {}.{}.{}.{}",
                    octets[0], octets[1], octets[2], octets[3]
                );

                if let Some(gateway) = config.gateway {
                    let gw_octets = gateway.octets();
                    info!(
                        "Gateway: {}.{}.{}.{}",
                        gw_octets[0], gw_octets[1], gw_octets[2], gw_octets[3]
                    );
                }
            }

            // Initialize SNTP time synchronization (SR-NET-006)
            // This will write the time to internal RTC (clocked by LSE)
            info!("Initializing SNTP time synchronization with RTC (LSE)...");
            match time::initialize_time(&stack).await {
                Ok(ts) => {
                    info!(
                        "SNTP sync successful: {}.{:03} UTC (written to internal RTC)",
                        ts.unix_secs, ts.millis
                    );
                }
                Err(e) => {
                    defmt::warn!("SNTP initialization failed: {:?}", e);
                }
            }

            // Main event loop
            let mut stats_timer =
                embassy_time::Ticker::every(embassy_time::Duration::from_secs(10));

            loop {
                // Use select to handle both channel messages and periodic stats
                embassy_futures::select::select(receiver.receive(), stats_timer.next()).await;

                // Check for messages
                if let Ok(msg) = receiver.try_receive() {
                    match msg {
                        network::NetworkMessage::LogFrame { data } => {
                            info!("Received frame: {}", data.as_str());
                        }
                    }
                }

                // Log network statistics
                if let Some(config) = stack.config_v4() {
                    let ip = config.address.address();
                    let octets = ip.octets();
                    info!(
                        "IP: {}.{}.{}.{}",
                        octets[0], octets[1], octets[2], octets[3]
                    );
                }
            }
        };

        // SNTP periodic resync task (SR-NET-007: 15-minute interval)
        // Runs concurrently with main app logic
        // Updates internal RTC every 15 minutes to maintain accurate time
        let sntp_resync = async {
            // Wait for initial sync to complete
            stack.wait_config_up().await;
            Timer::after_secs(30).await; // Give initial sync time to complete

            info!("SNTP resync task started (15-minute interval)");
            time::start_resync_task(&stack).await // This function never returns
        };

        // Run all four concurrently - never returns
        join4(runner.run(), runner2.run(), app_logic, sntp_resync).await;
    }

    /// Example high-priority task that sends messages to network
    ///
    /// Priority 3: High priority (simulates interrupt-driven sensor)
    /// Demonstrates timestamp API usage for sensor data
    /// Timestamps are read from internal RTC between SNTP syncs
    #[task(priority = 3)]
    async fn frame_logger(_cx: frame_logger::Context) -> ! {
        let sender = network::network_sender();

        loop {
            // Simulate periodic data
            embassy_time::Timer::after_secs(5).await;

            // Get timestamp from internal RTC (SR-NET-007 requirement)
            let timestamp = time::get_timestamp();

            // Create message with timestamp
            let mut msg_str = String::new();
            if timestamp.unix_secs == 0 {
                // Time not yet synced - use local monotonic counter
                let _ = core::fmt::write(
                    &mut msg_str,
                    format_args!("Frame at {} ms (RTC not synced)", Mono::now().ticks()),
                );
            } else {
                // Time is synced - use Unix timestamp from RTC
                let _ = core::fmt::write(
                    &mut msg_str,
                    format_args!(
                        "Frame at {} UTC (from internal RTC/LSE)",
                        timestamp.unix_secs
                    ),
                );
            }

            let msg = network::NetworkMessage::LogFrame { data: msg_str };

            // Send to network task (non-blocking in async context)
            sender.send(msg).await;

            info!("Sent RTC-timestamped frame to network task");
        }
    }
}
