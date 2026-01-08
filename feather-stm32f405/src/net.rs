#![deny(unsafe_code)]
#![deny(warnings)]
//! Network module: W5500 Ethernet with embassy-net
//!
//! This module contains all network-related functionality:
//! - Hardware initialization
//! - Network stack management
//! - Application logic

use embassy_net::Stack;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals;
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use heapless::String;

use crate::{ccmram, time};

/// Bundle of initialized hardware for W5500 network
/// This contains hardware that has been set up in init() and is ready to use
pub struct NetworkHardware {
    pub spi: Spi<'static, Async>,
    pub cs: Output<'static>,
    pub reset: Output<'static>,
    pub int: ExtiInput<'static>,
}

/// Initialize W5500 network hardware
/// Called from init() to set up SPI, GPIO, and perform hardware reset
pub fn init_hardware(
    spi2: peripherals::SPI2,
    pb13: peripherals::PB13,
    pb15: peripherals::PB15,
    pb14: peripherals::PB14,
    pc6: peripherals::PC6,
    pc3: peripherals::PC3,
    pc2: peripherals::PC2,
    exti2: peripherals::EXTI2,
    dma1_ch4: peripherals::DMA1_CH4,
    dma1_ch3: peripherals::DMA1_CH3,
) -> NetworkHardware {
    use defmt::info;

    info!("Setting up W5500 network hardware...");

    // Setup SPI for W5500
    let mut spi_config = spi::Config::default();
    spi_config.frequency = Hertz(10_000_000); // 10 MHz for W5500

    let spi = Spi::new(
        spi2,   // SPI Bus 2
        pb13,   // SCK
        pb15,   // MOSI
        pb14,   // MISO
        dma1_ch4, // TX DMA
        dma1_ch3, // RX DMA
        spi_config,
    );

    // Setup GPIO pins
    let cs = Output::new(pc6, Level::High, Speed::VeryHigh);
    let mut reset = Output::new(pc3, Level::High, Speed::Low);
    let int = ExtiInput::new(pc2, exti2, Pull::Up);

    // Perform hardware reset
    info!("Performing W5500 hardware reset...");
    reset.set_low();
    // Note: Using blocking delay in init() is acceptable
    cortex_m::asm::delay(168_000); // ~1ms at 168 MHz
    reset.set_high();
    cortex_m::asm::delay(336_000); // ~2ms at 168 MHz

    info!("W5500 hardware setup complete");

    NetworkHardware {
        spi,
        cs,
        reset,
        int,
    }
}

/// Register the network stack for access by other tasks
/// This stores the stack pointer in a safe, thread-safe location (ccmram module)
pub fn register_stack(stack: &'static Stack<'static>) {
    ccmram::set_network_stack(stack);
}

/// Messages that can be sent to the network task
#[derive(Clone, Debug)]
pub enum NetworkMessage {
    /// Log a message (for demonstration)
    LogFrame { data: String<128> },
}

/// Channel for sending messages to network task
/// Using CriticalSectionRawMutex makes it safe across all RTIC priorities
pub static NETWORK_CHANNEL: Channel<CriticalSectionRawMutex, NetworkMessage, 8> = Channel::new();

/// Get a sender for the network channel (can be called from any task)
pub fn network_sender() -> Sender<'static, CriticalSectionRawMutex, NetworkMessage, 8> {
    NETWORK_CHANNEL.sender()
}

/// Get a receiver for the network channel (used inside network task)
pub fn network_receiver() -> Receiver<'static, CriticalSectionRawMutex, NetworkMessage, 8> {
    NETWORK_CHANNEL.receiver()
}

/// Run the network monitor task
/// This handles DHCP configuration, IP logging, message processing, and statistics
pub async fn run_network_monitor(stack: &Stack<'static>) {
    use defmt::info;

    let receiver = network_receiver();

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
    match time::initialize_time(stack).await {
        Ok(ts) => {
            info!(
                "SNTP sync successful: {}.{:06} UTC (written to internal RTC)",
                ts.unix_secs, ts.micros
            );
        }
        Err(e) => {
            defmt::warn!("SNTP initialization failed: {:?}", e);
        }
    }

    // Main event loop
    let mut stats_timer = embassy_time::Ticker::every(embassy_time::Duration::from_secs(10));

    loop {
        // Use select to handle both channel messages and periodic stats
        embassy_futures::select::select(receiver.receive(), stats_timer.next()).await;

        // Check for messages
        if let Ok(msg) = receiver.try_receive() {
            match msg {
                NetworkMessage::LogFrame { data } => {
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
}
