# Embedded Rust IoT Firmware

Welcome to the documentation for the Embedded Rust IoT Firmware project.

## Project Overview

This project implements a reference embedded IoT firmware for the **STM32F405RG microcontroller** (Adafruit Feather STM32F405 Express board), written entirely in Rust using a `no_std` environment.

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

- Rust 1.75+ with `thumbv7em-none-eabihf` target
- `probe-rs` or `cargo-embed` for flashing and debugging
- J-Link debugger for SWD access and RTT logging

### Building the Firmware

```bash
cd feather-stm32f405
cargo build --release --target thumbv7em-none-eabihf
```

### Flashing to Hardware

```bash
cargo embed --release
```

### Running Tests

```bash
# Host-side unit tests
cargo test --lib

# On-device integration tests (requires hardware)
cargo test --target thumbv7em-none-eabihf
```

## Project Status

**Current Phase**: Phase 2 - Network Stack (In Progress)

See the [Roadmap](./roadmap.md) for detailed status and upcoming milestones.

## Contributing

This is a reference implementation and learning project. Contributions, suggestions, and feedback are welcome.

## License

See the repository root for license information.

---

*Last updated: 2026-01-12*
