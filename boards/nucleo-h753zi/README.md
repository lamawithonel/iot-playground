# nucleo-h753zi-- NUCLEO-H753ZI Bring-Up Board Crate

**Status: active workspace member** (promoted 2026-07-19 after
hardware-verified bring-up; workspace gates cover it).  Phase 1
proves flash, defmt over RTT, RTIC 2.x scheduling on TIM2, and
the idle/WFI path with a heartbeat LED blink (LD1, PB0).

## Hardware

ST NUCLEO-H753ZI (STM32H753ZI, Cortex-M7 @ up to 480 MHz, 2 MB
flash, 128 KB DTCM + AXI/D2/D3 SRAM).  Board reference: UM2407
Rev 6.

## Planned Purpose

This board is a dual-role prototyping rig:

- An active acoustic resonance spectroscopy (ARS) loopback
  prototyping board: DAC1_OUT1 (PA4) jumpered to an ADC input
  (A0/PA3), sweep synthesis logic in `core/`.
- The ADR-009 Layer-3 network trigger board: on-chip Ethernet MAC
  + on-board PHY over RMII + `embassy-net`.

Neither module exists yet; see `AGENTS.md` for the module map.

## Project Docs

The ARS loopback role is part of the ARS toolhead sensor project;
see
[`docs/src/projects/ars-toolhead-sensor/`](../../docs/src/projects/ars-toolhead-sensor/README.md)
for the full project context.

## Building and Flashing

```sh
choom -n 1000 -- cargo build --release --target thumbv7em-none-eabihf
probe-rs run --probe 0483:374e --chip STM32H753ZITx \
  target/thumbv7em-none-eabihf/release/nucleo-h753zi
```

The bench has two probes; this board is on the ST-LINK
(`0483:374e`).  Always pass an explicit `--probe` selector.
