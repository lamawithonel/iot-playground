# boards/-- Board Profiles

One directory per board profile: a specific board plus its
peripherals and application purpose.  BSP crates are named after
the board (see
[`rust_style.md`](../.agents/rules/rust_style.md)).

## Profiles

Roster authority:
[`docs/src/boards/README.md`](../docs/src/boards/README.md)-- it
wins on any disagreement with this table.

| Directory | Board | Status |
|-----------|-------|--------|
| [`feather-stm32f405/`](feather-stm32f405/AGENTS.md) | Adafruit Feather STM32F405 + W5500 + SEN66 | Active (workspace member) |
| [`nucleo-h753zi/`](nucleo-h753zi/AGENTS.md) | ST NUCLEO-H753ZI (ARS loopback rig / net trigger board) | Active (workspace member) |
| [`nucleo-n657x0/`](nucleo-n657x0/AGENTS.md) | ST NUCLEO-N657X0-Q (ARS toolhead sensor) | Bring-up spike in progress, hardware on bench (workspace-excluded); G0 RAM-boot bench-verified 2026-07-21 |

## Local Rules

- Board documentation lives in the mdBook at
  [`docs/src/boards/`](../docs/src/boards/README.md), one page per
  board; a board directory here keeps only `AGENTS.md` and a short
  README pointing at its page (see
  [`prose_style.md`](../.agents/rules/prose_style.md)).
- New profiles start as `workspace.exclude` entries until they
  compile in CI (target installed, clippy clean); only then move
  to `members`.
- Promotion checklist: board crates select mutually-exclusive
  stm32-metapac chip features, so a promoted board needs its own
  `-p` cargo invocation in CI clippy/build steps and in
  [`testing_gates.md`](../.agents/rules/testing_gates.md), plus
  an `--exclude` in the host-test commands (bin crates do not
  host-test).
- Board crates hold hardware I/O and RTIC task wiring only; pure
  logic goes to `core/`, trait boundaries to `hal-abstractions/`.
- Each active profile needs a Layer 2 bring-up test binary
  (`tests/bringup.rs` pattern) per
  [ADR-009](../docs/src/architecture/decisions.md).
