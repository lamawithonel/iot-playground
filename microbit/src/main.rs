#![deny(unsafe_code)]
#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_nrf::gpio::{Level, Output, OutputDrive};
use rtic::app;

// 1. Import the implementation
use rtic_monotonics::nrf::rtc::Rtc1;
// 2. FIX: Import the trait that adds .millis() to u64
use rtic_monotonics::nrf::rtc::ExtU64;

#[app(device = nrf52833_pac, dispatchers = [SWI0_EGU0])]
mod app {
    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("--- RTIC v2: Monotonic Fixed (Final) ---");

        // Initialize the Token
        let token = rtic_monotonics::create_nrf_rtc1_monotonic_token!();
        Rtc1::start(cx.device.RTC1, token);

        // Initialize GPIO (Embassy)
        let p = embassy_nrf::init(Default::default());
        let col = Output::new(p.P0_28, Level::Low, OutputDrive::Standard);
        let row = Output::new(p.P0_21, Level::High, OutputDrive::Standard);

        blinky::spawn(col, row).ok();

        (Shared {}, Local {})
    }

    #[task(priority = 1)]
    async fn blinky(_cx: blinky::Context, mut col: Output<'static>, mut row: Output<'static>) {
        loop {
            defmt::info!("Tick");
            col.set_high();
            row.set_low();

            // Now .millis() will be found because ExtU64 is in scope
            Rtc1::delay(500u64.millis()).await;

            defmt::info!("Tock");
            col.set_low();
            row.set_high();

            Rtc1::delay(500u64.millis()).await;
        }
    }
}
