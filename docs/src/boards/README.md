# Board Profiles

One documentation page per board profile.  This index is the single
source of truth for the board roster; each board's page owns the
board-specific detail (flashing, debugging, configuration).

## Supported Boards

- [**Adafruit Feather STM32F405**](./feather-stm32f405.md)-- flagship board,
  active workspace member.  Tier 2 connected device, hardware-validated:
  DHCP, SNTP, TLS 1.3, MQTT v5 QoS-1, and a SEN66 sensor.
- **ST NUCLEO-H753ZI**-- active workspace member.  Minimal RTIC heartbeat
  app today; planned home for an ARS DAC->ADC loopback rig and Ethernet
  bring-up.
- **ST NUCLEO-N657X0-Q**-- workspace-excluded scaffold for the ARS
  toolhead sensor project.  Compile-spike only, awaiting hardware.

## Board Profiles vs. Boards

A **board profile** is a specific configuration combining:

- A board type (e.g., Feather STM32F405)
- Peripheral components (e.g., Ethernet chip, sensors)
- Application purpose (e.g., sensor gateway, bring-up rig)

Board profiles in `boards/`:

- `feather-stm32f405/`-- Feather STM32F405 + W5500 Ethernet + SEN66 sensor
  (Tier 2 connected device; active, flagship board)
- `nucleo-h753zi/`-- ST NUCLEO-H753ZI bring-up rig (active workspace
  member; planned home for an ARS DAC->ADC loopback rig and Ethernet
  bring-up)
- `nucleo-n657x0/`-- ST NUCLEO-N657X0-Q scaffold for the ARS toolhead
  sensor project (workspace-excluded, awaiting hardware)

CAN bus and PTP (IEEE 1588) support are planned features; no board profile
implements either yet.

Each profile shares common code (like network stack) but has unique
configuration and glue code.
