# Project Roadmap
## Embedded Rust IoT Firmware

**Version:** 3.0
**Last Updated:** 2026-04-05
**Project Phase:** Phase 2 (Network Stack) / Phase 3 Complete

---

## 1. Project Scope

### 1.1 Objectives

Build a reference implementation for embedded Rust IoT firmware
on STM32F405RG, demonstrating:

- RTIC 2.x real-time framework with Embassy HAL
- Secure MQTT connectivity via TLS 1.3
- Protocol Buffers telemetry with cloud-side CloudEvents
  enrichment
- Environmental sensor integration (SEN66)
- AWS IoT Core integration with Rules Engine decoding
- AsyncAPI contract for MQTT API documentation
- E-ink display status dashboard
- Secure OTA firmware updates via `embassy-boot-stm32`

### 1.2 Deliverables

- Firmware binary for STM32F405RG (Adafruit Feather)
- Protocol Buffer schemas (`proto/`) with CI linting
- AsyncAPI contract (`docs/api/asyncapi.yaml`) for MQTT API
- AWS IoT Core descriptor files for Rules Engine
- Documentation site (GitHub Pages via mdBook)
- Test infrastructure (GitHub Actions CI/CD)
- Container-based local test environment (Podman/Docker)

### 1.3 Out of Scope

- AWS IoT Core infrastructure provisioning (Terraform/IaC)
- On-device CloudEvents envelope encoding (cloud-side only;
  see §7 item 9)
- CAN bus gateway (deferred to backlog; see
  [Backlog: CAN Gateway](#backlog-can-gateway))
- Hardware PCB design
- Production manufacturing
- Buf Schema Registry (BSR) hosting

---

## 2. Current Status

### 2.0 Workspace Migration ✅ Complete

**Completed:**

- [x] Cargo workspace with centralized dependency management
- [x] Board profile architecture (`boards/` directory)
- [x] Migrated `feather-stm32f405` to `boards/feather-stm32f405/`
- [x] Single root `.cargo/config.toml` with probe-rs runner
- [x] Centralized board configurations in root `Embed.toml`
- [x] Native probe-rs board selection via `PROBE_RS_CONFIG_PRESET`
- [x] Skeleton crates: `core/` and `hal-abstractions/`
- [x] Build system fixes for `memory.x` linker script handling
- [x] Documentation updates (README, AGENTS.md, ADRs)

### 2.1 Phase 2: Network Stack (In Progress)

**Completed:**

- [x] W5500 SPI driver and hardware initialization
- [x] DHCP and Layer 2 networking
- [x] Network stack abstraction (`network` module)
- [x] SNTP client with RTC synchronization
- [x] TLS 1.3 handshake (using `embedded-tls`)
- [x] Local MQTT broker test environment (Podman/Docker)
- [x] MQTT v5.0 persistent connection with reconnection
- [x] MQTT keep-alive handling
- [x] Device identification using STM32 UID
- [x] Decoupled error architecture
- [x] Platform-agnostic `core/` crate with host-side unit tests

**In Progress:**

- [ ] Interrupt-driven packet reception (EXTI2)
- [ ] WFI/Sleep mode between messages
- [ ] Full AWS IoT Core integration

### 2.2 TLS Library Decision

**Status:** ✅ Resolved

**Decision:** Use `embedded-tls` (commit
dccd96634679d52a7eac7a0ed216b9c24dbfb122)

**Rationale:**

- No allocator required (works in `no_std` without heap)
- TLS 1.3 support with modern cipher suites
- Compatible with Embassy async traits

**Known Limitations:**

- **No RSA support** — servers must use ECDSA certificates
- **No SHA-512/SHA-3** — only SHA-256/SHA-384 available
- **ECDSA only** — supports secp256r1 and secp384r1 curves
- **Single cipher suite** — TLS_AES_128_GCM_SHA256

**Alternatives Evaluated:**

| Library | Status | Reason Not Selected |
|---------|--------|---------------------|
| `rustls` | Deferred | Requires allocator |
| `wolfSSL` (`wolfcrypt-rs`) | Deferred | C library dependency |
| `mbedTLS` | Deferred | Larger footprint, C dependency |

**Future Consideration:** Revisit TLS library when:

- Hardware with more memory is available (STM32F7/H7)
- FIPS 140-3 certification is required (wolfSSL)
- RSA certificate support becomes mandatory

### 2.3 Serialization & Architecture Research ✅ Complete

**Status:** ✅ Research complete — ready for implementation

Three parallel research tracks evaluated serialization,
inter-task communication, cloud integration, and API contract
specification across ~9,000 lines of research.

**Serialization (Protobuf):**

- **micropb 0.6** selected as the Protobuf library
  (no_std/no_alloc, ~8–12 KB flash, <1 KB RAM)
- **CloudEvents cloud-side only** — devices send raw
  Protobuf (32 B); AWS IoT Rules Engine adds CloudEvents
  envelope before EventBridge
- **Encoding in sensor task** — 7–11 µs at 168 MHz;
  negligible vs. 2–5 ms I2C read
- **AWS double-decode** confirmed — Rules Engine supports
  two `decode()` calls per SQL expression, Basic Ingest
  compatible

**API Contract (AsyncAPI):**

- **AsyncAPI v3.1.0** adopted as documentation and contract
  specification — zero MCU code, CI-verifiable
- Complements Protobuf (payload schema) and CloudEvents
  (envelope standard)
- Describes MQTT topics, QoS settings, server endpoints,
  and security schemes in a single machine-readable document

See [ADR-007](./architecture/decisions.md#adr-007-cloudevents-binary_data-over-proto_data)
and [ADR-008](./architecture/decisions.md#adr-008-micropb-for-protobuf-encoding)
for formal decisions.

---

## 3. Implementation Phases

### Phase 0: Workspace Migration ✅ Complete

- [x] Create Cargo workspace with resolver = "2"
- [x] Define workspace.package and workspace.dependencies
- [x] Move feather-stm32f405 to boards/ directory
- [x] Create skeleton crates: core/ and hal-abstractions/
- [x] Single root .cargo/config.toml with common settings
- [x] Root Embed.toml with board presets
- [x] Board selection via PROBE_RS_CONFIG_PRESET
- [x] Build script (build.rs) for memory.x linker script
- [x] Remove legacy board directories
- [x] Update documentation (README, AGENTS.md, ADRs, roadmap)

### Phase 1: Core Platform ✅ Complete

- [x] GPIO and LED control
- [x] SWD debugging with RTT logging

### Phase 2: Network Stack 🔄 In Progress

- [x] Verify network pin assignments with logic analyzer
- [x] W5500 SPI driver
- [x] DHCP and Layer 2 networking
- [x] Network stack abstraction
- [x] SNTP client (`sntpc`)
- [x] TLS 1.3 handshake
- [x] MQTT v5.0 client with TLS 1.3 (basic connectivity)
- [x] Device identification using STM32 UID
- [x] Decoupled error architecture
- [x] MQTT persistent connection with exponential backoff
- [x] MQTT keep-alive handling (configurable interval)
- [ ] Event-driven MQTT message handling
- [ ] Shared MQTT connection resource (RTIC Shared)
- [ ] WFI/Sleep mode between messages
- [ ] Interrupt-driven packet reception (EXTI2)
- [ ] AWS IoT Core endpoint and TLS cert verification

**Phase 2 Remaining Work:**

- Wire EXTI2 interrupt for W5500 packet reception
- Implement proper interrupt-driven wake from WFI
- Event-driven MQTT message handling
- AWS IoT Core endpoint configuration and TLS cert
  verification

### Phase 3: Sensor Integration ✅ Complete

- [x] SEN66 I2C driver (via `sen6x` crate with CRC)
- [x] Periodic sensor readings (configurable interval)
- [x] Sensor conditioning guards (SEN66 warmup tracking)
- [x] Publish sensor data via MQTT (JSON — migrating to
  Protobuf in Phase 5)
- [x] Platform-agnostic sensor types and conditioning
  in `core/`

### Phase 4: Security Foundation ⏳ Not Started

Establish security posture before non-lab deployments.
Extracted from a formerly monolithic security phase based
on cross-functional consensus that security is a property
threaded through all phases, not a late-stage bolt-on.

**Compliance Gate 1:** No deployment beyond dev-lab without
software certificate verification.

- [ ] Feature-gate `NoVerify` behind `danger-no-verify`
  Cargo feature with `compile_error!` on default builds
- [ ] Make `CryptoProvider` generic over verifier type
- [ ] Implement software-only CA-pinned certificate
  verification (ECDSA-P256 with embedded root CA)
- [ ] Write threat model document
- [ ] Design flash partitioning for dual-image OTA
  (A/B layout, metadata sector)
- [ ] Add `cargo audit` to CI pipeline
- [ ] Add SBOM generation to CI pipeline
- [ ] Create `SECURITY.md` with vulnerability reporting
  policy
- [ ] Create `CONTRIBUTING.md` with contributor guidelines

### Phase 5: Protobuf Telemetry ⏳ Not Started

Migrate MQTT telemetry encoding from JSON to Protocol
Buffers.  Devices send raw Protobuf on the wire; CloudEvents
enrichment occurs cloud-side via AWS IoT Rules Engine.

Defined by research completed in
[§2.3](#23-serialization--architecture-research--complete)
and [ADR-008](./architecture/decisions.md#adr-008-micropb-for-protobuf-encoding).

**Schema Infrastructure:**

- [ ] Create `proto/` directory with buf.yaml configuration
- [ ] Vendor Google well-known types (timestamp.proto)
- [ ] Write `proto/iot/v1/telemetry.proto` (SEN66 with
  deci-scaled integers, field numbers 1–10)
- [ ] Write `proto/iot/v1/common.proto` (shared types)
- [ ] Write TOML capacity configs for micropb-gen
- [ ] Add `buf` to mise tool dependencies

**Build Integration:**

- [ ] Add `micropb` 0.6 + `heapless` to workspace deps
- [ ] Write `core/build.rs` for micropb-gen code generation
- [ ] Generate Rust types from project `.proto` files and
  vendored `timestamp.proto`
- [ ] Re-export generated types from `core/src/lib.rs`

**Encoding Migration:**

- [ ] Refactor sensor task: encode Protobuf (replaces
  JSON encoding)
- [ ] Event ID generation: device UID short hash +
  monotonic sequence counter
- [ ] Source metadata: `urn:dev:stm32:{uid_hex}` in MQTT
  topic or user property

**AWS IoT Descriptor Generation:**

- [ ] mise task `proto:gen-desc` using
  `protoc --descriptor_set_out --include_imports`
- [ ] mise task `proto:lint` using `buf lint`
- [ ] mise task `proto:breaking` using `buf breaking`

**AsyncAPI Contract:**

- [ ] Create `docs/api/asyncapi.yaml` describing MQTT API
  (channels, operations, servers, bindings)
- [ ] Inline Protobuf schemas in asyncapi.yaml with CI
  sync check against canonical `.proto` files
- [ ] Add `@asyncapi/cli` to mise tool dependencies
- [ ] mise task `asyncapi:validate`
- [ ] CI step: validate AsyncAPI document
- [ ] Optional: generate HTML docs for GitHub Pages

**Validation:**

- [ ] Host-side unit tests for encode/decode round-trip
- [ ] Wire size validation (<40 B for raw SEN66 payload)
- [ ] On-device integration test with local MQTT broker
- [ ] CI: `buf lint` + `buf breaking --against main`

**Phase 5 Key Metrics:**

| Metric | JSON (current) | Protobuf (Phase 5) |
|--------|-----------------|---------------------|
| Payload size | ~220 B | ~32 B |
| Encoding time | ~25 µs | ~8 µs |
| Flash overhead | — | ~8–12 KB |
| RAM overhead | — | <1 KB |

### Phase 6: Display ⏳ Not Started

- [ ] Verify E-ink breakout pin assignments
- [ ] SSD1681 SPI communication
- [ ] Test patterns and text rendering
- [ ] Status dashboard with sensor data and device state
- [ ] LED blink patterns for device state (boot,
  connecting, active, error, conditioning)
- [ ] Partial refresh optimization

### Phase 7: Secure Boot & OTA ⏳ Not Started

Narrowed from a formerly monolithic security phase.
Security foundation (Phase 4) must be complete before this
phase begins.

- [ ] `embassy-boot-stm32` integration (conduct research
  spike before implementation to validate choice)
- [ ] Firmware signature verification
- [ ] MQTT-based firmware delivery (AWS IoT Jobs)
- [ ] Dual-image A/B update with flash partitioning
  (designed in Phase 4)
- [ ] Watchdog rollback protection (IWDG)
- [ ] Certificate rotation

**Compliance Gate 2:** No regulated-environment deployment
without signed firmware and secure boot chain.

### Phase 8: Fleet Operations ⏳ Not Started

Leverage AWS IoT Device Management where possible; build
only what requires custom firmware logic.

- [ ] AWS IoT Fleet Provisioning (claim certificates)
- [ ] Device Shadow integration (JSON via
  `serde-json-core`)
- [ ] IoT Jobs for command dispatch
- [ ] `device/{id}/status` MQTT health topic
- [ ] MQTT Last Will and Testament for disconnect
  detection
- [ ] Device quarantine workflow
- [ ] EventBridge integration (thin Lambda bridge)
- [ ] MQTT 5.0 user properties (AWS-only optimization)
- [ ] Audit logging for compliance

### Backlog: CAN Gateway

Deferred from the active roadmap by cross-functional
consensus.  CAN bus gateway is a different product vertical
that does not contribute to the sensor telemetry MVP.

- [ ] Verify CAN pin assignments with logic analyzer
- [ ] CAN bus configuration at 1 Mbps
- [ ] CAN → MQTT forwarding
- [ ] MQTT → CAN transmission

---

## 4. Test Strategy

See [Testing Strategy](./development/testing.md) for full
details.

### 4.1 Summary

| Test Type | Automation | Environment |
|-----------|------------|-------------|
| Formatting (`rustfmt`) | ✅ CI | Public runners |
| Linting (`clippy`) | ✅ CI | Public runners |
| Build verification | ✅ CI | Public runners |
| Host-side unit tests | ✅ CI | Public runners |
| Security audit (`cargo audit`) | 🔜 Phase 4 | Public runners |
| Proto linting (`buf`) | 🔜 Phase 5 | Public runners |
| AsyncAPI validation | 🔜 Phase 5 | Public runners |
| Container image builds | ✅ CI | Public runners |
| On-device integration | 🔄 Future | Self-hosted runner |
| Hardware validation | ❌ Manual | Local workstation |

### 4.2 Self-Hosted Runner (Future)

When on-device testing automation becomes valuable, a
self-hosted GitHub Actions runner will be configured:

- Single container on local workstation
- USB passthrough for J-Link/probe-rs access
- Network access to local Mosquitto container
- No Kubernetes required

---

## 5. Milestones

| Milestone | Status |
|-----------|--------|
| Phase 0: Workspace Migration | ✅ Complete |
| Phase 1: Core Platform | ✅ Complete |
| Phase 2: Network Stack | 🔄 In Progress |
| Phase 3: Sensor Integration | ✅ Complete |
| Phase 4: Security Foundation | ⏳ Not Started |
| Phase 5: Protobuf Telemetry | ⏳ Not Started |
| Phase 6: Display | ⏳ Not Started |
| Phase 7: Secure Boot & OTA | ⏳ Not Started |
| Phase 8: Fleet Operations | ⏳ Not Started |
| Documentation Site | ⏳ Not Started |

### 5.1 Compliance Gates

| Gate | Phase | Requirement |
|------|-------|-------------|
| Dev → Staging | 4 | Software certificate verification |
| Staging → Production | 7 | Signed firmware + secure boot |
| Production → Regulated | 8 | Audit logging + fleet management |

---

## 6. Memory Budget

Total MCU resources: 192 KB SRAM (128 KB main + 64 KB CCM),
1 MB flash.  Target utilization ceiling: 80% (154 KB RAM).

### 6.1 Current Allocations

| Component | Region | Size |
|-----------|--------|------|
| TLS read buffer | CCM RAM | 18 KB |
| TLS write buffer | CCM RAM | 16 KB |
| TCP RX/TX buffers | Main SRAM | 8 KB |
| MQTT packet buffer | Main SRAM | 2 KB |
| W5500 driver state | Main SRAM | 4 KB |
| Stack | Main SRAM | 16 KB |
| **Subtotal (current)** | | **~70 KB** |

### 6.2 Phase 5 Additions (Protobuf Telemetry)

| Component | Region | Size |
|-----------|--------|------|
| Protobuf encode scratch | Stack | 256 B |
| micropb runtime + types | Flash | ~8–12 KB |
| **Subtotal (Phase 5)** | | **~0.3 KB RAM** |

### 6.3 Projected Total (after Phase 5)

```
Main SRAM:  ~70 KB / 128 KB = 55% utilization
CCM RAM:    ~35 KB /  64 KB = 55% utilization
Total:     ~105 KB / 192 KB = 55% utilization
Budget:     154 KB ceiling  → 49 KB headroom ✅
Flash:     ~287 KB / 1 MB   = 28% utilization
```

---

## 7. Future Considerations

Items intentionally deferred:

1. **Alternative TLS Libraries** — requires hardware with
   allocator support or C toolchain integration
2. **Additional Board Profiles** — future profiles added as
   `boards/<profile-name>/`
3. **Multi-MCU Support** — ATSAM, STM32F7/H7, nRF52, ESP32
4. **Full HIL Test Automation** — depends on project maturity
5. **FIPS 140-3 Certification** — production requirement, not
   development priority
6. **Wireless Connectivity** — WiFi/BLE modules (current
   primary design is Ethernet)
7. **BBQueue Zero-Copy Pipeline** — lock-free SPSC inter-task
   communication; research complete (see session archives),
   implementation deferred to dedicated feature branch
8. **Buf Schema Registry (BSR)** — hosted schema registry for
   cross-team sharing; simple S3 + git tags suffice initially
9. **On-Device CloudEvents Envelope** — currently cloud-side
   only via Rules Engine; on-device encoding adds ~48 B per
   message but enables offline CloudEvents compliance.
   Implement when a second consumer demands the envelope.
10. **Azure IoT Hub / GCP IoT** — secondary cloud support via
    trait-based abstraction; implemented after AWS is proven
11. **CAN Bus Gateway** — deferred to backlog; different
    product vertical from sensor telemetry

---

## References

- IEEE 16326:2019 — Systems and software engineering — Life
  cycle processes — Project management
- [System Requirements Specification](./system_requirements.md)
- [Risk Register](./risk_register.md)
- [Architecture Decisions](./architecture/decisions.md)
- [AsyncAPI Specification v3.1.0](https://www.asyncapi.com/docs/reference/specification/v3.1.0)
- [CloudEvents Protobuf Format](https://github.com/cloudevents/spec/blob/main/cloudevents/formats/protobuf-format.md)

---

*This document is maintained as a living reference.  Update
as project status changes.*
