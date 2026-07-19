# nucleo-n657x0-- ARS Toolhead Sensor Board Crate

**Status: scaffold only.**  No hardware is present, and nothing
builds; `src/main.rs` is a deliberate `compile_error!`.

Planned target: `thumbv8m.main-none-eabihf` (Cortex-M55; rustc
has no thumbv8.1m triple-- Helium/MVE selects via target-cpu).

## Why Workspace-Excluded

Per the `boards/` rule ([`boards/AGENTS.md`](../AGENTS.md)), new
profiles stay in the root `[workspace]` exclude list until they
compile in CI (target installed, clippy clean).  This crate
additionally gates on the bring-up spike: the probe-rs flow for
the flashless signed-FSBL external-NOR boot chain is unverified,
so no dependencies are pinned yet.

## Pinout

**Status: provisional.**  No hardware has arrived; every assignment below
is gated on Spike Gates G0-G5 in
[`pinout.md`](../../docs/src/projects/ars-toolhead-sensor/pinout.md) and
may still change-- see G2 (mic capture quality) and G3 (PWM audio
fidelity) in particular.  This diagram renders that document for quick
reference; it is not a second source of truth, and `pinout.md` wins on
any disagreement.

```
LEFT SIDE (Arduino Uno V3)              BOARD         RIGHT SIDE (ST Morpho)
==========================        Physical Layout     ======================
                                        .....
Signal      Func       Pin  Mark  .--|          |--.  Mark  Pin  Func       Signal
----------  ---------  ---  ----     |  N657X0  |     ----  ---  ---------  ------
                                     |          |
MIC_AUD     ADC1_INP5  PA8  A0    o--|          |
                                     |          |
AUDIO_PWM   TIM1_CH1   PE9  D3    o--|          |
                                     |          |--o  3     PH9  I2C1_SCL   AMP_I2C_SCL
                                     |          |--o  5     PC1  I2C1_SDA   AMP_I2C_SDA
AMP_MUTE_N  GPIO       PD0  D2    o--|          |
                                     '----------'
```

Signals the diagram cannot show (also assigned in `pinout.md`):

- `AUDIO_PWM` (PE9) and `AMP_MUTE_N` (PD0) also route to ST Morpho CN15
  pins 31 and 33-- the Arduino and Morpho headers share the same net on
  this board.
- `USER_BTN` (PC13, EXTI13) is the on-board blue button B1, also present
  on morpho CN3 pin 23.  Zero external wiring.
- `VCP_TX`/`VCP_RX` (PE5/PE6, USART1) are internal to the STLINK-V3EC
  debug probe, reserved for the ST-LINK virtual COM port, and exposed
  only via the CN10 USB connector-- never available for I2C1 or TIM.

| Signal | AF / Function | GPDMA Request | Notes |
|--------|---------------|----------------|-------|
| `MIC_AUD` | ADC1_INP5 (analog, no AF) | GPDMA1 REQSEL 7 (`adc1_dma`) | Linked-list circular capture. |
| `AUDIO_PWM` | TIM1_CH1 (AF1) | GPDMA1 REQSEL 18 (`tim1_upd_dma`) | Duty-cycle stream into CCR1; external RC low-pass feeds MAX9744 LEFTIN. |
| `AMP_I2C_SCL` | I2C1_SCL (AF4) | -- | Interrupt-driven event/error; private bus (not the VCP, not I2C2). |
| `AMP_I2C_SDA` | I2C1_SDA (AF4) | -- | Interrupt-driven event/error. |
| `AMP_MUTE_N` | GPIO, open-drain recommended | -- | Drive LOW to engage mute-- the MAX9744 board inverts MUTE. |
| `USER_BTN` | EXTI13 | -- | On-board B1; bring-up interaction and EXTI-path verification. |
| `VCP_TX` | USART1_TX (AF7) | -- | Reserved for the ST-LINK VCP. |
| `VCP_RX` | USART1_RX (AF7) | -- | Reserved for the ST-LINK VCP. |

Authoritative source:
[`pinout.md`](../../docs/src/projects/ars-toolhead-sensor/pinout.md)-- read
it for the full rationale, decisions, open questions, and gate criteria
behind every assignment above.

## Where the Real Content Lives

- Project docs:
  [`docs/src/projects/ars-toolhead-sensor/`](../../docs/src/projects/ars-toolhead-sensor/README.md)
- Decision record: ADR-010 in
  [`decisions.md`](../../docs/src/architecture/decisions.md)
