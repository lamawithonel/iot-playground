# boards/-- Board Profiles

One directory per board profile: a specific board plus its
peripherals and application purpose.  BSP crates are named after
the board (see
[`rust_style.md`](../.agents/rules/rust_style.md)).

## Profiles

| Directory | Board | Status |
|-----------|-------|--------|
| [`feather-stm32f405/`](feather-stm32f405/AGENTS.md) | Adafruit Feather STM32F405 + W5500 + SEN66 | Active (workspace member) |
| [`nucleo-n657x0/`](nucleo-n657x0/AGENTS.md) | ST NUCLEO-N657X0-Q (ARS toolhead sensor) | Scaffold only (workspace-excluded) |

## Local Rules

- New profiles start as `workspace.exclude` entries until they
  compile in CI (target installed, clippy clean); only then move
  to `members`.
- Board crates hold hardware I/O and RTIC task wiring only; pure
  logic goes to `core/`, trait boundaries to `hal-abstractions/`.
- Each active profile needs a Layer 2 bring-up test binary
  (`tests/bringup.rs` pattern) per
  [ADR-009](../docs/src/architecture/decisions.md).
