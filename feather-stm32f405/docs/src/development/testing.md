# Testing Strategy
## Embedded Rust IoT Firmware

**Last Updated:** 2026-01-12

---

## 1. Overview

This document describes the testing strategy for the embedded Rust IoT firmware project, covering automated CI/CD testing and manual hardware validation.

### 1.1 Testing Challenges

Embedded firmware presents unique testing challenges:
- **Hardware dependency**: Many tests require physical MCU and peripherals
- **Real-time behavior**: Timing-sensitive code difficult to test in simulation
- **Limited emulation**: QEMU has minimal STM32F4 peripheral support
- **Debug interfaces**: On-device tests require SWD/JTAG access

### 1.2 Testing Philosophy

1. **Maximize host-testable code**: Pure logic in separate modules
2. **Automate what's practical**: CI for builds, linting, host tests
3. **Document manual procedures**: Hardware validation checklists
4. **Defer complexity**: Self-hosted runners when test burden justifies

---

## 2. Test Categories

### 2.1 Static Analysis (Automated)

**Environment:** GitHub Actions public runners

| Check | Tool | Configuration |
|-------|------|---------------|
| Formatting | `rustfmt` | Default settings |
| Linting | `clippy` | `-D warnings` (deny all warnings) |
| Build (debug) | `cargo build` | `thumbv7em-none-eabihf` target |
| Build (release) | `cargo build --release` | Optimized for size (`opt-level = "s"`) |

**Status:** ✅ Implemented in `.github/workflows/ci.yaml`

### 2.2 Unit Tests (Automated)

**Environment:** GitHub Actions public runners (host architecture)

**Testable Modules:**
- `time/calendar.rs` - Date/time conversions (Howard Hinnant algorithms)
- `time/rtc.rs` - Timestamp operations (without RTC hardware)
- `ccmram.rs` - Wall-clock calibration math
- Future: Protocol encoding/decoding, message parsing

**Running Tests:**
```bash
# Host-side unit tests (no embedded target)
cargo test --manifest-path feather-stm32f405/Cargo.toml --lib
```

**Status:** 🔄 Partially implemented - calendar tests exist, CI integration pending

### 2.3 Docker Image Validation (Automated)

**Environment:** GitHub Actions public runners

**Purpose:** Catch regressions in test infrastructure (Mosquitto MQTT broker)

**Tests:**
- Docker image builds successfully
- Mosquitto starts and listens on expected ports
- TLS certificates generated correctly

**Status:** ⏳ To be implemented

### 2.4 On-Device Integration Tests (Future - Self-Hosted)

**Environment:** Self-hosted GitHub Actions runner

**Prerequisites:**
- Linux workstation with Docker
- J-Link or compatible SWD debugger
- STM32F405 Feather connected via USB
- Local network access (for Mosquitto container)

**Architecture:**
```
┌────────────────────────────────────────────────────┐
│  Self-Hosted Runner (Docker container)             │
│  ├─ probe-rs / cargo-embed                         │
│  ├─ USB passthrough to J-Link                      │
│  └─ Network access to Mosquitto container          │
└──────────────────────┬─────────────────────────────┘
                       │ USB
                       ▼
              ┌─────────────────┐
              │ STM32F405       │
              │ Feather Board   │
              └─────────────────┘
```

**Test Types:**
- Flash and boot verification
- RTT log capture and assertion
- TLS handshake with local Mosquitto
- SNTP synchronization
- Sensor read (when implemented)

**Status:** ⏳ Planned for Phase 3+

### 2.5 Hardware Validation (Manual)

**Environment:** Local workstation with test equipment

**Equipment Required:**
- Logic analyzer (SPI/I2C protocol decode)
- Oscilloscope (timing, signal integrity)
- Multimeter (power consumption)
- Network packet capture (Wireshark)

**Validation Procedures:**
| Test | Equipment | Acceptance Criteria |
|------|-----------|---------------------|
| SPI timing (W5500) | Logic analyzer | Clock ≤21 MHz, Mode 0/3 |
| I2C timing (SEN66) | Logic analyzer | 400 kHz, proper ACK/NAK |
| EXTI latency | Oscilloscope | <500 μs interrupt response |
| Power consumption | Multimeter | <100 mA average (spec SR-PERF-005) |
| TLS handshake | Wireshark | TLS 1.3, correct cipher suite |

**Status:** 🔄 Ongoing as features are implemented

---

## 3. CI/CD Pipeline

### 3.1 Current Pipeline (Public Runners)

```yaml
# .github/workflows/ci.yaml
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - Check formatting (rustfmt)
      - Run clippy
      - Build debug
      - Build release
      # Future additions:
      - Run host unit tests
      - Build Docker image
```

### 3.2 Future Pipeline (With Self-Hosted Runner)

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    # ... existing checks ...

  docker:
    runs-on: ubuntu-latest
    steps:
      - Build Mosquitto Docker image
      - Validate image starts correctly

  on-device:
    runs-on: self-hosted
    needs: [check]
    steps:
      - Flash firmware
      - Capture RTT logs
      - Run integration tests
      - Report results
```

---

## 4. Self-Hosted Runner Setup

### 4.1 Requirements

- Linux host (Ubuntu 22.04+ recommended)
- Docker installed
- USB access for J-Link debugger
- Network connectivity

### 4.2 Installation (Docker Method)

```bash
# Create runner container
docker run -d \
  --name github-runner \
  --restart unless-stopped \
  -e RUNNER_REPOSITORY_URL=https://github.com/lamawithonel/iot-playground \
  -e RUNNER_TOKEN=<token_from_github> \
  -v /dev/bus/usb:/dev/bus/usb \
  --privileged \
  myoung34/github-runner:latest

# Verify runner registration
docker logs github-runner
```

### 4.3 USB Device Access

```bash
# Add udev rules for J-Link (create /etc/udev/rules.d/99-jlink.rules)
SUBSYSTEM=="usb", ATTR{idVendor}=="1366", MODE="0666"

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 4.4 Network Configuration

The runner container needs access to:
- Internet (for GitHub API)
- Local Mosquitto container (e.g., via Docker bridge network)

```bash
# Create shared network
docker network create iot-test-net

# Run Mosquitto on shared network
docker run -d \
  --name mosquitto-test \
  --network iot-test-net \
  -p 8883:8883 \
  -v $(pwd)/feather-stm32f405/docker/mosquitto:/mosquitto \
  eclipse-mosquitto:latest

# Connect runner to shared network
docker network connect iot-test-net github-runner
```

---

## 5. Test Coverage Goals

| Component | Target Coverage | Current Status |
|-----------|----------------|----------------|
| Time/calendar utilities | 100% | ✅ Achieved |
| Network stack abstraction | 80% | ⏳ Not started |
| TLS integration | 60% (integration tests) | ⏳ Not started |
| Sensor drivers | 70% | ⏳ Not started |
| Display drivers | 60% | ⏳ Not started |
| CAN gateway | 80% | ⏳ Not started |
| OTA bootloader | 90% | ⏳ Not started |

---

## 6. Manual Testing Procedures

### 6.1 Network Stack Validation

**Objective:** Verify TLS handshake with local Mosquitto broker

**Procedure:**
1. Start Mosquitto with TLS enabled: `cd feather-stm32f405/docker && docker-compose up`
2. Flash firmware: `cargo embed --release`
3. Monitor RTT logs: Look for "TLS handshake successful"
4. Capture packets: `sudo tcpdump -i any -w capture.pcap port 8883`
5. Analyze with Wireshark: Verify TLS 1.3, cipher suite TLS_AES_128_GCM_SHA256

**Pass Criteria:**
- Handshake completes in <2 seconds
- No certificate errors
- Connection remains stable for 5+ minutes

### 6.2 SPI Timing Verification (W5500)

**Objective:** Validate SPI communication meets W5500 timing requirements

**Equipment:** Logic analyzer with SPI decoder

**Procedure:**
1. Connect logic analyzer to SPI2 pins: PB13 (SCK), PB14 (MISO), PB15 (MOSI), PC6 (CS)
2. Configure analyzer: SPI Mode 0, MSB first
3. Flash firmware and trigger network activity
4. Capture and analyze: Clock rate ≤21 MHz, proper CS toggling

**Pass Criteria:**
- Clock frequency within specification
- Setup/hold times met
- No glitches or protocol errors

### 6.3 Power Consumption Measurement

**Objective:** Verify average power consumption <100 mA

**Equipment:** Multimeter with DC current measurement

**Procedure:**
1. Insert multimeter in series with 3.3V supply
2. Run firmware in normal operation (periodic sensor reads + MQTT)
3. Measure current over 5-minute window
4. Calculate average (exclude display refresh spikes)

**Pass Criteria:**
- Average current <100 mA
- Peak current (with display) <200 mA

---

## 7. Known Limitations

### 7.1 QEMU Emulation

QEMU's STM32F4 support is limited:
- Basic CPU and timers work
- No W5500 emulation (network testing impossible)
- No I2C peripheral emulation (sensor testing impossible)
- SPI only partially implemented

**Conclusion:** QEMU not suitable for meaningful integration testing.

### 7.2 CI Hardware Tests

On-device testing in CI requires:
- Self-hosted runner (setup complexity)
- Physical hardware (single point of failure)
- USB device passthrough (limited to local machine)

**Decision:** Defer until test burden justifies setup effort (Phase 3+).

---

## 8. Future Enhancements

### 8.1 HIL Test Framework

When the project matures, consider:
- Dedicated test fixture with multiple boards
- Automated flashing and test execution
- Result aggregation and reporting
- Integration with GitHub status checks

### 8.2 Fuzz Testing

For protocol parsing and encoding:
- Use `cargo-fuzz` for protobuf decoders
- Test TLS state machine edge cases
- CAN message parser robustness

### 8.3 Performance Benchmarks

Track key metrics over time:
- Flash/RAM utilization
- Interrupt latency
- MQTT message throughput
- Power consumption trends

---

## References

- [System Requirements Specification](../system_requirements.md)
- [Project Roadmap](../roadmap.md)
- [CI Workflow](https://github.com/lamawithonel/iot-playground/blob/main/.github/workflows/ci.yaml)

---

*This document evolves with the project. Update as testing practices mature.*
