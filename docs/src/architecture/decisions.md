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
**Updated:** 2026-04-05
**Status:** Accepted (updated with implementation details)

**Context:**
Need to choose serialization format for MQTT payloads.

**Decision:** Use Protocol Buffers (protobuf) for telemetry
payloads via `micropb` 0.6.  Use JSON via `serde-json-core`
for AWS Device Shadows and IoT Jobs (required by AWS reserved
topics).

**Rationale:**

- 82% smaller telemetry payloads (32 B vs 220 B JSON)
- Deterministic encoding (~7–11 µs on Cortex-M4 at 168 MHz)
- Native AWS IoT Rules Engine `decode()` support
- Type-safe with compile-time schema validation
- `micropb` is purpose-built for no_std/no_alloc environments

**Consequences:**

- Requires `.proto` schema files in `proto/` and
  `micropb-gen` code generation in `core/build.rs`
- Requires MSRV 1.88.0 (micropb requirement)
- AWS Device Shadows still need JSON — dual format approach
- Less human-readable; `buf` tooling for schema management
- AWS Rules Engine needs `.desc` descriptor files in S3

**Alternatives Considered:**

- JSON: Human-readable but 82% larger, slower parsing
- CBOR: 42 B (vs 32 B Protobuf), no native AWS `decode()`
- MessagePack: Less tooling, no AWS Rules Engine support
- `prost`: Requires `alloc` — incompatible with no_std

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

## ADR-007: CloudEvents `binary_data` Over `proto_data`

**Date:** 2026-04-05
**Status:** Accepted

**Context:**
The CloudEvents Protobuf Event Format v1.0.3-wip defines three
data variants: `binary_data` (bytes), `text_data` (string), and
`proto_data` (`google.protobuf.Any`).  The spec states that
`proto_data` MUST be used "where the data is a protobuf message."
However, `google.protobuf.Any` requires runtime type URLs and
dynamic dispatch, which are fundamentally incompatible with
no_std/no_alloc embedded systems.  Furthermore, `micropb` — the
only viable Protobuf crate for our constraints — does not
support `google.protobuf.Any`.

**Decision:** Use the `binary_data` (bytes) field to carry
Protobuf-encoded sensor telemetry inside the CloudEvents
envelope.  Identify the inner schema via the CloudEvents `type`
field (e.g., `iot.sen66.v1`).

**Rationale:**

- micropb 0.6 does not support `google.protobuf.Any`
- `binary_data` saves 12–20 bytes per message (no type URL
  overhead), reducing CloudEvents PB from ~96 B to ~80 B
- Encoding is simpler: two-pass (inner message → bytes →
  envelope) with no `Any::pack()` complexity
- Cloud-side routing uses the CloudEvents `type` field, not
  type URLs — functionally equivalent
- AWS IoT Rules Engine double-decode works with both variants
- Widely practiced in production IoT systems

**Consequences:**

- Technically non-compliant with CloudEvents Protobuf spec
  §3.2 ("data MUST be stored in `proto_data`" for protobuf)
- Cloud consumers must know the inner schema out-of-band
  (via `type` field) rather than via `type_url`
- The `datacontenttype` attribute should be set to
  `application/protobuf` to signal the binary encoding

**Alternatives Considered:**

- `proto_data` with manual `Any` encoding: Possible but adds
  complexity, 12–20 B overhead, and requires string formatting
  for type URLs on the MCU
- Custom envelope (skip CloudEvents): Loses interoperability
  with EventBridge and cross-system consumers
- CloudEvents JSON: 3.5× larger than CloudEvents PB; negates
  the benefit of Protobuf telemetry

---

## ADR-008: micropb for Protobuf Encoding

**Date:** 2026-04-05
**Status:** Accepted

**Context:**
Need a Protobuf library that works in `no_std` without an
allocator on Cortex-M4 (STM32F405, 192 KB SRAM, 1 MB flash).

**Decision:** Use `micropb` 0.6 with `micropb-gen` for
build-time code generation from `.proto` files.

**Rationale:**

- Purpose-built for no_std/no_alloc embedded systems
- Uses `heapless` containers for bounded collections
- TOML configuration files for per-field capacity budgets
- ~8–12 KB flash, <1 KB RAM for typical sensor schemas
- Deterministic encoding: ~1,200–1,800 cycles per message
- Proto3 support with full encode/decode
- Actively maintained and widely used in the Rust ecosystem

**Consequences:**

- MSRV 1.88.0 — must track recent stable Rust
- No `google.protobuf.Any` support (see ADR-007)
- No Protobuf Editions — proto3 only
- No defmt integration — requires manual `Format` impls
- Every `string`/`bytes`/`repeated`/`map` field needs explicit
  `max_len()` configuration in build.rs or TOML config
- `micropb-gen` requires `protoc` on the build host

**Alternatives Considered:**

- `prost`: Mature, widely used — requires `alloc` (disqualified)
- `femtopb`: Smallest footprint, zero-panic — borrow-based API
  complicates ownership across RTIC tasks, API still evolving
- `nanopb` (C via FFI): Proven but adds C toolchain dependency

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

*Update this document when significant architectural decisions
are made.*
