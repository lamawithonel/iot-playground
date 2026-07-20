# ST NUCLEO-N657X0-Q-- ARS Toolhead Sensor

**Status: scaffold only.**  No hardware is present, and nothing
builds; `src/main.rs` is a deliberate `compile_error!`.

Planned target: `thumbv8m.main-none-eabihf` (Cortex-M55; rustc
has no thumbv8.1m triple-- Helium/MVE selects via target-cpu).

## Why Workspace-Excluded

Per the `boards/` rule
([`boards/AGENTS.md`](../../../boards/AGENTS.md)), new profiles stay
in the root `[workspace]` exclude list until they compile in CI
(target installed, clippy clean).  This crate additionally gates on
the bring-up spike: the probe-rs flow for the flashless external-NOR
boot chain via a signed first-stage boot loader (FSBL) is unverified,
so no dependencies are pinned yet.

## Pinout

**Status: provisional.**  No hardware has arrived; every ARS assignment
below is gated on Spike Gates G0-G5 in
[`pinout.md`](../projects/ars-toolhead-sensor/pinout.md) and
may still change-- see G2 (mic capture quality) and G3 (PWM audio
fidelity) in particular.  This diagram renders that document against the
full board header pinout; `pinout.md` wins on any disagreement.

The full Arduino Uno V3 header pinout is shown below (data from UM3417
Rev 3, Table 12).  Every header pin carries its Mark, MCU function, and
MCU pin; the five ARS-used signals are flagged with `*`.

```
LEFT  Arduino Power (CN5) + Analog (CN4)                  RIGHT  Arduino Digital (CN14 + CN13)
Device     Func         Pin   Mark                       Mark  Pin   Func           Device
------     ----         ---   ----                       ----  ----  -------------  ------
                                        .----------.
           5V_IN test   --    NC     o--|          |--o  D15   PH9   I2C1_SCL       ARS amp SCL *
           3V3 ref      --    IOREF  o--|          |--o  D14   PC1   I2C1_SDA       ARS amp SDA *
           Reset        NRST  RST    o--|          |--o  AREF  --    AVDD
           3.3V out     --    3V3    o--|          |--o  GND   --    Ground
           5V out       --    5V     o--|          |--o  D13   PE15  SPI5_SCK
           Ground       --    GND    o--|          |--o  D12   PG1   SPI5_MISO
           Ground       --    GND    o--|          |--o  D11   PG2   SPI5_MOSI/T14
           Power in     --    VIN    o--|          |--o  D10   PA3   SPI5_CS/T16
ARS mic *  ADC12_INP5   PA8   A0     o--| N657X0-Q |--o  D9    PD7   TIM1_CH2
           ADC12_INP10  PA9   A1     o--|  MB1940  |--o  D8    PD12  --
           ADC12_INP11  PA10  A2     o--|          |--o  D7    PE11  --
           ADC12_INP13  PA12  A3     o--|          |--o  D6    PD5   TIM1_CH4N
           ADC1_INP16   PF3   A4     o--|          |--o  D5    PE10  TIM1_CH2N
           ADC12_INP7   PG15  A5     o--|          |--o  D4    PE0   --
                                        |          |--o  D3    PE9   TIM1_CH1       ARS audio PWM *
                                        |          |--o  D2    PD0   GPIO           ARS amp mute *
                                        |          |--o  D1    PD8   USART3_TX
                                        |          |--o  D0    PD9   USART3_RX
                                        '----------'
```

Notes:

- `*` marks the five ARS signals: mic input (A0/PA8), audio PWM
  (D3/PE9), amp mute (D2/PD0), and the amp I2C1 bus (D15/PH9 SCL,
  D14/PC1 SDA).  `pinout.md` is the authority for which physical tap
  each uses; PH9/PC1 and PE9/PD0 are also exposed on the ST morpho
  headers (CN15), which is where `pinout.md` routes the amp bus and
  mute to avoid the Arduino I2C solder bridges.
- A0-A5 each route both a 3.3 V digital pin and a 1.8 V ADC pin through
  an on-board voltage-adaptation amplifier (UM3417 Figure 14); the
  ADC-side MCU pin is shown, since the mic uses the analog path (the N6
  ADC lives in the 1.8 V VDDA domain).
- A4/A5 can alternatively carry I2C1 (PC1/PH9) instead of ADC, but only
  with solder-bridge changes (SB2/SB4 ON, SB3/SB5 OFF); D15/D14 expose
  the same I2C1 pins with no bridges.
- The ST morpho headers (CN2/CN3/CN15/CN16) break out most remaining
  STM32 I/Os and are not reproduced here; see UM3417 Tables 13-14.  The
  on-board blue user button B1 (PC13/EXTI13) needs no external wiring,
  and the ST-LINK VCP (PE5/PE6, USART1) is reserved and reachable only
  over the CN10 USB connector.

Authoritative source:
[`pinout.md`](../projects/ars-toolhead-sensor/pinout.md)-- read it
for the full rationale, decisions, open questions, and gate criteria
behind every ARS assignment above.

## Where the Real Content Lives

- Project docs:
  [`docs/src/projects/ars-toolhead-sensor/`](../projects/ars-toolhead-sensor/README.md)
- Decision record: ADR-010 in
  [`decisions.md`](../architecture/decisions.md)
