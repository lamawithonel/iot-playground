# ARS Toolhead Sensor -- Hardware

## Bill of Materials

| Part | Role |
|------|------|
| ST NUCLEO-N657X0-Q (STM32N657X0) | MCU board |
| Adafruit MAX9744 | 20 W class-D amplifier |
| Dayton Audio EX25VT2-4 | Audio exciter |
| SparkFun SPH8878LR5H-1 breakout (BOB-19389), modified | MEMS microphone |

### Part Notes

- **NUCLEO-N657X0-Q**: Cortex-M55 @ 800 MHz, ~4.2 MB contiguous
  SRAM, flashless with signed-FSBL external-NOR boot, Neural-ART
  NPU.
- **MAX9744**: analog line-level input; drives the exciter.
- **EX25VT2-4**: vented 25 mm exciter, two-hole mount, 20 W,
  4 ohm.
- **SPH8878LR5H-1 breakout**: analog output into the MCU ADC; see
  the modification below.

## Integration Notes

### Microphone Breakout Modification

Recorded as given, not independently verified:

- TI OPA345NA op-amp upgrade on the breakout.
- 30 kOhm resistor replacing C3 to accommodate the OPA345NA.

The resulting gain, bandwidth, and DC operating point of the
modified breakout are unknown; characterize them at bring-up
before fixing the ADC input range.

### Exciter Mounting (Open Mechanical Question)

How the EX25VT2-4 clamps to the H2C toolhead is unresolved.  Open
points: clamp geometry against the toolhead, mounting via the
two-hole pattern, preload for consistent structural coupling, and
thermal exposure near the hot end.  No mount design exists yet.

## Open Questions

- **Gain staging.**  Signal levels from the MCU audio output into
  the MAX9744 line input, amplifier gain setting, and drive level
  into the 20 W / 4 ohm exciter are all undetermined.  Mic-side
  gain after the breakout modification is likewise unknown.
- **Acoustic isolation.**  Separating the structure-borne
  toolhead response from printer ambient noise (mount coupling,
  mic placement, shielding) is unresolved.
- **Electrical specifics.**  Supply rails, wiring, and connector
  choices are not yet defined; nothing beyond the parts listed
  above is decided.
