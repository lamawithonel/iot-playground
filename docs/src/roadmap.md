# Project Roadmap
## Embedded Rust IoT Firmware

**Version:** 1.0  
**Last Updated:** 2026-01-12  
**Project Phase:** Phase 2 (Network Stack)

---

## 1. Project Scope

### 1.1 Objectives

Build a reference implementation for embedded Rust IoT firmware on STM32F405RG, demonstrating:
- RTIC 2.x real-time framework with Embassy HAL
- Secure MQTT connectivity via TLS 1.3
- Environmental sensor integration
- E-ink display status dashboard
- CAN bus gateway functionality
- Secure OTA firmware updates

### 1.2 Deliverables

- Firmware binary for STM32F405RG (Adafruit Feather)
- Documentation site (GitHub Pages via mdBook)
- Test infrastructure (GitHub Actions CI/CD)
- Docker-based local test environment

### 1.3 Out of Scope

- AWS IoT Core infrastructure setup
- Hardware PCB design
- Production manufacturing
- Multi-board support (initially STM32F405 only)

---

## 2. Current Status

### 2.1 Phase 2: Network Stack (In Progress)

**Completed:**
- [x] W5500 SPI driver and hardware initialization
- [x] DHCP and Layer 2 networking
- [x] Network stack abstraction (`network` module)
- [x] SNTP client with RTC synchronization
- [x] TLS 1.3 handshake (using `embedded-tls`)
- [x] Local MQTT broker test environment (Docker)

**In Progress:**
- [ ] MQTT client with AWS IoT Core
- [ ] Interrupt-driven packet reception (EXTI2)

**Blocked:**
- None currently

### 2.2 TLS Library Decision

**Status:** ✅ Resolved

**Decision:** Use `embedded-tls` (commit dccd96634679d52a7eac7a0ed216b9c24dbfb122)

**Rationale:**
- No allocator required (works in `no_std` without heap)
- TLS 1.3 support with modern cipher suites
- Compatible with Embassy async traits

**Known Limitations:**
- **No RSA support** - servers must use ECDSA certificates
- **No SHA-512/SHA-3** - only SHA-256/SHA-384 available
- **ECDSA only** - supports secp256r1 and secp384r1 curves
- **Single cipher suite** - TLS_AES_128_GCM_SHA256 (SHA-384 variant available)

**Alternatives Evaluated:**
| Library | Status | Reason Not Selected |
|---------|--------|---------------------|
| `rustls` | Deferred | Requires allocator; insufficient memory on STM32F405 |
| `wolfSSL` (via `wolfcrypt-rs`) | Deferred | C library dependency; complex build integration |
| `mbedTLS` | Deferred | C library; larger footprint than embedded-tls |

**Future Consideration:** Revisit TLS library selection when:
- Hardware with more memory is available (STM32F7/H7 with external SRAM)
- Production requirements demand FIPS 140-3 certification (wolfSSL)
- RSA certificate support becomes mandatory

---

## 3. Implementation Phases

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
- [ ] MQTT persistent connection with periodic test publishing (Phase 2.5)
- [ ] Event-driven MQTT message handling
- [ ] MQTT keep-alive handling (AWS IoT 1200s interval)
- [ ] Shared MQTT connection resource (RTIC Shared)
- [ ] WFI/Sleep mode between messages
- [ ] Interrupt-driven packet reception (EXTI2)
- [ ] Full AWS IoT Core integration

**Phase 2.5 Testing Goals (Current):**
- Implement periodic test data task (30s interval)
- Establish MQTT connection per publish cycle (temporary)
- Validate end-to-end TLS + MQTT + device ID flow
- Prepare infrastructure for persistent connection refactoring

**Phase 2.5 → Phase 3 Transition Requirements:**
- Refactor to maintain single persistent MQTT connection
- Move connection to RTIC Shared resource for cross-task access
- Implement proper keep-alive with AWS IoT recommendations (1200s)
- Add WFI/Sleep mode with interrupt-driven wake (requires EXTI2)
- Implement message queuing for reliability

### Phase 3: Sensor Integration ⏳ Not Started
- [ ] Verify sensor pin assignments with logic analyzer
- [ ] I2C abstraction
- [ ] I2C bus initialization
- [ ] SEN66 driver with CRC validation
- [ ] Periodic sensor readings (60s timer)
- [ ] Publish sensor data via MQTT

### Phase 4: Display ⏳ Not Started
- [ ] Verify E-ink breakout board pin assignments with logic analyzer
- [ ] SSD1681 SPI communication
- [ ] Test patterns and text rendering
- [ ] Status dashboard with sensor data
- [ ] Partial refresh optimization

### Phase 5: CAN Gateway ⏳ Not Started
- [ ] Verify CAN pin assignments with logic analyzer
- [ ] CAN bus configuration at 1 Mbps
- [ ] CAN → MQTT forwarding
- [ ] MQTT → CAN transmission

### Phase 6: Secure OTA ⏳ Not Started
- [ ] Bootloader integration (`embassy-boot-stm32`)
- [ ] Firmware signature verification
- [ ] MQTT-based firmware delivery
- [ ] Watchdog rollback protection

**Note:** Secure OTA may require hardware upgrade to STM32F7/H7 for sufficient flash/RAM.

---

## 4. Test Strategy

See [Testing Strategy](./development/testing.md) for full details.

### 4.1 Summary

| Test Type | Automation | Environment |
|-----------|------------|-------------|
| Formatting (`rustfmt`) | ✅ GitHub Actions | Public runners |
| Linting (`clippy`) | ✅ GitHub Actions | Public runners |
| Build verification | ✅ GitHub Actions | Public runners |
| Host-side unit tests | ✅ GitHub Actions | Public runners |
| Docker image builds | ✅ GitHub Actions | Public runners |
| On-device integration | 🔄 Future | Self-hosted runner |
| Hardware validation | ❌ Manual | Local workstation |

### 4.2 Self-Hosted Runner (Future)

When on-device testing automation becomes valuable (likely Phase 3+), a self-hosted GitHub Actions runner will be configured:
- Single Docker container on local workstation
- USB passthrough for J-Link/probe-rs access
- Network access to local Mosquitto container
- No Kubernetes required

---

## 5. Milestones

| Milestone | Target Date | Status |
|-----------|-------------|--------|
| Phase 1: Core Platform | - | ✅ Complete |
| Phase 2: Network Stack | TBD | 🔄 In Progress |
| Phase 3: Sensor Integration | TBD | ⏳ Not Started |
| Phase 4: Display | TBD | ⏳ Not Started |
| Phase 5: CAN Gateway | TBD | ⏳ Not Started |
| Phase 6: Secure OTA | TBD | ⏳ Not Started |
| Documentation Site (GitHub Pages) | TBD | ⏳ Not Started |

---

## 6. Future Considerations

Items intentionally deferred:

1. **Alternative TLS Libraries** - Requires hardware with allocator support or C toolchain integration
2. **Multi-MCU Support** - ATSAM, STM32F7/H7 variants
3. **Full HIL Test Automation** - Depends on project maturity and test burden
4. **FIPS 140-3 Certification** - Production requirement, not development priority
5. **Wireless Connectivity** - WiFi/BLE modules (current design is Ethernet-only)

---

## References

- IEEE 16326:2019 - Systems and software engineering — Life cycle processes — Project management
- [System Requirements Specification](./system_requirements.md)
- [Risk Register](./risk_register.md)
- [Architecture Decisions](./architecture/decisions.md)

---

*This document is maintained as a living reference. Update as project status changes.*
