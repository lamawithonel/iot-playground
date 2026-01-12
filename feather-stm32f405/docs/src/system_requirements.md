# System Requirements Specification
## Embedded Rust IoT Firmware

**Version:** 1.2  
**Date:** January 3, 2026  
**Status:** Active Development  
**Author:** Lucas Yamanishi

---

## 1. Introduction

### 1.1 Purpose

This document specifies requirements for embedded Rust firmware implementing real-time IoT capabilities on STM32F405RG. The system provides secure MQTT connectivity, environmental monitoring, local display, and OTA updates.

**Target Audience:** Firmware developers, hardware engineers, future maintainers, and AI assistants helping with implementation.

### 1.2 Scope

**System:** Embedded Rust IoT Firmware on an Adafruit Feather STM32F405 Express development board

**In Scope:** Firmware, device drivers, network protocols, security, OTA mechanism  
**Out of Scope:** Cloud infrastructure, AWS configuration, hardware design, manufacturing

### 1.3 References

- ISO/IEC/IEEE 29148:2018 - Requirements engineering
- STM32F405RG Reference Manual (RM0090 Rev 21)
- W5500 Datasheet v1.1.0, WIZnet
- SSD1681/IL0376F Datasheet, Solomon Systech / Good Display
- SEN66 Datasheet, Sensirion AG
- RTIC Book: https://rtic.rs/
- Embassy Framework: https://embassy.dev/

---

## 2. System Context

### 2.1 Operating Environment

**Hardware Platform:** Adafruit Feather STM32F405 Express  
**MCU:** STM32F405RG (ARM Cortex-M4F, 168 MHz, 1 MB Flash, 192 KB SRAM)  
**Debug:** Segger J-Link via SWD

**Peripherals:**
- W5500 Ethernet controller (SPI2)
- SSD1681 E-ink display 200×200 with SRAM (SPI1)
- SEN66 air quality sensor (I2C1)
- TJA1051 CAN transceiver (CAN1)
- microSD card (SDIO)

**Network:** Ethernet with DHCP, Internet access required for AWS IoT Core

### 2.2 Key Stakeholders

| Stakeholder | Interest |
|-------------|----------|
| **Developers** | Clean architecture, testability, documentation |
| **Device Operators** | Reliable operation, clear status, minimal maintenance |
| **IoT Platform Team** | Standard protocols, predictable message formats |

### 2.3 Design Philosophy

1. **Safety First:** Memory-safe Rust with minimal unsafe code
2. **Real-Time:** RTIC framework with formal verification (Stack Resource Policy)
3. **Security:** Defense-in-depth with TLS 1.3, authenticated updates
4. **Maintainability:** Idiomatic Rust, comprehensive docs, 80%+ test coverage
5. **Incremental:** Layer 2 → DHCP → TCP → TLS → MQTT progression
6. **Example-Driven:** Reference implementation for embedded Rust projects

### 2.4 Constraints

**Hardware:**
- Fixed pin assignments per PCB layout (cannot be changed in software)
- No hardware crypto accelerator (software only)
- Limited secure boot support on STM32F4 series

**Software:**
- Rust `no_std` environment (no heap, no OS)
- RTIC 2.x required for real-time guarantees
- Embassy HAL where compatible with RTIC

**Regulatory:**
- TLS 1.3 minimum
- FIPS 140-3 algorithms where possible

---

## 3. Hardware Interface Requirements

### 3.1 Pin Assignments (CRITICAL - DO NOT MODIFY)

```
LEFT SIDE (16-Pin Header)             BOARD          RIGHT SIDE (12-Pin Header)
==============================    Physical Layout    ====================================
                                       .....
Device         Func  Pin  Mark     .--|     |--.     Mark  Pin   Func    Device
-------------  ----- ---- ----     |  |USB-C|  |     ----  ----  -----  -----------------
                                   |  |     |  |
               Reset NRST RST   o--|  '-----'  |
               3.3V       3V3   o--|           |
               3.3V       3V3   o--|           |
               GND   GND  GND   o--|           |--o  VBAT  VBAT  Power
eInk display   BUSY  PA4  (A0)  o--|           |--o  EN    EN    Enable
eInk SPI       SCK   PA5  (A1)  o--|           |--o  VBUS  VBUS  USB 5V
eInk SPI       MISO  PA6  A2    o--|           |--o  13    PC1   LED    On-board red LED
eInk SPI       MOSI  PA7  A3    o--|           |--o  12    PC2   IRQ    WIZnet W5500
eInk display   ECS   PC4  A4    o--|           |--o  11    PC3   Reset  WIZnet W5500
eInk display   D/C   PC5  A5    o--|           |--o  10    PB9   TX     CAN1
WIZnet W5500   SCK   PB13 SCK   o--|           |--o  9     PB8   RX     CAN1
WIZnet W5500   MISO  PB14 MO    o--|           |--o  6     PC6   CS     WIZnet W5500
WIZnet W5500   MOSI  PB15 MI    o--|           |--o  5     PC7   Reset  eInk display
eInk SRAM      SCS   PB11 RX    o--|           |--o  SCL   PB6   SCL    I2C (SEN66, etc.)
eInk microSD   SDCS  PB10 TX    o--|           |--o  SDA   PB7   SDA    I2C (SEN66, etc.)
                                   '-----------'
```


### 3.2 Peripheral Pin Map

| Peripheral | Pins | Interface | Notes |
|------------|------|-----------|-------|
| **W5500 Ethernet** | PC2(IRQ), PC3(RST), PC6(CS), PB13(SCK), PB14(MISO), PB15(MOSI) | SPI2, Mode 0/3, 21 MHz max | IRQ via EXTI2 (priority 1) |
| **SSD1681 E-ink** | PA4(BUSY), PA5(SCK), PA6(MISO), PA7(MOSI), PC4(ECS), PC5(D/C), PC7(RST) | SPI1, Mode 0, 10 MHz write | BUSY via EXTI4 (priority 2) |
| **23LC1024 SRAM** | PB11(SRCS) + shared SPI1 | SPI1, Mode 0, 20 MHz | Framebuffer storage |
| **E-ink microSD Card** | PB10(SDCS) | SPI1 | E-ink bitmap storage (TBD if needed/useful) |
| **On-board microSD Card** | PC8-PC12, PD2 | SDIO | Firmware storage, certificates, persistent message queue |
| **SEN66 Sensor** | PB6(SCL), PB7(SDA) | I2C1, 400 kHz, addr 0x6B | 4.7kΩ pull-ups |
| **CAN Transceiver** | PB8(RX), PB9(TX) | CAN1, 1 Mbps | TJA1051 |
| **LED** | PC1 | GPIO output | Active-high |
| **NeoPixel LED** | PC0 | GPIO | NeoPixel |
| **Debug** | PA13(SWDIO), PA14(SWCLK) | SWD | J-Link RTT |
| **12.000 MHz Crystal** | PH0(OSC_IN), PH1(OSC_IN) | 3.3V | External oscillator |
| **32.768 kHz Crystal** | PC14(OSC32_IN), PC15(OSC32_OUT) | 3.3V | RTC external crystal | 
| **25Q16 (2 MiB) Flash Chip** | PV3(SCK), PB4(MOSI), PB5(MISO), PA15(CS) | SPI1 | Flash

### 3.3 EXTI Configuration

- **EXTI2 (PC2, W5500 IRQ):** Priority 1 (highest), dedicated vector, ~200ns latency, active-low with pull-up
- **EXTI4 (PA4, E-ink BUSY):** Priority 2 (medium), dedicated vector, ~200ns latency, active-high

**Rationale:** Network interrupts require highest priority for minimal packet loss; display can tolerate brief delays.

### 3.4 Timer Configuration

TODO: Decide on timer requirements

### 3.5 Memory Usage Strategy

```txt
 Main SRAM (128KB) - DMA-accessible:
 ├─ Stack:                    16KB (at top, grows down)
 ├─ TLS session state:        40KB
 ├─ TCP/IP buffers:           20KB
 ├─ Application heap:         20KB
 ├─ W5500 DMA buffers:        12KB
 ├─ Sensor data buffers:       8KB
 ├─ Protobuf encoding:         8KB
 └─ Firmware update buffer:    4KB

 CCM RAM (64KB) - CPU-only, zero wait states:
 ├─ Critical variables:        <1KB
 │   └─ TIME_SYNCED flag
 └─ Reserved for future:       63KB+
     └─ Available for timing-critical data

 Note: TLS buffers (34KB: 18KB read + 16KB write) now in main SRAM.
 Stack in main RAM allows more flexibility and prevents
       linker conflicts between stack and .ccmram section.
```

---

## 4. Functional Requirements

### 4.1 Network Communication

**SR-NET-001:** System SHALL establish TLS 1.3 connection to AWS IoT Core on startup (MQTT v5.0)
**SR-NET-002:** System SHALL authenticate using X.509 client certificates stored in flash  
**SR-NET-003:** System SHALL maintain MQTT connection with 60s keep-alive and automatic reconnect  
**SR-NET-004:** System SHALL enter sleep mode between transmissions, waking on EXTI2 or timer  
**SR-NET-005:** System SHALL process network interrupts within 500 μs  
**SR-NET-006:** System SHALL synchronize time using SNTP (RFC 5905)  
**SR-NET-007:** System SHALL timestamp all MQTT messages with an `event_timestamp` when the event is first captured and a `send_timestamp` when the MQTT message is sent  
**SR-NET-008:** System SHALL retry failed MQTT QoS level 0 messages up to 5 times with exponential back-off, then log error and discard the message  
**SR-NET-009:** System SHALL retry failed MQTT QoS level 1 and 2 messages up to 5 times with exponential back-off, then log error and place the message in one of two DLQs on the microSD card: one for network failures and one for rejected messages  
**SR-NET-010:** System SHALL place all outgoing MQTT QoS level 1 and 2 messages in the network failure DLQ if the DLQ is not empty  
**SR-NET-011:** System SHALL retry the oldest message in the network failure DLQ whenever a new message is placed in the queue, and retry all subsequent messages in FIFO order when one message succeeds, stopping if there are any additional failures and waiting for the next new message  
**SR-NET-012:** System SHALL log an error and stop queuing new MQTT messages to the network failure DLQ when microSD card utilization exceeds 80%

TODO: Decide on failed message DLQ maximum size and what to do about its contents

### 4.2 Sensor Data

**SR-SENS-001:** System SHALL read SEN66 sensor every 60 seconds (±5s)  
**SR-SENS-002:** System SHALL retrieve PM1.0, PM2.5, PM4.0, PM10, CO2, VOC, NOx, temperature, humidity  
**SR-SENS-003:** System SHALL validate readings using CRC-8 checksum, rejecting invalid data  
**SR-SENS-004:** System SHALL publish sensor data as Protocol Buffers to `device/{id}/telemetry` with MQTT QoS level 1  
**SR-SENS-005:** System SHALL add a monotonically increasing event ID to all sensor output MQTT messages, which MAY reset with a device power cycle  
**SR-SENS-006:** System SHALL retry failed sensor reads up to 3 times, then log error and skip

### 4.3 Display

**SR-DISP-001:** System SHALL update the E-ink display after every sensor read grouping (initial read plus retries) showing PM2.5, CO2, VOC, temperature, humidity, network upload status, firmware version, and timestamp  
**SR-DISP-002:** System SHOULD use partial refresh where supported to minimize display wear  
**SR-DISP-003:** System SHALL monitor BUSY pin (PA4) and wait for LOW before sending commands  
**SR-DISP-004:** System SHALL display error messages for critical faults (network offline, sensor failure)  
**SR-DISP-005:** System SHALL update the display after every network state change (online/offline status, CIDR IP address, FQDN)

### 4.4 CAN Gateway

**SR-CAN-001:** System SHALL receive CAN 2.0B messages at 1 Mbps with standard/extended IDs  
**SR-CAN-002:** System SHALL forward CAN messages to MQTT topic `device/{id}/can/{can_id}`  
**SR-CAN-003:** System SHALL accept MQTT on `device/{id}/can/tx` and transmit to CAN bus  
**SR-CAN-004:** System SHALL support configurable CAN ID filtering via MQTT

### 4.5 Firmware Updates

**SR-OTA-001:** System SHALL receive firmware via MQTT topic `device/{id}/ota` with chunked transfer  
**SR-OTA-002:** System SHALL verify firmware signature using ECDSA/RSA before installation  
**SR-OTA-003:** System SHALL store firmware images encrypted (AES-256-GCM) on microSD card  
**SR-OTA-004:** System SHALL implement an atomic update scheme  
**SR-OTA-005:** System SHALL use watchdog timer to rollback to the last known-good firmware after 3 failed boot attempts  
**SR-OTA-006:** System SHALL publish update status to `device/{id}/ota/status`

### 4.6 Error Handling

**SR-ERR-001:** System SHALL log errors using `defmt` with ERROR severity including component, code, context  
**SR-ERR-002:** System SHALL retry transient failures up to 5 times with exponential back-off  
**SR-ERR-003:** System SHALL enter safe mode after persistent failures, attempting recovery every 60s  
**SR-ERR-004:** System SHALL indicate errors via LED: slow blink (warning), fast blink (error), solid (critical)

---

## 5. Performance Requirements

**SR-PERF-001:** EXTI2 interrupt latency SHALL be <500 μs from assertion to ISR entry  
**SR-PERF-002:** Sensor read SHALL complete within 2 seconds  
**SR-PERF-003:** Display update SHALL complete within 10 seconds for full refresh  
**SR-PERF-004:** MQTT publish SHALL occur within 100 ms from data ready to transmission  
**SR-PERF-005:** Average power consumption SHALL be <100 mA (excluding display updates)  
**SR-PERF-006:** Incoming MQTT messages SHALL be processed within 50 ms  
**SR-PERF-007:** Flash utilization SHALL be ≤90% (900 KB), SRAM ≤80% (154 KB)

---

## 6. Quality Requirements

### 6.1 Reliability

**SR-REL-001:** System SHALL achieve 99% uptime over 30-day period  
**SR-REL-002:** System SHALL recover from single faults (network drop, sensor timeout) within 5 minutes  
**SR-REL-003:** System SHALL detect corrupted data with >99.99% probability

### 6.2 Security

**SR-SEC-001:** All network communication SHALL use TLS 1.3 with approved cipher suites  
**SR-SEC-002:** Cryptographic operations SHOULD use FIPS 140-3 algorithms where available  
**SR-SEC-003:** Random numbers SHALL be generated from hardware RNG (STM32 RNG peripheral)  
**SR-SEC-004:** Private keys SHALL be stored in read-protected flash, never transmitted  
**SR-SEC-005:** Production builds SHALL disable debug interfaces (SWD, RTT)

### 6.3 Maintainability

**SR-MAINT-001:** Code SHALL follow Rust style guide, formatted with `rustfmt`, pass `clippy` with zero warnings  
**SR-MAINT-002:** Public functions/modules SHALL have documentation comments  
**SR-MAINT-003:** Code SHALL be organized in layers: drivers, application logic, protocols  
**SR-MAINT-004:** Code coverage SHALL be ≥80%  
**SR-MAINT-005:** Rust file MUST use the `#![deny(warnings)]` macro

### 6.4 Design Constraints

**SR-CONST-001:** System SHALL be implemented in Rust `no_std` environment  
**SR-CONST-002:** System SHALL use RTIC 2.x for task scheduling and formal verification  
**SR-CONST-003:** System SHOULD use Embassy HAL where compatible with RTIC  
**SR-CONST-004:** System MUST NOT use the Embassy executor (`embassy-executor` crate)  
**SR-CONST-005:** Unsafe code SHALL be <5% of codebase, isolated and documented, and all Rust files SHOULD use the `#![deny(unsafe_code)]` macro  
**SR-CONST-006:** Pin assignments SHALL match hardware layout exactly (see Section 3)

---

## 7. Implementation Phases

### Phase 1: Core Platform (MVP)
- [x] GPIO and LED control
- [x] SWD debugging with RTT logging

### Phase 2: Network Stack
- [x] Verify network pin assignments with logic analyzer
- [x] W5500 SPI driver
- [x] DHCP and Layer 2 networking
- [x] Newtwork stack abstraction
- [x] SNTP client (`sntpc`)
- [x] TLS 1.3 handshake
- [ ] MQTT client with AWS IoT Core
- [ ] Interrupt-driven packet reception

### Phase 3: Sensor Integration
- [ ] Verify sensor pin assignments with logic analyzer
- [ ] I2C abstraction
- [ ] I2C bus initialization
- [ ] SEN66 driver with CRC validation
- [ ] Periodic sensor readings (60s timer)
- [ ] Publish sensor data via MQTT

### Phase 4: Display
- [ ] Verify E-ink breakout board pin assignments with logic analyzer
- [ ] SSD1681 SPI communication
- [ ] Test patterns and text rendering
- [ ] Status dashboard with sensor data
- [ ] Partial refresh optimization

### Phase 5: CAN Gateway
- [ ] Verify CAN pin assignments with logic analyzer
- [ ] CAN bus configuration at 1 Mbps
- [ ] CAN → MQTT forwarding
- [ ] MQTT → CAN transmission

### Phase 6: Secure OTA
- [ ] Bootloader integration (`embassy-boot-stm32`)
- [ ] Firmware signature verification
- [ ] MQTT-based firmware delivery
- [ ] Watchdog rollback protection

NOTE: Secure OTA updates may require a more capable MCU

---

## 8. Verification

### Test Methods
- **Unit Tests:** All non-hardware functions, ≥80% coverage, `embedded-test` framework
- **Integration Tests:** `defmt-test` on real hardware via GitHub Actions self-hosted runners
- **Hardware Tests:** Logic analyzer for timing, oscilloscope for latency, packet capture for protocols

### Acceptance Criteria
- All Critical/High priority requirements verified
- ≥90% of Medium priority requirements verified
- Zero Critical/High severity defects open
- 30-day reliability test passes
- Security audit finds no vulnerabilities ≥7.0 CVSS

---

## 10. Reference Information

**Development Tools:**
- Rust 1.75+ with `thumbv7em-none-eabihf` target
- `cargo-embed` or `probe-rs` for flashing/debugging
- J-Link tools for RTT logging
- Logic analyzer and oscilloscope for hardware validation

**Key Dependencies:**
- `rtic` 2.x - Real-time framework
- `embassy-stm32` - HAL and drivers
- `embedded-tls` - TLS stack (chosen library; supports TLS 1.3 without allocator)
- `rumqttc` or `rust-mqtt` - MQTT client
- `prost` - Protocol Buffers
- `defmt` + `defmt-rtt` - Logging

**Testing:**
- `embedded-test` - Unit tests
- `embedded-hal-mock` - HAL mocking
- `defmt-test` - Integration tests on hardware
- `cargo-tarpaulin` - Coverage analysis

---

*This document is maintained as a living reference. Update as requirements evolve or decisions change.*
