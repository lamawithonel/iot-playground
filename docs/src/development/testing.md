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
| 4 | System smoke test | MCU + RTT + MQTT + Cucumber-RS | Required with HW |
| 5 | End-to-end | MCU + MQTT broker + validator | Required with HW |

A new contributor should be able to run Layer 1 immediately and
Layers 2 and 4 with a board and probe attached.

## Why Bring-Up Tests Don't Use RTIC

Bring-up tests (Layer 2) deliberately run *outside* RTIC to
validate the hardware foundation beneath the application
runtime, not the runtime itself.  RTIC adds task scheduling,
priority-based preemption, and shared-resource management above
this foundation.  Testing without RTIC isolates hardware
behavior-- clock trees, bus timing, peripheral registers-- from
framework behavior.  If a bring-up test fails, the problem is
in the hardware configuration or wiring, never in an RTIC task
interaction.  This makes failures unambiguous and drastically
simplifies debugging.

## Test Layers

### Layer 1: Host Unit Tests

**What it tests:** Pure logic in `core/` and
`hal-abstractions/`-- state machines, data formatting, time math,
sensor value conversions, and protocol encoding.

**Where it runs:** `cargo test` on the host (x86_64).  No
embedded target, no hardware.

**Key test categories:**

- Data integrity across transformation boundaries (e.g.,
  sensor value -> formatted payload)
- Payload size budgets (worst-case values must fit in bounded
  buffers)
- NaN/Inf handling (must propagate as `None`, not `0.0`)
- Schema contract tests (field names, types, and optionality)
- Channel backpressure properties (mock sender/receiver under
  simulated stalls)

Host tests are the cheapest to write and the fastest to run.
When in doubt, write a host test.

### Layer 2: Peripheral Bring-Up

**What it tests:** Hardware initialization-- clocks, buses,
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
- **EXTI2 W5500 INT pin:** Reset W5500, configure PC2 as
  input with pull-up, assert pin reads HIGH (active-low INT
  line is deasserted at idle).  Validates physical wiring for
  the EXTI2 interrupt path.
- **TIM2 tick rate:** Configure TIM2 at 1 MHz, measure counter
  delta over a CPU delay, assert within broad tolerance.

These tests catch PLL divider regressions, bus wiring
mistakes, dead peripherals, and configuration drift between
the test and production `init`.

### Layer 3: RTIC Integration (Deferred)

**What it will test:** Inter-task communication via
`rtic_sync` channels, monotonic timer behavior under load,
and vertical data slices (sensor -> channel -> network task).

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

**What it tests:** The full firmware boots successfully, enters
sleep mode correctly, publishes MQTT messages with valid data,
and handles sensor conditioning windows.

**Routing:** Smoke is a test class, not a board feature.
`mise run test:smoke [<board>...]` resolves boards exactly like
`mise run build` (arguments, the `IOT_BOARDS` pin, or the default
board) and dispatches each to its suite at `test/smoke/<board>.sh`;
a board without a suite skips loudly.  The feather suite is the
Cucumber-RS pipeline described below; the nucleo-n657x0 suite
RAM-boots the net firmware and asserts the DHCP -> SNTP -> MQTT
publish/PUBACK chain over RTT.

**How the feather suite works:** A Cucumber-RS validator
(`test/smoke-validator/`) orchestrates the entire pipeline:

1. Flash debug firmware via probe-rs
2. Capture RTT output to a log file via `probe-rs run`
3. Subscribe to MQTT topics and record messages as JSONL
4. After the configured test duration, parse both files
   against Gherkin feature scenarios

**Feature categories** (`test/features/`):

- **RTT boot milestones** (`rtt/boot_sequence.feature`):
  Assert `defmt` milestones appear in order-- system init,
  TIM2 monotonic, I2C, SEN66, network stack, SNTP sync.
- **WFI sleep mode** (`rtt/wfi_sleep.feature`): Verify
  `wfi_wakes:` counter appears in RTT, values are non-zero
  (proving the device sleeps), and not excessively large
  (ruling out busy-wait).
- **Interrupt-driven reception**
  (`rtt/interrupt_driven.feature`): Prove EXTI2 events are
  non-zero (interrupts fire), bounded (not polling), and
  present every publish cycle.
- **MQTT protocol** (`rtt/mqtt_protocol.feature`): Validate
  topic structure, JSON schema, required fields, QoS level,
  and retain flag.
- **Sensor conditioning**
  (`rtt/sensor_conditioning.feature`): Verify CO2
  conditioning (needs >=60s) and VOC/NOx conditioning
  (needs longer).  Uses `@extended` and `@full` tags to
  control which tiers run these scenarios.
- **Error absence** (`rtt/error_absence.feature`): Assert
  no TLS errors, MQTT disconnections, or sensor CRC failures
  in the RTT log.
- **RTT<->MQTT correlation**
  (`mqtt/cross_validation.feature`): Verify that sensor
  readings logged via RTT match those published via MQTT.
- **Message timing** (`mqtt/message_timing.feature`):
  Assert messages arrive at the configured sample interval
  +/- tolerance.

**Milestone ordering and log messages are the test contract.**
Changing a `defmt` message that serves as a milestone or
data source is a test-breaking change.

### Layer 5: End-to-End

**What it tests:** The full data path from device through
TLS to an MQTT broker, with a subscriber validating received
messages against Gherkin scenarios.

**Infrastructure:**

- **Firmware:** Debug build flashed via probe-rs
- **RTT capture:** `probe-rs run` flashes the firmware and
  streams defmt output to a log file
- **MQTT subscriber:** `test/mqtt-subscriber/` records all
  messages on the device topic as JSONL
- **Broker:** Mosquitto container (`test/broker/`) with
  mutual TLS, managed by `mise run broker:start`
- **TLS certificates:** Generated by
  `test/scripts/gen-tls-cert.sh`
- **Validator:** Cucumber-RS binary (`test/smoke-validator/`)
  parses RTT log and MQTT JSONL against feature files

The integration test pipeline (`mise run test:integration`)
orchestrates all five layers in sequence: host tests,
device bring-up, firmware flash, RTT + MQTT capture, and
Cucumber-RS validation.

## Running Tests

### Layer 1: Host Unit Tests

```sh
mise run test:host
```

The raw `cargo test` invocations behind this task-- and every other
pre-commit gate command-- are canonical in
[`testing_gates.md`](../../../.agents/rules/testing_gates.md); this
page does not duplicate them.

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
mise run test:smoke [<board>...]
```

Routes each resolved board (arguments, the `IOT_BOARDS` pin, or the
default) to its suite in `test/smoke/`.  The feather suite flashes
debug firmware, captures RTT output, and validates boot milestones
via Cucumber-RS at the standard tier (165 seconds, 5-second sample
interval).

### Layers 4-5: Tiered Integration Tests

The smoke and end-to-end tests support three duration tiers
to balance fast feedback against thorough sensor validation.
Tier selection controls which Cucumber-RS scenarios run:

| Tier | Command | Duration | Interval | What it adds |
|------|---------|----------|----------|--------------|
| Standard | `mise run test:integration` | 165s | 5s | Boot, MQTT, WFI, errors, correlation |
| Extended | `mise run test:integration-extended` | 300s | 15s | + CO2 conditioning (`@extended`) |
| Full | `mise run test:integration-full` | 780s | 60s | + VOC/NOx conditioning (`@full`) |

**Tag-based filtering:** The Cucumber-RS validator skips
scenarios tagged `@extended` or `@full` unless the test
tier is high enough.  This lets you run `test:integration`
for quick feedback and save the longer tiers for thorough
validation or pre-release testing.

To run just the smoke test portion at a specific tier, use the
matching task name-- tier selection is a separate task per tier, not
a flag:

| Tier | Command | Duration | Interval |
|------|---------|----------|----------|
| Standard | `mise run test:smoke` | 165s | 5s |
| Extended | `mise run test:smoke-extended` | 300s | 15s |
| Full | `mise run test:smoke-full` | 780s | 60s |

### Full Pipeline

```sh
mise run test:integration
```

Runs Layers 1, 2, 4, and 5 in sequence at the standard
tier.  Use `test:integration-extended` or
`test:integration-full` for longer-duration tiers.

### Static Analysis

Formatting:

```sh
cargo fmt --all -- --check
```

Linting (illustrative-- board crates select mutually-exclusive
stm32-metapac chip features, so the full per-board invocation set
lives in [`testing_gates.md`](../../../.agents/rules/testing_gates.md),
which is canonical):

```sh
cargo clippy -p iot-core --all-features \
  --target thumbv7em-none-eabihf -- -D warnings
```

## When to Add a Test

Use this decision matrix to decide which layer needs a new
test:

| You changed... | Add a test in... |
|:-------------|:---------------|
| `core/` logic or data types | Layer 1 (host unit test) |
| BSP peripheral init or pin mapping | Layer 2 (bring-up test) |
| Boot sequence or milestone ordering | Layer 4 (smoke test covers you) |
| RTIC task interaction or channel wiring | Layer 4 for now; Layer 3 when available |
| MQTT payload format or TLS config | Layer 1 (schema/contract test) + Layer 5 |
| Sensor conditioning thresholds | Layer 5 (extended or full tier) |
| WFI/sleep behavior or wake sources | Layer 4 (WFI smoke feature) |
| `defmt` log messages used as milestones | Layer 4 (boot sequence feature) |

When in doubt: if the code can be tested without hardware,
write a host test.  Hardware test cycles are expensive-- save
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

Hardware-in-the-loop (HIL) testing will use the bench's Saleae
Logic MSO 2x100 for automated protocol validation-- capturing SPI,
I2C, and UART traffic during test runs and asserting timing
and data correctness programmatically.  This is planned for
when the self-hosted runner infrastructure is in place.

See [HIL Measurements](../projects/ars-toolhead-sensor/hil-measurements.md)
for a worked example of what the analyzer captures and why.

---

See [ADR-009](../architecture/decisions.md#adr-009-test-strategy-and-the-embedded-test-pyramid)
for the full decision record, including alternatives
considered and consequences.
