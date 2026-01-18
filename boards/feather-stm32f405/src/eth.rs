#![deny(unsafe_code)]
#![deny(warnings)]
//! Ethernet hardware layer module

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as SpiDeviceBus;
use embassy_net_wiznet::chip::W5500;
use embassy_net_wiznet::{Device, Runner};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use static_cell::StaticCell;

/// Type alias for the W5500 device used with embassy-net
#[allow(dead_code)]
pub type W5500Device = Device<'static>;

/// Ethernet peripherals bundle
pub struct EthPeripherals<'a> {
    pub spi: Spi<'a, Async>,
    pub cs: Output<'a>,
    pub reset: Output<'a>,
    pub int: ExtiInput<'a>,
}

/// Initialize the W5500 Ethernet hardware
///
/// Returns device and runner. Runner must be continuously polled for device operation.
pub async fn init_w5500(
    periph: EthPeripherals<'static>,
    mac_addr: [u8; 6],
) -> (
    Device<'static>,
    Runner<
        'static,
        W5500,
        SpiDeviceBus<'static, CriticalSectionRawMutex, Spi<'static, Async>, Output<'static>>,
        ExtiInput<'static>,
        Output<'static>,
    >,
) {
    let EthPeripherals {
        spi,
        cs,
        mut reset,
        int,
    } = periph;

    info!("Performing W5500 hardware reset...");
    reset.set_low();
    embassy_time::Timer::after_millis(1).await;
    reset.set_high();
    embassy_time::Timer::after_millis(2).await;

    type SpiBusType = embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Spi<'static, Async>>;
    static SPI_BUS: StaticCell<SpiBusType> = StaticCell::new();
    let spi_bus = SPI_BUS.init(embassy_sync::mutex::Mutex::new(spi));
    let spi_device = SpiDeviceBus::new(spi_bus, cs);

    info!(
        "MAC address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5]
    );

    static STATE: StaticCell<embassy_net_wiznet::State<8, 8>> = StaticCell::new();
    let state = STATE.init(embassy_net_wiznet::State::<8, 8>::new());

    let (device, runner) = embassy_net_wiznet::new(mac_addr, state, spi_device, int, reset)
        .await
        .unwrap();

    info!("W5500 initialized");

    (device, runner)
}
