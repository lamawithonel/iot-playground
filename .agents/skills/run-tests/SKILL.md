---
name: run-tests
description: >-
  Run host unit tests, clippy lints, formatting checks, bring-up
  tests, and smoke tests for the embedded Rust IoT firmware
  workspace.  Use when asked to test code, validate changes, check
  for errors, run CI locally, or before committing changes.  Covers
  cargo test on the host target, cargo clippy for the
  thumbv7em-none-eabihf BSP target, and on-device smoke tests with
  MQTT broker lifecycle management.
---

# Run Tests

Validate code changes by running the project's test and lint
pipeline locally.  This skill covers host unit tests,
cross-compilation linting, formatting, peripheral bring-up
tests, and smoke testing with MQTT broker management — the
same checks that CI runs on every pull request.

## When to Use This Skill

- Before committing or pushing changes
- When asked to "run tests", "check the build", or "validate"
- After modifying code in `core/`, `hal-abstractions/`, or
  `boards/`
- When debugging test failures or clippy warnings
- When running integration or smoke tests that exercise the
  firmware end-to-end

## Prerequisites

- **mise** must be installed.  See
  [Getting Started](https://mise.jdx.dev/getting-started.html)
  for installation instructions.
- After installing mise, run `mise install` in the repository
  root to install all required tools.
- **Rust** and **probe-rs** are managed by mise — no separate
  installation is needed.

## Quick Reference

| Check | Command | Target |
|-------|---------|--------|
| Host tests | `mise run test:host` | Host (auto-detected) |
| BSP clippy | `cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings` | ARM Cortex-M4F |
| Formatting | `cargo fmt --all -- --check` | N/A |
| Bring-up tests | `mise run test:device` | ARM (requires probe) |
| Smoke test | `mise run test:smoke` | ARM (requires probe) |
| Full pipeline | `mise run test:integration` | All of the above |
| Broker status | `mise run broker:status` | N/A |
| Broker start | `mise run broker:start` | N/A |
| Broker stop | `mise run broker:stop` | N/A |

## Procedure

### 1. Determine What Changed

Run `git diff --name-only` (or `git diff --staged --name-only`
for staged changes) to identify modified files.  Use this to
decide which checks are necessary:

| Files changed | Required checks |
|---------------|-----------------|
| `core/**` | Host tests, BSP clippy |
| `hal-abstractions/**` | Host tests, BSP clippy |
| `boards/**` | BSP clippy, bring-up tests, smoke test |
| `Cargo.toml` or `Cargo.lock` | Host tests, BSP clippy |
| `test/**` | Restart broker if running |
| `docs/**`, `*.md` | None (skip tests) |

When in doubt, run all checks.

### 2. Run Formatting Check

```bash
cargo fmt --all -- --check
```

If it fails, fix with `cargo fmt --all` and re-check.

### 3. Run Host Unit Tests

```bash
mise run test:host
```

Or equivalently:

```bash
cargo test --workspace \
  --target "$(rustc -vV | sed -n 's/^host: //p')" \
  --exclude feather-stm32f405
```

The `core/` and `hal-abstractions/` crates compile for the
host architecture using `#![cfg_attr(not(test), no_std)]`.
Board crate binaries have `test = false` and are excluded
from host tests automatically.  The `--exclude` is required
because `embedded-test`'s semihosting dependency does not
compile on the host.

### 4. Run BSP Clippy

```bash
cargo clippy --workspace \
  --target thumbv7em-none-eabihf -- -D warnings
```

This cross-compiles the entire workspace including board
support packages and treats all warnings as errors.

### 5. Run Bring-Up Tests (When Hardware Connected)

```bash
mise run test:device
```

This auto-detects the probe.  If no probe is connected, the
task prints a warning and exits successfully (skip, not fail).

Bring-up tests live in `boards/feather-stm32f405/tests/bringup.rs`
and validate peripheral initialization outside of RTIC.  The
five tests cover:

- **Clock tree:** PLL source is HSE
- **RNG:** hardware entropy is non-zero
- **I2C:** SEN66 sensor probe on I2C1
- **SPI:** W5500 Ethernet version register read on SPI1
- **TIM2:** monotonic tick rate

Tests use `embedded-test` 0.7.x with probe-rs.  Each test
resets the MCU for full isolation.

### 6. Manage the MQTT Broker and Run Smoke Test

The smoke test flashes release firmware to the device and
monitors RTT output for panics, HardFaults, and boot health
markers.  For a complete smoke test, the MQTT broker should
be running so the firmware can establish its TLS connection.

#### Check Broker Status

```bash
mise run broker:status
```

Prints `running` (exit 0) or `stopped` (exit 1).

#### Broker Lifecycle — Start If Needed, Restore After

Before running the smoke test, check whether the broker is
already running and preserve that state:

```bash
# Record whether broker was already running
_broker_was_running=false
if mise run broker:status >/dev/null 2>&1; then
  _broker_was_running=true
fi

# Start broker if not already running
if [ "$_broker_was_running" = false ]; then
  mise run broker:start
fi
```

After all tests complete (pass or fail), restore the broker
to its original state:

```bash
# Restore broker to original state
if [ "$_broker_was_running" = false ]; then
  mise run broker:stop
fi
```

**Always restore the broker state**, even after failures.
Use a trap or ensure the cleanup runs in a finally block.

#### Run the Smoke Test

```bash
mise run test:smoke
```

The smoke test builds debug firmware, flashes it, and
monitors RTT output for a configurable observation window
(default 45 seconds via `SMOKE_TEST_DURATION`).

**Smoke test validation** (Cucumber-RS, 53 scenarios):
The smoke test runs a Cucumber-RS validator binary
(`test/smoke-validator/`) that reads captured RTT log and
MQTT message files.  Gherkin feature specs in
`test/features/rtt/` and `test/features/mqtt/` define all
test scenarios:

- **RTT features** (6 files, 23 scenarios): boot sequence,
  network, security, MQTT protocol, sensor conditioning,
  error absence
- **MQTT features** (5 files, 30 scenarios): message
  structure, telemetry ranges, message ordering, timestamp
  accuracy, RTT/MQTT cross-validation

Each scenario is individually reported as PASS/FAIL.  The
validator also writes a CSV artifact of all captured MQTT
messages.

**Result categories:**
- **FAILS** on: panics, HardFaults, no RTT output,
  excessive ERROR-level messages, any validator scenario
  failure
- **WARNS** on: ERROR-level defmt messages in the RTT log
- **PASSES** when firmware boots cleanly and all validator
  scenarios pass

### 7. Interpret Results and Fix

#### All Checks Pass

Every command exits with code 0.  Proceed with the next step
(commit, report success, etc.).

#### Failing Host Tests

Look for the `FAILED` summary:

```
test result: FAILED. 37 passed; 1 failed; 0 ignored
```

Fix the code and re-run only host tests.

#### Failing Clippy

Clippy errors include file location and suggested fix:

```
error: unused variable: `x`
  --> core/src/lib.rs:42:9
```

Remember:

- `#![deny(warnings)]` is mandatory — any warning is an error
- `#![deny(unsafe_code)]` is mandatory unless the file is in
  the unsafe allowlist (see `AGENTS.md`)
- All public items must have doc comments

#### Failing Smoke Test

Check the RTT output printed between the `── RTT Output ──`
markers.  Common issues:

- **No output:** Device didn't boot — check flash/probe
  connection
- **Panic:** Stack overflow or assertion failure — check the
  panic message for the source location
- **HardFault:** Memory access violation — likely a buffer
  overflow or null pointer

#### Compilation Errors

If compilation fails entirely, check:

1. Missing target: `rustup target add thumbv7em-none-eabihf`
2. Missing tools: `mise install`
3. Dependency conflict: `cargo update`

### 8. Retry After Fixes

After fixing issues, re-run only the failing check — not the
full pipeline.  Once the failing check passes, confirm the
other checks still pass before proceeding.

## Gotchas

- The workspace default build target is
  `thumbv7em-none-eabihf` (set in `.cargo/config.toml`).
  Host tests **must** specify a host target or use
  `mise run test:host`, which auto-detects it.  Running
  `cargo test` without a target flag will fail.
- BSP crates cannot run on the host.  Do not attempt
  `cargo test -p feather-stm32f405` on a host target.
- The `feather-stm32f405` binary has `test = false` in its
  `Cargo.toml` — it never produces host test binaries.
- Clippy must target `thumbv7em-none-eabihf` to catch
  BSP-specific issues.
- This project uses `defmt` for logging, not `println!` or
  `log`.  Host tests may use standard `assert!` macros, but
  BSP code uses `defmt::info!`, `defmt::error!`, etc.
- Bring-up tests use `embedded-test` (not `defmt-test`,
  which is deprecated).  They deliberately run outside RTIC
  to validate the hardware foundation — clock tree, buses,
  and peripherals — before the RTIC scheduler is involved.
- The MQTT broker runs in a Podman container
  (`mosquitto-tls`).  It requires TLS certs generated by
  `mise run tls:server:broker`.  The `broker:start` task
  depends on this automatically.
- `broker:status` exits 0 if running, 1 if stopped.  Use
  this to conditionally start/stop the broker around tests.
- The smoke test detects ERROR-level defmt messages as
  warnings, not failures.  MQTT connection errors are
  expected when the broker is unreachable and do not fail
  the smoke test.
- MQTT validation requires `BROKER_HOST_IP` to be set and
  the native MQTT subscriber binary to be built.  MQTT tests
  are automatically skipped if the subscriber is not active.
- The smoke-validator binary (`test/smoke-validator/`) and
  MQTT subscriber (`test/mqtt-subscriber/`) are standalone
  Rust crates.  The validator reads RTT log and MQTT message
  files and runs Gherkin feature specs from `test/features/`.
  The subscriber is a TLS-capable MQTT client that captures
  messages to a JSONL file.

## Troubleshooting

### Command Not Found: mise

If `mise` is not found, it is not installed or not on your
`PATH`.  Install it from
[mise Getting Started](https://mise.jdx.dev/getting-started.html),
then run `mise install` in the repository root to set up
all required tools (Rust, probe-rs, etc.).

## Pre-Commit Checklist

Before committing, confirm all pass in order:

1. `cargo fmt --all -- --check`
2. `mise run test:host`
3. `cargo clippy --workspace --target thumbv7em-none-eabihf -- -D warnings`
4. `mise run test:device` (if probe connected)
5. `mise run test:smoke` (if probe connected; start broker
   first for full coverage)

This matches the CI pipeline in `.github/workflows/ci.yaml`.
Add a `Tested-on:` Git trailer noting what was validated.
