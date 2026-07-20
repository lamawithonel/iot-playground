# Risk Register
## Embedded Rust IoT Firmware

**Last Updated:** 2026-07-18

---

## Active Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Flash size constraints with TLS stack | High | Low | Using `embedded-tls` (no allocator; 34KB static buffers in CCM RAM) | ✅ Mitigated |
| R2 | Embassy-RTIC compatibility gaps | Medium | Low | PAC fallback available for unsupported peripherals | 🔄 Monitoring |
| R3 | Limited secure boot on STM32F4 | Medium | High | Plan hardware upgrade path to F7/H7 for production | 📋 Accepted |
| R4 | `embedded-tls` lacks RSA support | Low | N/A | Use ECDSA certificates; document server requirements | ✅ Mitigated |
| R5 | Self-hosted runner availability | Low | Medium | Manual testing fallback; runner on primary workstation | 📋 Accepted |
| R6 | micropb MSRV 1.88.0 toolchain requirement | Low | Low | Workspace already tracks recent stable Rust | 🔄 Monitoring |
| R7 | CloudEvents Protobuf spec compliance | Low | Medium | Using `binary_data`-- technically non-compliant for PB payloads | 📋 Accepted |
| R8 | `embedded-tls` `NoVerify` in production | High | Medium | Custom `CertVerifier` required before staging deployment | ⚠️ Active |
| R9 | AWS IoT double-decode limit | Medium | Low | Rules Engine allows max 2 decode() calls; sufficient for CE + payload | 🔄 Monitoring |
| R10 | STM32N6 Rust ecosystem maturity (ARS project) | Medium | Medium | Project stays scaffold-only until a bring-up spike validates boot, flash, and HAL | 🔄 Monitoring |

---

## Risk Details

### R1: Flash Size Constraints with TLS Stack

**Description:** TLS libraries (especially `rustls`) require
significant flash and RAM, potentially exceeding STM32F405 resources.

**Impact:** High-- Cannot establish secure connections if TLS doesn't fit.

**Mitigation:** 
- Selected `embedded-tls` which requires no allocator
- TLS buffers (34KB) placed in CCM RAM, freeing main SRAM for the
  handshake stack (see `system_requirements.md` section 3.5)
- Flash usage currently well under 900KB limit

**Status:** ✅ Mitigated-- TLS 1.3 handshake working

---

### R2: Embassy-RTIC Compatibility Gaps

**Description:** Some Embassy HAL drivers may conflict with RTIC's
interrupt-driven model or require Embassy executor features.

**Impact:** Medium-- May need to implement custom drivers using PAC.

**Mitigation:**
- Use Embassy HAL where compatible
- PAC (Peripheral Access Crate) available for direct register access
- RTIC-first architecture documented in design constraints

**Status:** 🔄 Monitoring-- No issues encountered yet

---

### R3: Limited Secure Boot on STM32F4

**Description:** STM32F4 series has limited hardware support for
secure boot compared to F7/H7.

**Impact:** Medium-- Production deployments may require stronger boot security.

**Mitigation:**
- Current development uses F405 for cost/availability
- Plan upgrade path to STM32F7/H7 for production
- `embassy-boot-stm32` provides software-based secure boot

**Status:** 📋 Accepted-- Will address in production hardware selection

---

### R4: `embedded-tls` Lacks RSA Support

**Description:** The `embedded-tls` library only supports ECDSA
signature algorithms; RSA certificates cause handshake failures.

**Impact:** Low-- Requires server-side certificate configuration.

**Mitigation:**
- Document requirement for ECDSA certificates (secp384r1 recommended)
- Local Mosquitto test broker configured with ECDSA
- AWS IoT Core supports ECDSA certificates

**Status:** ✅ Mitigated-- Server requirements documented

---

### R5: Self-Hosted Runner Availability

**Description:** On-device testing requires a self-hosted
GitHub Actions runner on local workstation, which may have
availability issues.

**Impact:** Low-- Affects CI automation, not development
capability.

**Mitigation:**
- Manual testing always available as fallback
- Runner is simple Docker container (no Kubernetes)
- Public runners handle all non-hardware tests

**Status:** 📋 Accepted-- Will implement when test burden
justifies

---

### R6: micropb MSRV 1.88.0

**Description:** micropb 0.6 requires Rust 1.88.0.  The
crate recently bumped its MSRV (was 1.80) and may do so
again as it adopts new language features.

**Impact:** Low-- Workspace already tracks recent stable
Rust via mise.

**Mitigation:**
- Pin exact micropb version in Cargo.toml
- Monitor upstream releases for MSRV bumps
- mise manages Rust toolchain centrally

**Status:** 🔄 Monitoring-- Current toolchain satisfies
requirement

---

### R7: CloudEvents Protobuf Spec Non-Compliance

**Description:** ADR-007 chose `binary_data` over
`proto_data` for carrying Protobuf payloads inside
CloudEvents envelopes.  The CloudEvents Protobuf Event
Format §3.2 states that `proto_data` MUST be used for
Protobuf data.  Our approach is technically non-compliant.

**Impact:** Low-- No known conformance test suites or
enforcement mechanisms.  Cloud consumers route by `type`
field, not `type_url`.

**Mitigation:**
- Document the deviation in ADR-007
- Set `datacontenttype` to `application/protobuf` for
  clarity
- Cloud-side routing uses `type` attribute (unaffected)
- If spec finalizes with hard requirement, migration
  path exists (swap `binary_data` -> `proto_data` with
  manual `Any` packing)

**Status:** 📋 Accepted-- Defensible trade-off for
no_std/no_alloc constraints

---

### R8: `embedded-tls` NoVerify in Production

**Description:** The current TLS configuration uses
`embedded_tls::CertVerifier::None` (NoVerify), which skips
server certificate validation.  This is acceptable for
development but is a compliance blocker for staging and
production deployments.

**Impact:** High-- Certificates are not verified; a
man-in-the-middle attack could impersonate the MQTT broker.

**Mitigation:**
- Software-only verification using `webpki`-compatible
  roots (no hardware dependency)
- Schedule fix for Security Foundation phase

**Status:** ⚠️ Active-- Must resolve before any non-lab
deployment

---

### R9: AWS IoT Rules Engine Double-Decode Limit

**Description:** Extracting fields from CloudEvents
Protobuf payloads requires two `decode()` calls: one for
the CloudEvents envelope and one for the inner sensor
payload (base64-encoded in `binary_data`).  The AWS IoT
Rules Engine allows only 2 `decode()` invocations per SQL
expression.

**Impact:** Medium-- If future schemas require nested
Protobuf messages, a third `decode()` would fail.

**Mitigation:**
- Current schema design uses flat messages (no nesting)
- Complex processing offloaded to Lambda functions
- Monitor AWS for `decode()` limit changes

**Status:** 🔄 Monitoring-- Current design is within limits

---

### R10: STM32N6 Rust Ecosystem Maturity (ARS Project)

**Description:** The ARS toolhead-sensor project targets the
NUCLEO-N657X0-Q board.  The STM32N657 is a flashless MCU that
boots from external NOR flash via a signed First-Stage Boot
Loader (FSBL), and its Rust story is young: `embassy-stm32`
`stm32n657x0` support is recent and unproven here, the
`probe-rs` flash/debug flow for the flashless boot chain is
unverified, and Neural-ART NPU deployment tooling (ST Edge AI)
is C-centric with no established Rust FFI path.

**Impact:** Medium-- Blocks ARS bring-up, not the framework.

**Mitigation:**
- Project stays scaffold-only (workspace-excluded crate) until
  a hardware bring-up spike validates boot, flash, and HAL
- CNN work starts host-side and does not block on the NPU

**Status:** 🔄 Monitoring

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
