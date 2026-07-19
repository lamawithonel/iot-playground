---
paths:
  - "core/**"
  - "hal-abstractions/**"
  - "boards/**"
  - "**/Cargo.toml"
  - "Cargo.lock"
---

# Testing Rules and Gate Criteria

## Test Coverage Requirements

- **REQUIRED:** Platform-agnostic code in `core/` MUST have
  unit tests
- **REQUIRED:** Run all applicable tests before every commit
- **REQUIRED:** All gate criteria MUST pass before committing
- BSP code is tested via integration tests on hardware
- Use `embedded-test` for on-device tests (not `defmt-test`,
  which is deprecated)

## Pre-Commit Test Commands

Run these commands from the workspace root before every
commit.  The host target is auto-detected; do not hardcode
an architecture.

### 1. Formatting Check

```bash
cargo fmt --all -- --check
```

### 2. Host Unit Tests

```bash
mise run test:host
```

Or equivalently:

```bash
cargo test --workspace \
  --exclude feather-stm32f405 \
  --target "$(rustc -vV | sed -n 's/^host: //p')"
```

The `core/` and `hal-abstractions/` crates use
`#![cfg_attr(not(test), no_std)]` to compile as normal `std`
crates during host tests.  Board crate binaries have
`test = false` and are excluded from `cargo test --workspace`
automatically.

### 3. Clippy (Embedded Target)

```bash
cargo clippy --workspace \
  --target thumbv7em-none-eabihf -- -D warnings
```

Clippy MUST target the embedded architecture
(`thumbv7em-none-eabihf`), not the host, because board crates
only compile for ARM Cortex-M.

### 4. Device Tests (When Hardware Is Connected)

```bash
mise run test:device
```

Device tests require a probe (J-Link or compatible) connected
via USB.  If no probe is detected, the task skips gracefully.
Note the omission in the commit message if skipped.

## Test Selection Matrix

Choose which checks to run based on which paths changed:

| Changed Path        | fmt | Host Tests | Clippy | Device   |
|---------------------|:---:|:----------:|:------:|:--------:|
| `core/`             | ✅  |     ✅     |   ✅   | Optional |
| `hal-abstractions/` | ✅  |     ✅     |   ✅   | Optional |
| `boards/`           | ✅  |     ✅     |   ✅   | Recommended |
| `Cargo.toml` (root) | ✅  |     ✅     |   ✅   | Optional |
| `Cargo.lock`        | ✅  |     ✅     |   ✅   | Optional |
| `test/`             |  --  |      --     |    --   |    --     |
| `docs/`             |  --  |      --     |    --   |    --     |

Changes to `test/` (broker scripts, TLS certs) do not require
code tests, but a running broker may need to be restarted with
`mise run broker:stop && mise run broker:start`.

**When in doubt, run everything.**  The full host suite
completes in under 30 seconds.

## Gate Criteria

All of the following MUST pass before committing:

1. **Formatting**-- `cargo fmt --all -- --check` reports no
   diffs
2. **Host tests**-- all tests pass (exit code 0, zero
   failures)
3. **Clippy**-- zero warnings on the embedded target
   (exit code 0)
4. **Device tests**-- pass if hardware is connected; may be
   skipped otherwise

**FORBIDDEN:** Do not use `--skip`, `#[ignore]`, or
`#[allow(clippy::*)]` to bypass failures without explicit
user approval.

Add a `Tested-on:` Git trailer to commit messages indicating
what was validated:

```
Tested-on: host (x86_64-unknown-linux-gnu), clippy
```

When hardware was available:

```
Tested-on: host (x86_64-unknown-linux-gnu), clippy, device (STM32F405)
```
