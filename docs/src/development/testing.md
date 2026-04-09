# Testing Strategy

This document describes the project's five-layer embedded test
pyramid and how to run each layer.  For the full rationale behind
this structure, see
[ADR-009](../architecture/decisions.md#adr-009-test-strategy-and-the-embedded-test-pyramid).

## Overview

Embedded firmware testing is hard: most interesting behavior
requires physical hardware, flash cycles are slow, and emulators
lack peripheral fidelity.  The test strategy addresses this by
pushing as much validation as possible to the host, reserving
on-device time for things *only hardware can answer*.

The five layers, from cheapest to most expensive:

| Layer | Name | Runs On | Gate |
|------:|------|---------|------|
| 1 | Host unit tests | CI (x86_64) | Required, every PR |
| 2 | Peripheral bring-up | MCU via probe-rs | Required with HW |
| 3 | RTIC integration | Custom `#[app]` binaries | Future |
| 4 | System smoke test | MCU via probe-rs + RTT | Required with HW |
| 5 | End-to-end | MCU + MQTT broker | Future |

A new contributor should be able to run Layer 1 immediately and
Layers 2 and 4 with a board and probe attached.

## Why Bring-Up Tests Don't Use RTIC

Bring-up tests (Layer 2) deliberately run *outside* RTIC to
validate the hardware foundation beneath the application
runtime, not the runtime itself.  RTIC adds task scheduling,
priority-based preemption, and shared-resource management above
this foundation.  Testing without RTIC isolates hardware
behavior — clock trees, bus timing, peripheral registers — from
framework behavior.  If a bring-up test fails, the problem is
in the hardware configuration or wiring, never in an RTIC task
interaction.  This makes failures unambiguous and drastically
simplifies debugging.

## Test Layers

### Layer 1: Host Unit Tests

**What it tests:** Pure logic in `core/` and `hal-abstractions/`
— state machines, data formatting, time math, sensor value
conversions, and protocol encoding.

**Where it runs:** `cargo test` on the host (x86_64).  No
embedded target, no hardware.

**Key test categories:**

- Data integrity across transformation boundaries (e.g.,
  sensor value → formatted payload)
- Payload size budgets (worst-case values must fit in bounded
  buffers)
- NaN/Inf handling (must propagate as `None`, not `0.0`)
- Schema contract tests (field names, types, and optionality)
- Channel backpressure properties (mock sender/receiver under
  simulated stalls)

Host tests are the cheapest to write and the fastest to run.
When in doubt, write a host test.

### Layer 2: Peripheral Bring-Up

**What it tests:** Hardware initialization — clocks, buses,
and peripherals respond as expected.

**Where it runs:** On the MCU via `embedded-test` 0.7.x and
probe-rs.  Each test gets a full device reset for isolation.

**Test binary:** `boards/feather-stm32f405/tests/bringup.rs`

**Current tests:**

- **Clock tree:** Read `RCC_CFGR` SWS bits, confirm PLL is
  SYSCLK source.
- **RNG entropy:** Enable hardware RNG (requires 48 MHz PLLQ),
  read four 32-bit values, assert non-zero and non-identical.
- **I2C SEN66 probe:** Initialize I2C1 at 400 kHz, send
  `GetSerialNumber` (0xD033), assert valid response.
- **SPI W5500 version:** Reset W5500, read VERSIONR (0x0039)
  via SPI2, assert 0x04.
- **TIM2 tick rate:** Configure TIM2 at 1 MHz, measure counter
  delta over a CPU delay, assert within broad tolerance.

These tests catch PLL divider regressions, bus wiring
mistakes, dead peripherals, and configuration drift between
the test and production `init`.

### Layer 3: RTIC Integration (Deferred)

**What it will test:** Inter-task communication via
`rtic_sync` channels, monotonic timer behavior under load,
and vertical data slices (sensor → channel → network task).

**Why deferred:** Building custom `#[app]` test binaries
requires per-binary linker scripts, RTT-based pass/fail
parsing, and dedicated mise tasks.  This investment is
justified only after Layers 1, 2, and 4 are solid.

**Trigger criteria for starting Layer 3:**

- A bug escapes that Layer 2 + Layer 4 should have caught
  but didn't because it involves RTIC task interaction
- The `rtic_sync` channel backpressure under TLS stall
  becomes a real failure mode (not just theoretical)
- A second board profile is added, making shared RTIC test
  infrastructure worthwhile

### Layer 4: System Smoke Test

**What it tests:** The full firmware boots
successfully and reaches expected milestones in the correct
order.

**How it works:** Flash debug firmware, capture RTT output
via probe-rs, and assert that `defmt` milestones appear in
the expected sequence:

1. `"System initialized"`
2. `"TIM2 monotonic"`
3. `"I2C1 initialized"`
4. `"SEN66 initialized"`
5. `"Network stack initialized"`
6. `"SNTP sync successful"`

Milestone ordering is the test contract.  Changing a log
message that serves as a milestone is a test-breaking change.

### Layer 5: End-to-End (Future)

**What it will test:** The full data path from device through
TLS to an MQTT broker, with a subscriber validating received
messages.

**Infrastructure:** MCU + Mosquitto container (already in
`test/broker/`) + TLS certificates (generated by
`test/scripts/gen-tls-cert.sh`).

This layer is deferred until the MQTT publishing path is
stable and a self-hosted runner with USB passthrough is
available.

## Running Tests

### Layer 1: Host Unit Tests

```sh
mise run test:host
```

Or directly:

```sh
cargo test --workspace \
  --exclude feather-stm32f405 \
  --target "$(rustc -vV | sed -n 's/^host: //p')"
```

### Layer 2: On-Device Bring-Up Tests

```sh
mise run test:device
```

Or directly:

```sh
cargo test -p feather-stm32f405
```

Requires a probe-rs-compatible debug probe (J-Link, ST-Link,
or CMSIS-DAP) connected to the target board.

### Layer 4: System Smoke Test

```sh
mise run test:smoke
```

Flashes release firmware and monitors RTT output for
milestone ordering.  If the test requires a broker, it will
manage the broker lifecycle automatically.

### Full Pipeline

```sh
mise run test:integration
```

Runs Layers 1, 2, and 4 in sequence.

### Static Analysis

Formatting:

```sh
cargo fmt --all -- --check
```

Linting:

```sh
cargo clippy --workspace \
  --target thumbv7em-none-eabihf -- -D warnings
```

## When to Add a Test

Use this decision matrix to decide which layer needs a new
test:

| You changed… | Add a test in… |
|:-------------|:---------------|
| `core/` logic or data types | Layer 1 (host unit test) |
| BSP peripheral init or pin mapping | Layer 2 (bring-up test) |
| Boot sequence or milestone ordering | Layer 4 (smoke test covers you) |
| RTIC task interaction or channel wiring | Layer 4 for now; Layer 3 when available |
| MQTT payload format or TLS config | Layer 1 (schema/contract test) + Layer 4 |

When in doubt: if the code can be tested without hardware,
write a host test.  Hardware test cycles are expensive — save
them for questions only hardware can answer.

## CI/CD Strategy

**Host tests** (`test:host`) are a required merge gate on
every PR.  They run on GitHub Actions public runners with no
special hardware.

**On-device tests** and **smoke tests** require a probe and
target board.  In CI, these jobs detect whether hardware is
available and skip gracefully when it is not.  They run as
non-blocking checks until the self-hosted runner has completed
a 30-day reliability bake, after which they will be promoted
to required.

**Hardware attestation:** For changes that touch BSP code or
peripheral configuration, contributors should run on-device
tests locally and add a `Tested-on:` git trailer to their
commit message:

```
Tested-on: Feather STM32F405 + J-Link + SEN66 + W5500
```

Maintainers may also apply an `hw-verified` label to PRs that
have been validated on hardware.

**QEMU/Renode emulation** is deliberately skipped.  STM32F405
peripheral models are incomplete, and the ROI is negative at
one board.  This will be revisited when three or more target
boards are supported.

## Future: RTIC Integration Tests

Layer 3 will introduce custom `#[app]` test binaries that run
real RTIC tasks with channels, monotonic timers, and
peripheral interactions.  See the trigger criteria in the
[Layer 3 section](#layer-3-rtic-integration-deferred) above.

When the time comes, each test binary will be a standalone
RTIC application in `boards/<board>/tests/` with its own
`#[app]` macro, RTT-based pass/fail reporting, and a
corresponding mise task.

## Future: HIL Testing

Hardware-in-the-loop (HIL) testing will use a Saleae Logic
Pro 16 for automated protocol validation — capturing SPI,
I2C, and UART traffic during test runs and asserting timing
and data correctness programmatically.  This is planned for
Phase 3+ when the self-hosted runner infrastructure is in
place.

---

See [ADR-009](../architecture/decisions.md#adr-009-test-strategy-and-the-embedded-test-pyramid)
for the full decision record, including alternatives
considered and consequences.
