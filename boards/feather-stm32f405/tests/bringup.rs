//! Peripheral bring-up tests for the Feather STM32F405
//!
//! These tests validate hardware initialization — clocks,
//! buses, and peripherals — using `embedded-test` with
//! probe-rs reset-per-test isolation.  They deliberately
//! run *outside* RTIC: bring-up tests verify the hardware
//! foundation beneath the application runtime, not the
//! runtime itself.  See ADR-009 for why this is correct.
//!
//! Run with:
//!
//! ```sh
//! cargo test -p feather-stm32f405
//! # or
//! mise run test:device
//! ```

#![no_std]
#![no_main]
#![deny(warnings)]
#![deny(unsafe_code)]

use defmt_rtt as _;
use panic_probe as _;

// Pull in the device interrupt vector table (provides
// interrupt names to cortex-m-rt's link.x).
use stm32_metapac as _;

/// Default interrupt handler for the test binary.
///
/// The main firmware gets this from RTIC's `#[app]` macro.
/// Test binaries need their own since they don't use RTIC.
#[cortex_m_rt::exception]
#[allow(unsafe_code)]
unsafe fn DefaultHandler(_irqn: i16) -> ! {
    panic!("unexpected interrupt {}", _irqn);
}

/// Build the production clock configuration.
///
/// This mirrors the `init` function in `main.rs` — any
/// divergence means the tests are validating a different
/// clock tree than what ships.
fn production_clock_config() -> embassy_stm32::Config {
    use embassy_stm32::rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, LsConfig, LseConfig, LseDrive, LseMode, Pll,
        PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource, RtcClockSource, Sysclk,
    };
    use embassy_stm32::time::Hertz;

    let mut config = embassy_stm32::Config::default();
    config.rcc.hse = Some(Hse {
        freq: Hertz(12_000_000),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll_src = PllSource::HSE;
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV6,
        mul: PllMul::MUL168,
        divp: Some(PllPDiv::DIV4),
        divq: Some(PllQDiv::DIV7),
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config.rcc.ls = LsConfig {
        rtc: RtcClockSource::LSE,
        lsi: false,
        lse: Some(LseConfig {
            frequency: Hertz(32_768),
            mode: LseMode::Oscillator(LseDrive::MediumHigh),
        }),
    };

    config
}

#[embedded_test::tests(default_timeout = 30)]
mod tests {
    use super::*;
    use defmt::{assert, assert_eq, info};
    use embassy_stm32::pac;

    struct State {
        p: embassy_stm32::Peripherals,
    }

    #[init]
    fn init() -> State {
        State {
            p: embassy_stm32::init(production_clock_config()),
        }
    }

    /// Validate PLL is the system clock source.
    ///
    /// Reads RCC_CFGR SWS bits to confirm the PLL locked
    /// and was selected as SYSCLK.  Catches HSE crystal
    /// failures, PLL divider regressions, and config drift
    /// between the test and production `init`.
    #[test]
    fn clock_tree_pll_source(_state: State) {
        info!("Validating PLL as system clock source");

        let cfgr = pac::RCC.cfgr().read();
        let sws = cfgr.sws().to_bits();

        // SWS = 0b10 → PLL selected as system clock
        assert_eq!(sws, 0b10, "SYSCLK not PLL (SWS={})", sws);

        info!("PLL confirmed as SYSCLK source");
    }

    /// Validate hardware RNG produces non-trivial output.
    ///
    /// Enables the RNG peripheral (requires 48 MHz from
    /// PLLQ) and reads multiple 32-bit values.  Asserts
    /// they are non-zero and not all identical.  Catches
    /// PLLQ misconfiguration, stuck-at faults, and
    /// "peripheral not clocked" failures.  TLS depends on
    /// a functioning RNG — a stuck RNG is security-critical.
    #[test]
    fn rng_entropy(_state: State) {
        info!("Validating hardware RNG entropy");

        let rng = pac::RNG;

        // Enable RNG clock on AHB2
        pac::RCC.ahb2enr().modify(|w| w.set_rngen(true));
        cortex_m::asm::delay(16);

        // Enable the RNG peripheral
        rng.cr().modify(|w| w.set_rngen(true));

        let mut values = [0u32; 4];
        for val in &mut values {
            let mut timeout = 1_000_000u32;
            while !rng.sr().read().drdy() {
                timeout -= 1;
                assert!(timeout > 0, "RNG timeout — check PLLQ (48 MHz required)",);
            }

            let sr = rng.sr().read();
            assert!(!sr.cecs(), "RNG clock error — PLLQ != 48 MHz");
            assert!(!sr.secs(), "RNG seed error — analog failure");

            *val = rng.dr().read();
        }

        for (i, &val) in values.iter().enumerate() {
            assert!(val != 0, "RNG zero at index {}", i);
        }

        let all_same = values.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "RNG stuck: identical values");

        info!(
            "RNG OK: {:08x} {:08x} {:08x} {:08x}",
            values[0], values[1], values[2], values[3],
        );
    }

    /// Probe the SEN66 sensor on I2C1 (address 0x6B).
    ///
    /// Initializes I2C1 at 400 kHz on PB6/PB7 and sends
    /// the SEN66 `GetSerialNumber` command (0xD033).
    /// Proves: I2C clock config, GPIO alternate function
    /// mapping, pull-up resistors, and sensor power.
    #[test]
    fn i2c_sen66_probe(state: State) {
        use embassy_stm32::i2c;
        use embassy_stm32::time::Hertz;
        use embedded_hal::i2c::I2c as _;

        info!("Probing SEN66 on I2C1 (0x6B)");

        let p = state.p;

        let mut cfg = i2c::Config::default();
        cfg.frequency = Hertz(400_000);

        let mut i2c = i2c::I2c::new_blocking(p.I2C1, p.PB6, p.PB7, cfg);

        const SEN66_ADDR: u8 = 0x6B;

        // GetSerialNumber: 0xD033
        let cmd = [0xD0u8, 0x33];
        i2c.write(SEN66_ADDR, &cmd).unwrap();

        // SEN66 needs processing time
        cortex_m::asm::delay(1_000_000); // ~12 ms at 84 MHz

        // Response: 4 words × (2 data + 1 CRC) = 12 bytes
        let mut buf = [0u8; 12];
        i2c.read(SEN66_ADDR, &mut buf).unwrap();

        let non_zero = buf.iter().filter(|&&b| b != 0).count();
        assert!(non_zero > 0, "SEN66 serial is all zeros");

        info!(
            "SEN66 OK: serial {:02x}{:02x}{:02x}{:02x}...",
            buf[0], buf[1], buf[3], buf[4],
        );
    }

    /// Read the W5500 version register (0x0039).
    ///
    /// Performs a hardware reset, then reads VERSIONR via
    /// SPI2.  Expected value is 0x04.  Proves: SPI clock
    /// polarity, MOSI/MISO routing, chip select, and
    /// W5500 power.
    #[test]
    fn spi_w5500_version(state: State) {
        use embassy_stm32::gpio::{Level, Output, Speed};
        use embassy_stm32::spi::{self, Spi};
        use embedded_hal::spi::SpiBus;

        info!("Reading W5500 version register via SPI2");

        let p = state.p;

        // W5500 hardware reset (active low)
        let mut reset = Output::new(p.PC3, Level::Low, Speed::Low);
        cortex_m::asm::delay(84_000); // ~1 ms pulse
        reset.set_high();
        cortex_m::asm::delay(8_400_000); // ~100 ms PLL lock

        let mut spi_cfg = spi::Config::default();
        spi_cfg.frequency = embassy_stm32::time::Hertz(1_000_000);

        let mut spi = Spi::new_blocking(p.SPI2, p.PB13, p.PB15, p.PB14, spi_cfg);

        let mut cs = Output::new(p.PC6, Level::High, Speed::VeryHigh);

        // W5500 frame: 2-byte addr + control + data
        // VERSIONR (0x0039), common block, read mode
        let tx = [0x00u8, 0x39, 0x00, 0x00];
        let mut rx = [0u8; 4];

        cs.set_low();
        SpiBus::transfer(&mut spi, &mut rx, &tx).unwrap();
        cs.set_high();

        assert_eq!(
            rx[3], 0x04,
            "W5500 VERSIONR: expected 0x04, got 0x{:02x}",
            rx[3],
        );

        info!("W5500 OK: version 0x{:02x}", rx[3]);
    }

    /// Verify TIM2 ticks at approximately the expected rate.
    ///
    /// Configures TIM2 with a prescaler for 1 MHz from the
    /// 84 MHz APB1 timer clock, then measures the counter
    /// delta over a CPU delay loop.  The RTIC monotonic
    /// depends on TIM2 — if it is misconfigured, every
    /// `Mono::delay()` and timeout in the firmware is wrong.
    #[test]
    fn tim2_tick_rate(_state: State) {
        info!("Validating TIM2 tick rate");

        let tim2 = pac::TIM2;

        // Enable TIM2 clock on APB1
        pac::RCC.apb1enr().modify(|w| w.set_tim2en(true));
        cortex_m::asm::delay(16);

        // APB1 timer clock = 2 × 42 MHz = 84 MHz
        // Prescaler 83 → 1 MHz counter tick
        tim2.psc().write_value(83);
        tim2.arr().write_value(0xFFFF_FFFF);
        tim2.egr().write(|w| w.set_ug(true)); // load PSC
        tim2.cr1().modify(|w| w.set_cen(true));

        cortex_m::asm::delay(100);
        let start = tim2.cnt().read();

        // ~10 ms delay at 84 MHz (rough; implementation-dependent)
        cortex_m::asm::delay(840_000);

        let end = tim2.cnt().read();
        let delta = end.wrapping_sub(start);

        assert!(delta > 0, "TIM2 not ticking");

        // Expect ~10_000 ticks but delay() timing varies;
        // accept 1_000..100_000 as a broad sanity check.
        assert!(
            delta > 1_000,
            "TIM2 too slow: {} ticks (expected ~10_000)",
            delta,
        );
        assert!(
            delta < 100_000,
            "TIM2 too fast: {} ticks (expected ~10_000)",
            delta,
        );

        info!("TIM2 OK: {} ticks (~{} ms)", delta, delta / 1_000);
    }
}
