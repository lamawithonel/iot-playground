# Board Profiles

One documentation page per board profile.  This index is the single
source of truth for the board roster; each board's page owns the
board-specific detail (flashing, debugging, configuration).

## Supported Boards

- [**Adafruit Feather STM32F405**](./feather-stm32f405.md) (Tier 2:
  Connected device with TLS/MQTT)
- BBC micro:bit v2 (planned)
- STM32F3 Discovery (planned)
- **ST NUCLEO-N657X0-Q** (ARS toolhead sensor project;
  scaffold-only, workspace-excluded)

## Board Profiles vs. Boards

A **board profile** is a specific configuration combining:

- A board type (e.g., Feather STM32F405)
- Peripheral components (e.g., Ethernet chip, sensors)
- Application purpose (e.g., sensor gateway, PTP server)

Examples of board profiles in `boards/`:

- `feather-eth-sensor/` - Feather STM32F405 + Ethernet + SEN66 sensor
  + CAN gateway
- `feather-ptp-server/` - Feather STM32F405 + Ethernet + GPS clock
  (IEEE 1588 PTP)
- `feather-m4-can/` - Feather M4 CAN Express + sensors (CAN-only
  device)

Each profile shares common code (like network stack) but has unique
configuration and glue code.
