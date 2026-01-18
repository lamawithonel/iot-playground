# Embedded Rust IoT Firmware

Welcome to the documentation for the Embedded Rust IoT Firmware project.

## Project Overview

This project implements a multi-device embedded IoT firmware framework using Rust in a `no_std` environment. The framework uses a **Cargo workspace** architecture with **board profiles** that combine specific hardware, peripherals, and applications.

### Architecture

The project uses a board profile architecture where each profile in `boards/` represents:
- A specific board type (e.g., Feather STM32F405, Feather M4 CAN)
- Peripheral components (e.g., Ethernet, sensors, CAN)
- Application purpose (e.g., MQTT gateway, PTP server)

Shared code lives in workspace crates:
- `core/` - Platform-agnostic business logic
- `hal-abstractions/` - Hardware abstraction traits

### Key Features

- **Real-time Operation**: RTIC 2.x framework with formal verification via Stack Resource Policy
- **Secure Connectivity**: TLS 1.3 encrypted MQTT communication with AWS IoT Core
- **Environmental Monitoring**: SEN66 air quality sensor integration (PM, CO2, VOC, NOx, temperature, humidity)
- **Local Display**: E-ink status dashboard with ultra-low power consumption
- **CAN Bus Gateway**: Bidirectional CAN ↔ MQTT message forwarding
- **Secure OTA Updates**: Firmware updates with signature verification and atomic rollback

### Hardware Platform

- **MCU**: STM32F405RG (ARM Cortex-M4F @ 168 MHz)
- **Memory**: 1 MB Flash, 192 KB SRAM (128 KB main + 64 KB CCM)
- **Network**: W5500 Ethernet controller with hardware TCP/IP offload
- **Sensors**: Sensirion SEN66 environmental sensor
- **Display**: SSD1681 E-ink display (200×200 pixels)
- **CAN**: TJA1051 transceiver at 1 Mbps

## Documentation Standards

This documentation follows **IEEE 29148** (systems and software requirements engineering) and **IEEE 16326** (project management) standards in a lightweight, agile manner suitable for embedded development.

## Quick Links

### Requirements
- [System Requirements Specification](./system_requirements.md) - Functional and non-functional requirements (IEEE 29148)

### Project Management
- [Project Roadmap](./roadmap.md) - Implementation phases and milestones (IEEE 16326)
- [Risk Register](./risk_register.md) - Active and mitigated project risks

### Architecture
- [Architecture Decisions](./architecture/decisions.md) - Key architectural decision records (ADRs)

### Development
- [Testing Strategy](./development/testing.md) - Test methodology and CI/CD pipeline

## Getting Started

### Prerequisites

- Rust 1.75+ with `thumbv7em-none-eabihf` target and `rust-src` component
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

# Select different board via environment variable
PROBE_RS_CONFIG_PRESET=microbit cargo run --release
PROBE_RS_CONFIG_PRESET=stm32f3 cargo run --release

# Or use cargo embed with --chip flag
cargo embed --chip microbit --release
```

### Running Tests

```bash
# Host-side unit tests (for core/ and hal-abstractions/)
cargo test --lib

# Board-specific tests
cargo test -p feather-stm32f405
```

## Project Status

**Current Phase**: Phase 2 - Network Stack (In Progress)

**Recent Completion**: Phase 0 - Workspace Migration ✅
- Multi-device Cargo workspace with board profile architecture
- Native probe-rs integration for flexible board selection
- Skeleton crates for shared code (core/, hal-abstractions/)

See the [Roadmap](./roadmap.md) for detailed status and upcoming milestones.

## Contributing

This is a reference implementation and learning project. Contributions, suggestions, and feedback are welcome.

## License

See the repository root for license information.

---

*Last updated: 2026-01-18*
