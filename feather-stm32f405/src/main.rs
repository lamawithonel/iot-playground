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

// Configure SysTick as monotonic timer at 1kHz (1ms tick)
// Note: TIM2 at 1MHz would be preferred but rtic-monotonics STM32 support is complex
// Using SysTick for now as it's more straightforward and well-tested
systick_monotonic!(Mono, 1_000);

#[app(device = embassy_stm32, peripherals = true, dispatchers = [USART1, USART2, USART3])]
mod app {
    use super::*;
    use defmt::info;
    use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as SpiDeviceBus;
    use embassy_futures::join::join3;
    use embassy_net::{Stack, StackResources};
    use embassy_net_wiznet::chip::W5500;
    use embassy_stm32::gpio::{Level, Output, Speed};
    use embassy_stm32::mode::Async;
    use embassy_stm32::rcc::{Hse, HseMode, LsConfig, LseConfig, LseMode};
    use embassy_stm32::spi::Spi;
    use embassy_stm32::time::Hertz;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use static_cell::StaticCell;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: Output<'static>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
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

        // Initialize RTIC monotonic timer using SysTick at 1kHz
        Mono::start(cx.core.SYST, 168_000_000);

        // Initialize time module (RTC for wall-clock time)
        time::init_time_system(p.RTC);

        // Initialize Feather STM32F405 heartbeat LED (PC1)
        let led = Output::new(p.PC1, Level::High, Speed::Low);

        // Initialize network hardware and get bundle for network task
        let net_hardware = net::init_hardware(p.SPI2, p.PB13, p.PB15, p.PB14, p.PC6, p.PC3, p.PC2, p.EXTI2, p.DMA1_CH4, p.DMA1_CH3);

        heartbeat::spawn().ok();
        network_task::spawn(net_hardware).ok();
        sntp_resync_task::spawn().ok();
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
    async fn network_task(_cx: network_task::Context, hardware: net::NetworkHardware) -> ! {
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
        let (w5500_device, w5500_runner): (
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

        let (stack_val, mut net_runner) = embassy_net::new(
            w5500_device,
            config,
            RESOURCES.init(StackResources::new()),
            seed,
        );

        let stack = STACK.init(stack_val);

        info!("Network stack initialized");

        // Register stack for access by other tasks (SNTP resync)
        net::register_stack(stack);

        // --- D. Run Everything Concurrently ---
        // We need to run:
        // 1. W5500 runner (handles SPI/IRQ)
        // 2. Stack runner (handles TCP/IP state machine)
        // 3. Network monitor (from net module)

        // Run all three concurrently - never returns
        join3(
            w5500_runner.run(),
            net_runner.run(),
            net::run_network_monitor(stack),
        )
        .await;
    }

    /// SNTP periodic resync task
    ///
    /// Priority 1: Low priority background task
    /// Uses RTIC monotonic timer to wake every 15 minutes and sync time (SR-NET-007)
    #[task(priority = 1)]
    async fn sntp_resync_task(_cx: sntp_resync_task::Context) -> ! {
        time::run_sntp_resync_task().await
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
