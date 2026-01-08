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
mod net;
mod time;

systick_monotonic!(Mono, 1_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [USART1, USART2, USART3])]
mod app {
    use super::*;
    use defmt::info;
    use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as SpiDeviceBus;
    use embassy_futures::join::join4;
    use embassy_net::{Stack, StackResources};
    use embassy_net_wiznet::chip::W5500;
    use embassy_stm32::exti::ExtiInput;
    use embassy_stm32::gpio::{Level, Output, Pull, Speed};
    use embassy_stm32::mode::Async;
    use embassy_stm32::rcc::{Hse, HseMode, LsConfig, LseConfig, LseMode};
    use embassy_stm32::rtc::{Rtc, RtcConfig};
    use embassy_stm32::spi::{self, Spi};
    use embassy_stm32::time::Hertz;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use static_cell::StaticCell;

    /// Bundle of initialized hardware for W5500 network
    /// This contains hardware that has been set up in init() and is ready to use
    struct NetworkHardware {
        spi: Spi<'static, Async>,
        cs: Output<'static>,
        reset: Output<'static>,
        int: ExtiInput<'static>,
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

        // --- Network Hardware Setup ---
        info!("Setting up W5500 network hardware...");

        // Setup SPI for W5500
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000); // 10 MHz for W5500

        let spi = Spi::new(
            p.SPI2,     // SPI Bus 2
            p.PB13,     // SCK
            p.PB15,     // MOSI
            p.PB14,     // MISO
            p.DMA1_CH4, // TX DMA
            p.DMA1_CH3, // RX DMA
            spi_config,
        );

        // Setup GPIO pins
        let cs = Output::new(p.PC6, Level::High, Speed::VeryHigh);
        let mut reset = Output::new(p.PC3, Level::High, Speed::Low);
        let int = ExtiInput::new(p.PC2, p.EXTI2, Pull::Up);

        // Perform hardware reset
        info!("Performing W5500 hardware reset...");
        reset.set_low();
        // Note: Using blocking delay in init() is acceptable
        cortex_m::asm::delay(168_000); // ~1ms at 168 MHz
        reset.set_high();
        cortex_m::asm::delay(336_000); // ~2ms at 168 MHz

        info!("W5500 hardware setup complete");

        // Bundle initialized hardware for network task
        let net_hardware = NetworkHardware {
            spi,
            cs,
            reset,
            int,
        };

        heartbeat::spawn().ok();
        network_task::spawn(net_hardware).ok();
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
    /// Hardware setup is done in init(), this task initializes the W5500 driver and network stack
    /// This solves the !Send problem because Driver/Stack never cross task boundaries
    #[task(priority = 1)]
    async fn network_task(_cx: network_task::Context, hardware: NetworkHardware) -> ! {
        info!("Network task started - initializing W5500 driver...");

        // Hardware is already set up in init(), now initialize the driver

        // --- A. Create SPI Device (async version with Mutex wrapper) ---
        type SpiBusType = embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Spi<'static, Async>>;
        static SPI_BUS: StaticCell<SpiBusType> = StaticCell::new();
        let spi_bus = SPI_BUS.init(embassy_sync::mutex::Mutex::new(hardware.spi));
        let spi_device = SpiDeviceBus::new(spi_bus, hardware.cs);

        // --- B. Initialize W5500 Driver ---
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
        ) = embassy_net_wiznet::new(mac_addr, state, spi_device, hardware.int, hardware.reset)
            .await
            .unwrap();

        info!("W5500 initialized");

        // --- C. Initialize embassy-net Stack ---
        let config = embassy_net::Config::dhcpv4(Default::default());
        let seed = 0x1234_5678_u64; // TODO: Use proper RNG

        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        static STACK: StaticCell<Stack<'static>> = StaticCell::new();

        let (stack_val, mut runner2) =
            embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

        let stack = STACK.init(stack_val);

        info!("Network stack initialized");

        // --- D. Run Everything Concurrently ---
        // We need to run:
        // 1. W5500 runner (handles SPI/IRQ)
        // 2. Stack runner (handles TCP/IP state machine)
        // 3. Application logic (from net module)
        // 4. SNTP resync task (from net module)

        // Run all four concurrently - never returns
        join4(
            runner.run(),
            runner2.run(),
            net::run_app_logic(&stack),
            net::run_sntp_resync(&stack),
        )
        .await;
    }

    /// Example high-priority task that sends messages to network
    ///
    /// Priority 3: High priority (simulates interrupt-driven sensor)
    /// Demonstrates timestamp API usage for sensor data
    /// Timestamps are read from internal RTC between SNTP syncs
    #[task(priority = 3)]
    async fn frame_logger(_cx: frame_logger::Context) -> ! {
        let sender = net::network_sender();

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

            let msg = net::NetworkMessage::LogFrame { data: msg_str };

            // Send to network task (non-blocking in async context)
            sender.send(msg).await;

            info!("Sent RTC-timestamped frame to network task");
        }
    }
}
