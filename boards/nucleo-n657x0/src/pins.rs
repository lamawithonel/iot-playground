//! Pin map of record for the NUCLEO-N657X0-Q ARS toolhead sensor
//! node, in code form.
//!
//! This module mirrors
//! `docs/src/projects/ars-toolhead-sensor/pinout.md`, the
//! provisional pin map of record decided from UM3417, the
//! STM32N657X0 datasheet (DS14791), RM0486, and the analog-chain
//! survey documents.  Update both together; the doc carries
//! citations and rationale, this module carries the constants
//! firmware will bind to.
//!
//! Compiled unconditionally: these are plain `&str` constants with
//! no embassy dependency, so nothing here needs the `g1-spike`
//! feature gate.  `AMP_MUTE_N`, `USER_BTN`, `AMP_I2C_SCL`, and
//! `AMP_I2C_SDA` are consumed by the phase-1 RTIC app in
//! `main.rs`'s `app` module (behind `g1-spike`), for the
//! peripherals that build today.  `MIC_AUD` and `AUDIO_PWM` stay
//! unconsumed (`#[allow(dead_code)]`): ADC1 and TIM1 have no
//! embassy-stm32 0.6.0 peripheral singleton for this chip-- see
//! `main.rs`'s `capture`/`sweep_engine` doc comments.  `VCP_TX` and
//! `VCP_RX` stay unconsumed too, permanently: primary logging is
//! defmt-RTT, not the VCP UART.  See `boards/nucleo-n657x0/
//! README.md` and `AGENTS.md`.

#![deny(warnings)]
#![deny(unsafe_code)]

/// Microphone analog input.
///
/// Pin PA8, analog mode (no AF index).  ADC1_INP5 (dual-mode
/// ADC12_INP5), serviced by GPDMA1 REQSEL 7 (`adc1_dma`) in
/// linked-list circular capture mode.  Arduino CN4 pin 1 (A0),
/// routed through the Nucleo's on-board 3V3-to-1V8 adaptation
/// amplifier.
///
/// Unconsumed: ADC1 has no embassy-stm32 0.6.0 peripheral
/// singleton for `stm32n657x0` (`stm32-metapac` 21.0.0 has
/// `registers: None` for both "ADC1" and "ADC12_COMMON" on this
/// chip).  Referenced from `main.rs::app::init` once that gap
/// closes upstream.
#[allow(dead_code)]
pub const MIC_AUD: &str = "PA8";

/// Swept-sine audio output to the amplifier line input.
///
/// Pin PE9, AF1.  TIM1_CH1, duty stream via GPDMA1 REQSEL 18
/// (`tim1_upd_dma`).  Arduino CN13 pin 4 (D3); also morpho CN15
/// pin 31.  Feeds an external RC low-pass into MAX9744 JP2 pin 2
/// (LEFTIN).
///
/// Unconsumed: TIM1 has no embassy-stm32 0.6.0 peripheral
/// singleton for `stm32n657x0` either (only TIM9 is generated for
/// this chip).  Referenced from `main.rs::app::init` once that gap
/// closes upstream.
#[allow(dead_code)]
pub const AUDIO_PWM: &str = "PE9";

/// Amplifier I2C clock (MAX9744 volume control).
///
/// Pin PH9, AF4.  I2C1 (Fm+), interrupt-driven event/error.
/// Morpho CN15 pin 3 (direct, no solder-bridge change).  Arduino
/// D15/A5 route only with SB2/SB4 ON and SB3/SB5 OFF.
pub const AMP_I2C_SCL: &str = "PH9";

/// Amplifier I2C data (MAX9744 volume control).
///
/// Pin PC1, AF4.  I2C1 (Fm+), interrupt-driven event/error.
/// Morpho CN15 pin 5 (direct, no solder-bridge change).  Arduino
/// D14/A4 route only with SB2/SB4 ON and SB3/SB5 OFF.
pub const AMP_I2C_SDA: &str = "PC1";

/// Amplifier mute control; drive LOW to mute.
///
/// Pin PD0, GPIO output (open-drain recommended), no AF.  Arduino
/// CN13 pin 3 (D2); also morpho CN15 pin 33.  Wired to MAX9744
/// JP2 pin 8 (MUTE_INV); the Adafruit board inverts sense versus
/// the bare chip datasheet.
pub const AMP_MUTE_N: &str = "PD0";

/// Spare EXTI input, on-board user button.
///
/// Pin PC13, GPIO input, EXTI13, no AF.  On-board blue user
/// button B1; also morpho CN3 pin 23.  Zero external wiring.
pub const USER_BTN: &str = "PC13";

/// Virtual COM port transmit (debug console out).
///
/// Pin PE5, AF7.  USART1_TX.  Internal to the STLINK-V3EC,
/// exposed as a Virtual COM port on CN10 USB.  Reserved: never
/// available for I2C1 or TIM despite its other AF options.
///
/// Permanently unconsumed: primary logging is defmt-RTT, not the
/// VCP UART; USART1's NVIC vector is borrowed only as an RTIC
/// dispatcher slot (see `main.rs`'s `app` module), never as an
/// instantiated peripheral.
#[allow(dead_code)]
pub const VCP_TX: &str = "PE5";

/// Virtual COM port receive (debug console in).
///
/// Pin PE6, AF7.  USART1_RX.  Internal to the STLINK-V3EC,
/// exposed as a Virtual COM port on CN10 USB.  Reserved, paired
/// with `VCP_TX`.
///
/// Permanently unconsumed; see `VCP_TX`.
#[allow(dead_code)]
pub const VCP_RX: &str = "PE6";

// Future embassy-stm32 pin bindings (sketch only-- unverified
// against real hardware).  Gate G1 has already run: AMP_MUTE_N,
// USER_BTN, and I2C1 (AMP_I2C_SCL/SDA) all compile and are claimed
// for real in `main.rs`'s `app` module today.  `mic_adc` and
// `audio_pwm` stay sketch-only because ADC1/TIM1 have no
// embassy-stm32 0.6.0 peripheral singleton for `stm32n657x0`-- see
// `MIC_AUD`/`AUDIO_PWM`'s doc comments above.
//
// use embassy_stm32::adc::Adc;
// use embassy_stm32::gpio::{Level, Output, Speed};
// use embassy_stm32::i2c::I2c;
// use embassy_stm32::peripherals;
// use embassy_stm32::timer::simple_pwm::SimplePwm;
// use embassy_stm32::usart::Uart;
//
// pub struct SensorPins {
//     pub mic_adc: Adc<'static, peripherals::ADC1>,
//     pub audio_pwm: SimplePwm<'static, peripherals::TIM1>,
//     pub amp_i2c: I2c<'static, peripherals::I2C1>,
//     pub amp_mute_n: Output<'static, peripherals::PD0>,
//     pub user_btn: peripherals::PC13,
//     pub vcp: Uart<'static, peripherals::USART1>,
// }
//
// impl SensorPins {
//     pub fn init(p: embassy_stm32::Peripherals) -> Self {
//         todo!("bind peripherals per the pin map above once ADC1/TIM1 land upstream")
//     }
// }
