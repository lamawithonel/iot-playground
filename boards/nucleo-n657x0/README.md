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

## Where the Real Content Lives

- Project docs:
  [`docs/src/projects/ars-toolhead-sensor/`](../../docs/src/projects/ars-toolhead-sensor/README.md)
- Decision record: ADR-010 in
  [`decisions.md`](../../docs/src/architecture/decisions.md)
