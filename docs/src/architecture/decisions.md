# Architecture Decision Records
## Embedded Rust IoT Firmware

This document captures key architectural decisions using a lightweight ADR format.

---

## ADR-001: RTIC vs Embassy Executor

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need to choose between RTIC and Embassy executor for task scheduling in a real-time embedded system.

**Decision:** Use RTIC 2.x for task scheduling, Embassy for HAL and drivers.

**Rationale:**
- RTIC provides formal verification via Stack Resource Policy (SRP)
- Hard real-time guarantees required for interrupt latency (<500μs)
- Embassy executor is cooperative and cannot provide hard deadlines

**Consequences:**
- Steeper learning curve combining two frameworks
- Some driver incompatibilities (e.g., cannot use Embassy executor-dependent drivers)
- Must use `rtic-sync` instead of `embassy-sync` for inter-task communication where possible
- Embassy HAL works well; Embassy executor features must be avoided

**Alternatives Considered:**
- Pure Embassy: Rejected due to lack of formal real-time guarantees
- Pure RTIC with PAC: Would work but loses Embassy HAL conveniences

---

## ADR-002: W5500 vs ESP32 WiFi

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need network connectivity for MQTT communication with AWS IoT Core.

**Decision:** Use W5500 hardwired Ethernet controller.

**Rationale:**
- Hardware TCP/IP offload reduces MCU load
- Deterministic behavior (no WiFi interference/reconnection issues)
- Easier debugging (Wireshark on physical network)
- Industrial reliability (no RF concerns)

**Consequences:**
- Requires physical Ethernet cable (no wireless mobility)
- Additional SPI peripheral usage
- Simpler power management (no WiFi radio)

**Alternatives Considered:**
- ESP32 as WiFi coprocessor: More complex, less deterministic
- STM32 with built-in Ethernet (F7/H7): Higher cost, different board

---

## ADR-003: Protocol Buffers vs JSON

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need to choose serialization format for MQTT payloads.

**Decision:** Use Protocol Buffers (protobuf) for all MQTT message payloads.

**Rationale:**
- Smaller message size (critical for constrained networks)
- Faster serialization/deserialization
- Type-safe with compile-time schema validation
- Industry standard in IoT and financial services

**Consequences:**
- Less human-readable without protobuf decoder
- Requires `.proto` schema files and code generation
- Need `prost` or similar no_std-compatible library

**Alternatives Considered:**
- JSON: Human-readable but larger, slower parsing
- MessagePack: Good compromise but less tooling support
- CBOR: Similar to MessagePack, less common

---

## ADR-004: E-ink vs OLED Display

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need local display for device status and sensor readings.

**Decision:** Use E-ink display (SSD1681, 200x200).

**Rationale:**
- Ultra-low power consumption (power only during updates)
- Sunlight readable (high contrast)
- Persistent display without power (shows last state during sleep)
- Appropriate for sensor data that updates every 60 seconds

**Consequences:**
- Slow refresh (~10 seconds for full update)
- Limited to monochrome or limited color
- More complex update logic (partial refresh optimization)

**Alternatives Considered:**
- OLED: Faster updates but higher power, burn-in risk
- LCD: Middle ground but requires backlight power

---

## ADR-005: TLS Library Selection

**Date:** 2026-01-12  
**Status:** Accepted

**Context:**
Need TLS 1.3 support for secure MQTT connections without heap allocation.

**Decision:** Use `embedded-tls` library (commit dccd966).

**Rationale:**
- No allocator required - works in pure `no_std` environment
- TLS 1.3 support with AES-128-GCM-SHA256 cipher suite
- Compatible with Embassy async traits (`embedded-io-async`)
- Active maintenance and embedded-focused design

**Consequences:**
- **No RSA certificate support** - servers must use ECDSA
- Limited cipher suites (primarily AES-GCM)
- No SHA-512/SHA-3 support
- Certificate verification optional (enabled in production)

**Alternatives Considered:**
- `rustls`: Excellent library but requires allocator
- `wolfSSL` (via FFI): FIPS-certified but C library dependency
- `mbedTLS` (via FFI): Widely used but larger footprint, C dependency

**Future Reconsideration Triggers:**
- Hardware upgrade with more memory (can use `rustls`)
- FIPS 140-3 certification requirement (would need `wolfSSL`)
- RSA certificate requirement from cloud provider

---

## ADR-006: Cargo Workspace Architecture

**Date:** 2026-01-18  
**Status:** Accepted

**Context:**
Need to support multiple board profiles (board type + peripherals + application purpose) in a single repository while sharing common code like network stacks and HAL abstractions.

**Decision:** Use Cargo workspace with board profile architecture.

**Rationale:**
- Centralized dependency management via workspace.dependencies
- Shared code in `core/` (platform-agnostic logic) and `hal-abstractions/` (hardware traits)
- Board profiles in `boards/` directory (e.g., `feather-stm32f405`)
- Each board profile = specific hardware + peripherals + application
- Single `.cargo/config.toml` at workspace root with common linker flags
- Board selection via probe-rs native `PROBE_RS_CONFIG_PRESET` environment variable
- Board-specific configurations centralized in root `Embed.toml`

**Consequences:**
- All boards share common Cargo profile settings (panic = "abort")
- Workspace-level dependency versions ensure consistency
- Board profiles can share code via workspace crates
- `memory.x` linker scripts handled per-board via `build.rs`
- Scalable: easy to add new board profiles without duplication
- Build from workspace root: `cargo run --release` or `cargo embed --release`

**Alternatives Considered:**
- Separate repositories per board: Would duplicate network/HAL code
- Git submodules: Complex dependency management, harder to refactor
- Monolithic single crate: Doesn't scale to multiple board types

**Board Profile Examples:**
- `boards/feather-stm32f405/` - STM32F405 + Ethernet + sensors + MQTT gateway
- Future: `boards/feather-ptp-server/` - STM32F405 + Ethernet + GPS + PTP server
- Future: `boards/feather-m4-can/` - SAMD51 + CAN + sensors

---

## Template for New ADRs

```markdown
## ADR-XXX: [Title]

**Date:** YYYY-MM-DD  
**Status:** [Proposed | Accepted | Deprecated | Superseded]

**Context:**
[What is the issue we're addressing?]

**Decision:** [What did we decide?]

**Rationale:**
[Why did we make this decision?]

**Consequences:**
[What are the implications?]

**Alternatives Considered:**
[What other options were evaluated?]
```

---

*Update this document when significant architectural decisions are made.*
