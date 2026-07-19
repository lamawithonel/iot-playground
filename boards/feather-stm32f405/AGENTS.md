# feather-stm32f405/-- Air-Quality Sensor Node

Adafruit Feather STM32F405 (Cortex-M4F, 1 MB flash, 192 KB SRAM)
with W5500 Ethernet and SEN66 environmental sensor.  RTIC 2.x app
publishing telemetry over MQTT v5 + TLS 1.3.

## Module Map

| Module | Purpose |
|--------|---------|
| `main.rs` | RTIC `#[app]`: init, heartbeat, sensor_task, network_task, idle (WFI); injects board couplings (client ID, CCM TLS buffers, RTC clock, telemetry hook) into `iot-net` clients |
| `sensor/` | SEN66 I2C driver glue (conditioning logic lives in `core/`) |
| `time/` | RTC glue over `iot_core::time` calendar math |
| `ccmram.rs` | **Only allowed-unsafe file**: CCM RAM placement (TLS buffers) |
| `eth.rs`, `device_id.rs`, `counting_exti.rs`, `config.rs` | Hardware bring-up, STM32 UID, EXTI telemetry, build-time config |

## Local Rules

- `ccmram.rs` is the sole allowlisted unsafe file-- see
  [`rust_style.md`](../../.agents/rules/rust_style.md) before
  touching anything `unsafe`.
- Pin assignments are fixed by the PCB; treat the pin map in
  [`system_requirements.md`](../../docs/src/system_requirements.md)
  section 3 as read-only.
- On-device tests: `tests/bringup.rs` via `embedded-test`
  (`mise run test:device`); smoke and integration suites via
  `mise run test:smoke` / `test:integration`.
