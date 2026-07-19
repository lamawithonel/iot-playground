# feather-stm32f405-- Air-Quality Sensor Node

**Status: active workspace member.**  DHCP, SNTP, TLS 1.3, and
MQTT v5 QoS-1 telemetry from a SEN66 sensor are all verified on
hardware.  This is the framework's flagship board.

## Hardware

Adafruit Feather STM32F405 (STM32F405RG, Cortex-M4F @ 168 MHz,
1 MB flash, 192 KB SRAM) with a W5500 Ethernet module and a
Sensirion SEN66 environmental sensor (PM, CO2, VOC, NOx,
temperature, humidity) over I2C.  Debug probe: Segger J-Link over
SWD.

## What It Does

An RTIC 2.x application that reads the SEN66 over I2C and
publishes telemetry over MQTT v5 (QoS-1) with TLS 1.3, using DHCP
for addressing and SNTP for time sync.  See `AGENTS.md` for the
module map.

## Building and Flashing

```sh
choom -n 1000 -- cargo build --release -p feather-stm32f405 \
  --target thumbv7em-none-eabihf
cargo embed -p feather-stm32f405 --release
```

## Testing

On-device bring-up tests use `embedded-test`
(`mise run test:device`); smoke and integration suites run via
`mise run test:smoke` and `mise run test:integration` (tiered
`-extended` / `-full` variants exist for both).  See
[`testing.md`](../../docs/src/development/testing.md) for detail.
