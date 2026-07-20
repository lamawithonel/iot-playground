# ST NUCLEO-H753ZI

**Status: active workspace member**, promoted 2026-07-19 after
hardware-verified bring-up; workspace gates cover it.  Phase 1
proves flash, defmt over RTT, RTIC 2.x scheduling on TIM2, and the
idle/WFI path with a heartbeat LED blink (LD1, PB0).

## Hardware

ST NUCLEO-H753ZI (STM32H753ZI, Cortex-M7 @ up to 480 MHz, 2 MB
flash, 128 KB DTCM + AXI/D2/D3 SRAM).  Board reference: UM2407
Rev 6.

This board is a dual-role prototyping rig, and neither role is
wired up yet-- see the
[module map](../../../boards/nucleo-h753zi/AGENTS.md) for what
exists today:

- An active acoustic resonance spectroscopy (ARS) loopback
  prototyping board: DAC1_OUT1 (PA4) jumpered to an ADC input
  (A0/PA3), with sweep synthesis logic in `core/`.  This role is
  part of the wider ARS toolhead sensor project; see
  [its docs](../projects/ars-toolhead-sensor/README.md) for
  context.
- The ADR-009 Layer-3 network trigger board: an on-chip Ethernet
  MAC plus an on-board PHY over RMII, driven by `embassy-net`.

## Building and Flashing

```sh
choom -n 1000 -- cargo build --release --target thumbv7em-none-eabihf
probe-rs run --probe 0483:374e --chip STM32H753ZITx \
  target/thumbv7em-none-eabihf/release/nucleo-h753zi
```

The bench has two probes; this board is on the ST-LINK
(`0483:374e`).  Always pass an explicit `--probe` selector.

## HIL Bench: Saleae Probe Hookup

The plan below covers the NETWORK bring-up role (the ADR-009
Layer-3 trigger board), probed with the Logic MSO 2x100 (2 analog
channels, 8 digital channels, expandable to 20).  General MSO
operation-- the 1.65 V digital-threshold rule for 3.3 V logic,
grounding practice, and trigger-type explanations-- lives in the
[Saleae Logic skill](../../../.agents/skills/saleae-logic/SKILL.md);
this page states only what is specific to this board.  Facts below
are cited to the Saleae datasheet (`logic_mso_data_sheet.pdf`) and
user manual (`logic_mso_user_manual.pdf`).

This board has no W5500 (that part is on `feather-stm32f405`); the
STM32H753ZI drives its Ethernet MAC through an on-board LAN8742 PHY
over RMII instead, so the probe points below are RMII
management/clock lines, not an SPI bus.  The repo's `net/` module
that will configure those pins is still planned, not written (see
the [module map](../../../boards/nucleo-h753zi/AGENTS.md)).  Every
board-side pin identity below is therefore the standard
NUCLEO-H753ZI (UM2407, MB1364 baseboard) reference pinout used by
ST's own Ethernet HAL examples for this board family-- unverified
against this repo's code or a fresh read of the UM2407 PDF.  Two
exceptions are cited to the exact UM2407 location already
established elsewhere in this repo: LD1 (green, PB0, UM2407 Rev 6
p.27 Sec 7.6.1) and the USART3-is-the-ST-LINK-VCP assignment
(UM2407 Rev 6 p.28 Sec 7.6.5)-- the cite covers the USART3/VCP
pairing, not the PD8 pin number, which stays "typical".  Confirm
every other pin against UM2407's pinout tables before probing.

### Digital channel plan (NETWORK bring-up)

| Channel | Signal | Purpose |
|---|---|---|
| D0 | USART3 console TX (PD8, typical Nucleo-144 VCP pin, UM2407 Rev 6 p.28 Sec 7.6.5) | The debug-UART console falls back to this pin once `net/` lands; confirms firmware is alive and logging. |
| D1 | RMII_REF_CLK (PA1, typical) | 50 MHz PHY reference clock; presence confirms the clock tree feeding the LAN8742 is up before any SMI traffic is expected. |
| D2 | RMII_MDC (PC1, typical) | SMI management clock; toggling confirms the MAC is driving PHY register reads/writes. |
| D3 | RMII_MDIO (PA2, typical) | SMI management data; correlate with D2 to read back PHY register transactions (link status, autonegotiation state) by eye or with a custom decoder. |
| D4 | RMII_CRS_DV (PA7, typical) | Carrier-sense/data-valid-- the simplest single-line "is there RMII receive activity" probe point, i.e. the RMII activity channel. |
| D5 | LD1 (green, PB0, UM2407 Rev 6 p.27 Sec 7.6.1) | Heartbeat/general liveness, already wired in `main.rs`. |
| D6 | LD2 (blue, PE1, typical) | Free for a link-status indicator once `net/` lands. |
| D7 | LD3 (red, PB14, typical) | Free for an error/activity indicator once `net/` lands. |

That fills the 8-channel base kit.  NRST (board reset) is a 9th
signal of interest-- swap it into D0's slot for a dedicated
power-on-reset/warm-reset timing capture (the UART console has
nothing to say during reset anyway), or add a third digital probe
cable: the digital channel count expands to 20 across up to 5 probe
cables, 4 channels each (`logic_mso_user_manual.pdf`, Digital
Measurements).

### Analog channel plan (power/reference integrity)

| Channel | Signal | Purpose |
|---|---|---|
| Analog 1 | 3V3 rail | Confirms the regulator holds 3.3 V under RMII/PHY current transients (autonegotiation, TX bursts); AC-couple to inspect switching-regulator ripple, DC-couple to check absolute droop. |
| Analog 2 | VREF | Confirms the ADC/DAC reference rail is stable and matches 3V3 (typical Nucleo-144 default, tied via a solder bridge-- verify against UM2407 before assuming continuity). |

### Trigger points

For a NETWORK bring-up capture, an edge trigger on NRST's rising
edge (end of reset) or on D0's UART console TX (first falling edge,
i.e. the first UART start bit) anchors the capture to the start of
firmware execution.  From there, the REF_CLK/MDC/MDIO/CRS_DV
channels show whether the PHY clock, SMI transactions, and link
activity come up in the expected order.  (Trigger types-- Edge,
Pulse, Slope-- are explained in the skill linked above.)

### Verification status

Nothing in this section has been probed against real hardware.
Logic 2 is not installed on the bench machine, the `net/` module
has not been written, and the RMII/LED/reset pin numbers above were
not re-verified against the UM2407 PDF-- they are the standard
Nucleo-144 reference-design values, flagged "typical" throughout
except for LD1 and the USART3-to-VCP assignment (the PD8 pin
number itself is "typical"; see the paragraph above).
Treat every channel index and pin identity in this section as
unverified until it is run against the actual board and confirmed
in the Logic 2 UI.

The ARS DAC/ADC loopback role (PA4/PA3) has its own bench plan; see
[HIL measurements](../projects/ars-toolhead-sensor/hil-measurements.md)
in the ARS toolhead sensor project docs.
