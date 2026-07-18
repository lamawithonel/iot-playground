# nucleo-n657x0/ -- ARS Toolhead Sensor Node

ST NUCLEO-N657X0-Q (STM32N657X0, Cortex-M55 @ 800 MHz, ~4.2 MB
contiguous SRAM, flashless signed-FSBL external-NOR boot,
Neural-ART NPU).  **Status: scaffold** -- workspace-excluded, no
hardware, `main.rs` is a deliberate `compile_error!`.

## Module Plan

| Module | Purpose (planned) |
|--------|-------------------|
| `main.rs` | RTIC `#[app]`: init, sweep_engine, capture, analysis, telemetry, idle (WFI) |
| `audio/` | Sweep synthesis and the audio output path (spike-gated) |
| `capture/` | ADC mic capture windows (spike-gated) |
| `analysis/` | FFT / feature-extraction glue (pure logic goes to `core/`) |

## Local Rules

- No unsafe allowlist entries here; nothing in this crate may be
  `unsafe`.
- RTIC-first applies: RTIC 2.x scheduling, Embassy HAL only, no
  embassy-executor.
- Changes beyond docs require the bring-up spike first -- boot
  chain, probe-rs flow, and embassy-stm32 N6 coverage are all
  unverified (see the project docs' "Open Spikes").
