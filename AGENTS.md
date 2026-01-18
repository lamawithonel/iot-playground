# Agent Instructions for iot-playground

## Project Overview

This is an embedded Rust IoT framework for STM32 and Microchip ATSAM MCUs.
It is designed for financial services applications requiring real-time
guarantees and security.

## Critical Constraints

### RTIC-First Architecture

- **REQUIRED:** Use RTIC 2.x for all task scheduling
- **REQUIRED:** Use `rtic-sync` for inter-task communication
- **FORBIDDEN:** Do not use `embassy-executor` crate
- **FORBIDDEN:** Do not use Embassy's async executor features

### Embassy Usage

- **ALLOWED:** Embassy HAL crates (`embassy-stm32`, `embassy-nrf`, etc.)
- **ALLOWED:** Embassy PAC crates
- **ALLOWED:** Embassy network crates (`embassy-net`, `embassy-net-wiznet`)
- **FORBIDDEN:** `embassy-executor`
- **PREFER:** `rtic-sync` over `embassy-sync` where possible

### Interrupt-Driven Design

- **REQUIRED:** Use WFI/Sleep between interrupt events
- **REQUIRED:** Hardware timers for periodic interrupts
- **REQUIRED:** EXTI for external peripheral interrupts
- **FORBIDDEN:** Busy-wait loops (except brief hardware delays)

### Memory Model

- **REQUIRED:** `no_std` environment
- **REQUIRED:** No heap allocation (no `alloc` crate)
- **ALLOWED:** `heapless` collections
- **ALLOWED:** `static_cell` for static allocation

## Directory Structure

```
iot-playground/
├── core/               # Platform-agnostic business logic (NO hardware deps)
├── hal-abstractions/   # Traits for hardware abstraction
├── boards/             # Board Support Packages (BSPs)
│   └── {board-name}/   # One BSP per supported board
├── apps/               # Application binaries
└── docs/               # Framework documentation (mdBook)
```

## Device Tiers

- **Tier 1 (Minimal):** ≤128KB RAM, no TLS, basic I/O only
- **Tier 2 (Connected):** ≥192KB RAM, TLS/MQTT capable, primary target
- **Tier 3 (Gateway):** ≥512KB RAM, multi-protocol, edge compute

## Code Style

- All files MUST have `#![deny(warnings)]`
- All files SHOULD have `#![deny(unsafe_code)]` unless unsafe is required
- Unsafe code MUST be isolated and documented
- All public items MUST have doc comments
- Use `defmt` for logging, not `log` or `println!`

## Testing

- Platform-agnostic code in `core/` MUST have unit tests
- BSP code is tested via integration tests on hardware
- Use `defmt-test` for on-device tests

## Dependencies

Prefer crates in this order:
1. RTIC ecosystem (`rtic`, `rtic-sync`, `rtic-monotonics`)
2. Embassy ecosystem (`embassy-stm32`, `embassy-net`, etc.)
3. `embedded-hal` ecosystem
4. Other well-maintained embedded crates

## File Naming

- Use snake_case for all Rust files
- BSP crates named: `{board-name}` (e.g., `feather-stm32f405`)
- App crates named: `{descriptive-name}` (e.g., `mqtt-sensor-node`)
