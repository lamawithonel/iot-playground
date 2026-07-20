# ARS Toolhead Sensor-- HIL Measurements

This page defines bench measurements for two rigs: the
[gate G3](./pinout.md#spike-gates) audio path on the active
acoustic resonance spectroscopy (ARS) toolhead sensor, and the
planned DAC/ADC loopback rig on the
[NUCLEO-H753ZI bench](../../boards/nucleo-h753zi.md).  Captures use
a Saleae Logic MSO 2x100 via the Logic 2 automation API
(`.agents/skills/saleae-logic/SKILL.md`); that skill covers how to
drive the analyzer; this page covers what to measure and why.

Status: provisional.  Nothing below has been probed against real
hardware-- every capture plan here is on paper only, pending Logic 2
software on the bench machine.

## Gate G3 Audio-Path Measurements

Grounded in [Pinout](./pinout.md) and
[Roadmap](../../roadmap.md) section 7 item 4, hardware-in-the-loop
(HIL) test automation via the Logic 2 gRPC automation API.

| Measurement | What it validates |
|---|---|
| PWM carrier residue: dual-channel analog capture of the TIM1_CH1 PWM carrier (~200 kHz carrier, ~10-bit duty ceiling) before the RC low-pass, alongside the RC-filtered line signal into the MAX9744 | The SNR/THD comparison that gate G3 exists to answer. |
| Loopback sine fidelity: analog capture of the RC-filtered output during an end-to-end sweep/tone playback through the audio path (SAI1 fallback or TIM1-PWM path), analyzed offline (FFT/THD) from the exported CSV | THD and flatness of the played sweep against the source, end to end. |
| I2C volume transactions: I2C1 decode of MAX9744 volume/mute register writes on SCL (PH9) and SDA (PC1)-- see [Pinout](./pinout.md) for connector routing, device address, and mute-pin rationale | The amplifier receives the expected register writes during a measurement run.  Verify the analyzer's I2C decoder settings against the live Logic 2 UI before capture-- unverified as of this page. |

[Roadmap](../../roadmap.md) section 7 item 4 also lists SPI timing
(W5500, ePaper, SD card), CAN bus signaling, and interrupt latency
as future HIL targets; they sit outside the gate G3 audio path and
outside this page's scope.

## Planned H753ZI Loopback Measurements

Once the ARS `audio_loopback` module lands
(`boards/nucleo-h753zi/AGENTS.md` Module Map: DAC1_OUT1 on PA4,
jumpered to the ADC on PA3), the Logic MSO 2x100 covers that loop
with a channel assignment of its own, separate from the gate G3
captures above:

- Analog 1: DAC1_OUT1 (PA4), the synthesized sweep/tone source.
- Analog 2: ADC input (PA3), the looped-back signal.
- A digital channel: the TIM trigger (a timer output-compare or
  TRGO pin, once `audio_loopback` defines one) that starts each
  DAC/ADC conversion pair.
- A digital channel: a firmware capture-strobe GPIO, toggled at the
  start of each capture window, so an Edge trigger on that strobe
  aligns the analog capture to the exact sample the firmware
  believes it started on.

The Logic MSO's analog-to-digital delay is <=1 ns +/-1 sample
(`logic_mso_data_sheet.pdf`, Performance Characteristics), small
enough relative to typical audio-sweep sample periods to treat the
digital TIM-trigger/strobe edges and the analog DAC/ADC waveforms as
effectively simultaneous-- the basis for proving phase-locked
capture.  This loopback plan is provisional until `audio_loopback`
exists; no pin beyond PA3/PA4 is fixed yet.
