# Embedded Rust IoT Firmware

## Project Overview

This project implements a modular, multi-project embedded IoT
firmware framework using Rust in a `no_std` environment.  The
framework uses a **Cargo workspace** architecture with **board
profiles** that combine specific hardware, peripherals, and
applications, and it hosts more than one project built on that
shared platform: the air-quality environmental monitoring node
described below, and the ARS toolhead sensor (STM32N657,
scaffold only).  Per-project documentation lives under
`docs/src/projects/`, alongside each project's board profile(s)
in `boards/` (see the Projects links below).

### Architecture

The project uses a board profile architecture where each profile in
`boards/` represents:
- A specific board type (e.g., Feather STM32F405, NUCLEO-H753ZI,
  or the scaffolded ST NUCLEO-N657X0-Q)
- Peripheral components (e.g., Ethernet, sensors)
- Application purpose (e.g., environmental telemetry, bring-up
  rig, toolhead sensor)

Shared code lives in workspace crates:
- `core/`-- Platform-agnostic business logic
- `hal-abstractions/`-- Hardware abstraction traits

### Key Features

- **Real-time Operation**: RTIC 2.x framework with formal verification
  via Stack Resource Policy
- **Secure Connectivity**: TLS 1.3 encrypted MQTT communication with
  AWS IoT Core
- **Environmental Monitoring**: SEN66 air quality sensor integration
  (PM, CO2, VOC, NOx, temperature, humidity)
- **Local Display**: E-ink status dashboard with ultra-low power consumption
- **CAN Bus Gateway** (planned): Bidirectional CAN <-> MQTT message forwarding
- **Secure OTA Updates**: Firmware updates with signature verification
  and atomic rollback

### Hardware Platform

- **MCU**: STM32F405RG (ARM Cortex-M4F @ 168 MHz)
- **Memory**: 1 MB Flash, 192 KB SRAM (128 KB main + 64 KB CCM)
- **Network**: W5500 Ethernet controller with hardware TCP/IP offload
- **Sensors**: Sensirion SEN66 environmental sensor
- **Display**: SSD1681 E-ink display (200x200 pixels)

## Documentation Standards

This documentation follows **IEEE 29148** (systems and software
requirements engineering) and **IEEE 16326** (project management)
standards in a lightweight, agile manner suitable for embedded
development.

## Quick Links

### Requirements
- [System Requirements Specification](./system_requirements.md)--
  Functional and non-functional requirements (IEEE 29148)

### Project Management
- [Project Roadmap](./roadmap.md)-- Implementation phases and
  milestones (IEEE 16326)
- [Risk Register](./risk_register.md)-- Active and mitigated project risks

### Architecture
- [Architecture Decisions](./architecture/decisions.md)-- Key
  architectural decision records (ADRs)

### Development
- [Testing Strategy](./development/testing.md)-- Test methodology
  and CI/CD pipeline

### Projects
- [ARS Toolhead Sensor](./projects/ars-toolhead-sensor/README.md)--
  Project overview (scaffold)
- [ARS Toolhead Sensor Hardware](./projects/ars-toolhead-sensor/hardware.md)--
  Hardware platform details (scaffold)

## Getting Started

### Prerequisites

- Rust 1.88+ (workspace MSRV: `rust-toolchain.toml` pins the exact
  toolchain) with the `thumbv7em-none-eabihf` target and `rust-src`
  component
- probe-rs tools: `cargo install probe-rs-tools cargo-embed cargo-flash`
- Debug probe compatible with probe-rs (e.g., J-Link, ST-Link)

### Building the Firmware

```bash
# From workspace root - builds default board (feather-stm32f405)
cargo build --release

# Build specific board profile
cargo build -p feather-stm32f405 --release
```

### Flashing to Hardware

```bash
# Default board (feather)
cargo run --release
cargo embed --release

# Build and flash a different board profile; see the board's own
# README (e.g. boards/nucleo-h753zi/README.md) for its exact
# probe/chip selectors
cargo build -p nucleo-h753zi --release --target thumbv7em-none-eabihf
```

### Running Tests

```bash
# Host-side unit tests (for core/ and hal-abstractions/)
mise run test:host
```

Plain `cargo test` fails here: the workspace's default target is the
embedded `thumbv7em-none-eabihf`, which cannot link host tests.  See
[`testing_gates.md`](../../.agents/rules/testing_gates.md) for the
full pre-commit command set and gate criteria.

## Project Status

**Firmware Track**

**Recent Completion**: Phases 0-3 ✅
- Phase 0-- Workspace Migration: multi-device Cargo workspace
  with board profile architecture
- Phase 1-- Core Platform: shared platform-agnostic crates
  (`core/`, `hal-abstractions/`)
- Phase 2-- Network Stack: W5500 Ethernet, TLS 1.3, and MQTT
  v5.0 messaging
- Phase 3-- Sensor Integration: SEN66 environmental sensor (PM,
  CO2, VOC, NOx, temperature, and humidity)

**Current Phase**: Phase 4-- Security Foundation (Not Started)

**Framework Track**: Active
- Agentic AI scaffolding: ✅ Complete (2026-07-18)-- project
  rules, skills, and subagent definitions under `.agents/`
- NUCLEO-H753ZI bring-up: ✅ Promoted to workspace member
  (2026-07-19) after hardware-verified bring-up
  (`boards/nucleo-h753zi/`); planned home for the ARS DAC->ADC
  loopback rig and Ethernet bring-up
- ARS toolhead sensor project: Scaffolded-- ST NUCLEO-N657X0-Q
  board profile (`boards/nucleo-n657x0/`), workspace-excluded

See the [Roadmap](./roadmap.md) for detailed status and upcoming milestones.

## Contributing

This is a reference implementation and learning project.
Contributions, suggestions, and feedback are welcome.

## License

Licensed under Apache-2.0.  Copyright Lucas Yamanishi.  See
[`LICENSE`](../../LICENSE) for the full text.

---

*Last updated: 2026-07-19*
