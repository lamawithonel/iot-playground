# ST NUCLEO-N657X0-Q-- ARS Toolhead Sensor

**Status: bring-up spike in progress.**  Gate G0's RAM-boot half
passed on the bench 2026-07-21: the `g1-spike` image builds, loads
over SWD, and boots from AXISRAM with a defmt log line decoded over
RTT (see [Bring-Up: RAM-Boot Dev Flow](#bring-up-ram-boot-dev-flow)
below).  The default feature set remains a deliberate
`compile_error!` scaffold guard, the external-NOR boot chain via a
signed first-stage boot loader (FSBL) is still unverified, and the
crate stays workspace-excluded.

Planned target: `thumbv8m.main-none-eabihf` (Cortex-M55; rustc
has no thumbv8.1m triple-- Helium/MVE selects via target-cpu).

## Why Workspace-Excluded

Per the `boards/` rule
([`boards/AGENTS.md`](../../../boards/AGENTS.md)), new profiles stay
in the root `[workspace]` exclude list until they compile in CI
(target installed, clippy clean).  This crate additionally gates on
the bring-up spike: the flow for the flashless external-NOR boot
chain via a signed FSBL is unverified.  The `g1-spike` feature pins
the dependency stack and is the G0 RAM-boot image.

## Bring-Up: RAM-Boot Dev Flow

**Status: bench-verified 2026-07-21** for the RAM-boot half of gate
G0.  The N6 is flashless; this flow loads and runs an image entirely
from AXISRAM over SWD.  The external-NOR/FSBL boot chain is a
separate, still-unverified flow.

Board precondition: JP2 (BOOT1) in position 2 (logical 1, dev
boot)-- the boot ROM only exposes a debug-friendly state in dev
boot, and BOOT pins latch at reset, so power-cycle after moving the
jumper.  In flash-boot position the debug port enumerates but
reports no access ports.

1. Build: `cargo build --features g1-spike` in the crate directory
   (`thumbv8m.main-none-eabihf`).
2. Load: `STM32_Programmer_CLI -c port=SWD mode=HOTPLUG -halt -w
   <image>.elf --verify` (8 MHz SWD; the CLI requires a `.elf` file
   extension).
3. Start: read initial SP and entry from the ELF (vector table base
   `0x341A0000`), then `-coreReg MSP=<vector0> PC=<entry> -run`.
   Do not use `-s`: it soft-resets the chip, which re-enters the
   boot ROM's dev-boot wait loop instead of the loaded image.
4. Observe: `probe-rs attach --chip STM32N657 --speed 100 <elf>`
   decodes defmt over RTT.

Two toolchain caveats, both bench-reproduced: probe-rs `run`'s load
path wedges the onboard STLINK-V3EC with USB bulk-transfer timeouts
(upstream probe-rs issue; `attach` is unaffected), so the load step
goes through STM32CubeProgrammer CLI (2.21.0+; 2.20.0 has a
flash-erase bug on this board).  And the image must set VTOR
itself-- the boot ROM's vector table stays live otherwise and the
first exception hard-faults into ROM; the crate uses cortex-m-rt's
`set-vtor` feature for this (see the comment in `Cargo.toml`).

## Pinout

**Status: provisional.**  Hardware has been on the bench since 2026-07-20, but
nothing below is bench-verified; every ARS assignment is gated on Spike
Gates G0-G5 in
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

## HIL Bench: Saleae Probe Hookup

The plan below covers the ARS toolhead sensor's five ARS signals
plus two bring-up aids, probed with the Logic MSO 2x100 (2 analog
channels, 8 digital channels, expandable to 20).  General MSO
operation-- the 1.65 V digital-threshold rule for 3.3 V logic,
grounding practice, and trigger-type explanations-- lives in the
[Saleae Logic skill](../../../.agents/skills/saleae-logic/SKILL.md);
this page states only what is specific to this board.
[`pinout.md`](../projects/ars-toolhead-sensor/pinout.md) wins on
every pin identity below; this page decides no pin assignments of
its own.

All six probed digital signals sit on the 3.3 V header domain, so
every digital channel uses the 1.65 V threshold preset.  The debug
console is not a probe point: USART1 VCP (PE5/PE6) is internal to
the STLINK-V3EC and reachable only over the CN10 USB connector, so
console liveness is watched over USB, never captured on a digital
channel.

### Digital channel plan

| Channel | Signal | Purpose |
|---|---|---|
| D0 | AMP_I2C_SCL (PH9, morpho CN15 pin 3) | I2C1 clock to the MAX9744; pair with D1 to decode volume and mute register writes. |
| D1 | AMP_I2C_SDA (PC1, morpho CN15 pin 5) | I2C1 data to the MAX9744 at address 0x4B; see `pinout.md` for bus rationale. |
| D2 | AUDIO_PWM (PE9, Arduino D3, also morpho CN15 pin 31) | TIM1_CH1 PWM carrier into the RC low-pass ahead of the MAX9744 line input; a gate G3 fidelity input. |
| D3 | AMP_MUTE_N (PD0, Arduino D2, also morpho CN15 pin 33) | Mute control into the amp's inverting stage-- drive LOW to mute; rising edge marks unmute/sweep start. |
| D4 | USER_BTN (PC13, morpho CN3 pin 23) | On-board blue user button; bring-up interaction and EXTI-path liveness. |
| D5 | NRST | Board reset; anchors bring-up captures to power-on/warm-reset timing. |

D6 and D7 are free in the 8-channel base kit; the digital channel
count expands to 20 across up to 5 probe cables
(`logic_mso_user_manual.pdf`, Digital Measurements) if a later spike
adds signals of interest.

### Analog channel plan

| Channel | Signal | Purpose |
|---|---|---|
| Analog 1 | MIC_AUD, Arduino A0 (CN4 pin 1), 3.3 V header side | Mic capture ahead of the on-board 3.3 V-to-1.8 V adaptation amplifier; probe the 3.3 V header pin, never the 1.8 V ADC-side pin-- the mic's ~1.65 V bias lives on the 3.3 V side.  A gate G2 input. |
| Analog 2 | Audio line after the external RC low-pass, MAX9744 LEFTIN (JP2 pin 2)-- or moved to raw PE9 to inspect the 200 kHz PWM carrier directly | Gate G3 SNR/THD comparison between the filtered line signal and, when moved, the raw carrier. |

### Trigger points

An edge trigger on AMP_MUTE_N's rising edge (unmute, marking sweep
start) or on I2C1's first start bit (the volume command to the
MAX9744 at 0x4B) anchors an audio-path capture to the start of a
measurement window.  Bring-up captures trigger on USER_BTN or NRST
instead.  (Trigger types-- Edge, Pulse, Slope-- are explained in the
skill linked above.)

### Verification status

Hardware has been on the bench since 2026-07-20, but no probe is
attached yet and Logic 2 is not installed on the bench machine.
Treat every channel assignment above as unverified until it runs
against the board and is confirmed in the Logic 2 UI.  Gate G2/G3
pass/fail measurement specs-- not channel assignments-- live in
[HIL measurements](../projects/ars-toolhead-sensor/hil-measurements.md).

## Where the Real Content Lives

- Project docs:
  [`docs/src/projects/ars-toolhead-sensor/`](../projects/ars-toolhead-sensor/README.md)
- Decision record: ADR-010 in
  [`decisions.md`](../architecture/decisions.md)
