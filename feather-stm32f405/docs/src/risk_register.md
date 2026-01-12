# Risk Register
## Embedded Rust IoT Firmware

**Last Updated:** 2026-01-12

---

## Active Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Flash size constraints with TLS stack | High | Low | Using `embedded-tls` (no allocator, ~40KB TLS state) | ✅ Mitigated |
| R2 | Embassy-RTIC compatibility gaps | Medium | Low | PAC fallback available for unsupported peripherals | 🔄 Monitoring |
| R3 | Limited secure boot on STM32F4 | Medium | High | Plan hardware upgrade path to F7/H7 for production | 📋 Accepted |
| R4 | `embedded-tls` lacks RSA support | Low | N/A | Use ECDSA certificates; document server requirements | ✅ Mitigated |
| R5 | Self-hosted runner availability | Low | Medium | Manual testing fallback; runner on primary workstation | 📋 Accepted |

---

## Risk Details

### R1: Flash Size Constraints with TLS Stack

**Description:** TLS libraries (especially `rustls`) require significant flash and RAM, potentially exceeding STM32F405 resources.

**Impact:** High - Cannot establish secure connections if TLS doesn't fit.

**Mitigation:** 
- Selected `embedded-tls` which requires no allocator
- TLS buffers (34KB) fit in main SRAM
- Flash usage currently well under 900KB limit

**Status:** ✅ Mitigated - TLS 1.3 handshake working

---

### R2: Embassy-RTIC Compatibility Gaps

**Description:** Some Embassy HAL drivers may conflict with RTIC's interrupt-driven model or require Embassy executor features.

**Impact:** Medium - May need to implement custom drivers using PAC.

**Mitigation:**
- Use Embassy HAL where compatible
- PAC (Peripheral Access Crate) available for direct register access
- RTIC-first architecture documented in design constraints

**Status:** 🔄 Monitoring - No issues encountered yet

---

### R3: Limited Secure Boot on STM32F4

**Description:** STM32F4 series has limited hardware support for secure boot compared to F7/H7.

**Impact:** Medium - Production deployments may require stronger boot security.

**Mitigation:**
- Current development uses F405 for cost/availability
- Plan upgrade path to STM32F7/H7 for production
- `embassy-boot-stm32` provides software-based secure boot

**Status:** 📋 Accepted - Will address in production hardware selection

---

### R4: `embedded-tls` Lacks RSA Support

**Description:** The `embedded-tls` library only supports ECDSA signature algorithms; RSA certificates cause handshake failures.

**Impact:** Low - Requires server-side certificate configuration.

**Mitigation:**
- Document requirement for ECDSA certificates (secp384r1 recommended)
- Local Mosquitto test broker configured with ECDSA
- AWS IoT Core supports ECDSA certificates

**Status:** ✅ Mitigated - Server requirements documented

---

### R5: Self-Hosted Runner Availability

**Description:** On-device testing requires a self-hosted GitHub Actions runner on local workstation, which may have availability issues.

**Impact:** Low - Affects CI automation, not development capability.

**Mitigation:**
- Manual testing always available as fallback
- Runner is simple Docker container (no Kubernetes)
- Public runners handle all non-hardware tests

**Status:** 📋 Accepted - Will implement when test burden justifies

---

## Closed Risks

| ID | Risk | Resolution | Date |
|----|------|------------|------|
| - | - | - | - |

---

## Risk Status Legend

| Status | Meaning |
|--------|---------|
| ✅ Mitigated | Risk addressed; no longer a concern |
| 🔄 Monitoring | Risk exists; actively watching for issues |
| 📋 Accepted | Risk acknowledged; no action planned |
| ⚠️ Active | Risk materializing; requires action |
| ❌ Closed | Risk no longer applicable |

---

*Review and update risks at each phase completion.*
