# ARS Toolhead Sensor

An active acoustic resonance spectroscopy (ARS) device clamped to
a Bambu Lab H2C toolhead.  An audio exciter injects swept-sine
energy into the toolhead structure, a MEMS microphone captures the
acoustic response, and resonance features classify filament
presence in the hot end and cold end.

## Purpose and Staged Goals

The project delivers value in three stages, each feeding the next:

1. **Active ARS sensing.**  Sweep, capture, and classify: detect
   filament presence in the hot end and cold end from resonance
   signatures.
2. **Labeled dataset.**  Each ARS classification yields a
   positive/negative label; labels recorded alongside operating
   sound accumulate into a training corpus.
3. **Passive CNN.**  A CNN trained on that corpus detects
   filament at cold-pull start from operating sound alone, with
   no active excitation.

Generalized fault detection (clogs, mechanical wear, or other
anomalies) is explicitly out of scope.

## System Architecture

The signal chain runs from sweep synthesis on the MCU, through
the acoustic path across the toolhead, and back into the MCU for
analysis:

```text
sweep generation (MCU, swept sine)
        |
        |  audio output path -- open spike:
        |  filtered PWM vs external DAC/codec
        v
MAX9744 class-D amplifier (line-level in, 20 W)
        |
        v
EX25VT2-4 exciter (clamped to H2C toolhead)
        |
        v
H2C toolhead structure  <-- filament presence
        |                   shifts resonances
        v
SPH8878LR5H-1 MEMS mic (modified breakout, analog out)
        |
        v
MCU ADC capture
        |
        v
FFT / feature extraction
        |
        v
classification (present / absent)
        |
        v
telemetry (framework MQTT stack, when ported)
```

## Firmware Architecture

RTIC 2.x application on the Embassy HAL (no embassy-executor),
`no_std`, no heap, defmt logging-- the same framework rules as
the feather board.  Planned task topology:

| Task | Purpose |
|------|---------|
| `init` | Clocks, pins, audio output, ADC setup |
| `sweep_engine` | Swept-sine synthesis into the audio output path |
| `capture` | ADC capture windows synchronized to the sweep |
| `analysis` | FFT, feature extraction, classification |
| `telemetry` | Publish labels/features via the framework MQTT stack once ported |
| `idle` | WFI sleep |

Pure signal-processing logic belongs in `core/`; the board crate
holds hardware I/O and RTIC task wiring only, per the `boards/`
rules.

## Phased Delivery

1. **Bring-up spike.**  Boot chain (probe-rs vs
   STM32CubeProgrammer for the flashless signed-FSBL flow),
   blinky, defmt over RTT, embassy-stm32 N6 peripheral survey.
2. **Audio I/O.**  Decide and implement the audio output path to
   the MAX9744; capture mic samples through the ADC.
3. **Sweep + capture.**  Synchronized sweep generation and
   windowed capture; raw response spectra out over defmt.
4. **Resonance analysis.**  FFT and feature extraction on-device;
   present/absent classification from resonance signatures.
5. **Dataset tooling.**  Export labeled captures (ARS label plus
   operating sound) into a host-side training corpus.
6. **Passive CNN.**  Train on the corpus; deploy per the
   inference-path spike (Neural-ART, CMSIS-NN, or host-side).

## Open Spikes

Unverified claims stay here until a bring-up spike settles them.

- **Flash/debug flow.**  probe-rs vs STM32CubeProgrammer for the
  flashless signed-FSBL external-NOR boot chain; the probe-rs
  flow is unverified.
- **embassy-stm32 N6 coverage.**  An `stm32n657x0` feature exists
  (Cortex-M55 target `thumbv8.1m.main-none-eabihf`), but
  peripheral coverage (ADC, timers, DMA) for this part is
  unverified.
- **Audio output path.**  Provisionally filtered PWM (TIM1_CH1 on
  PE9 plus an external RC low-pass) into the MAX9744 line input;
  confirmed or overturned at gate G3, with SAI1 plus an external
  I2S DAC as the fallback (pinout.md Decisions and Gates).
- **ADC sampling strategy.**  DMA capture is provisionally fixed
  to GPDMA1 adc1_dma REQSEL 7, linked-list circular mode,
  confirmed at gate G1 (pinout.md Decisions and Gates).  Sample
  rate, trigger source, and anti-aliasing for the mic input
  remain open.
- **Inference deployment.**  Neural-ART NPU vs CMSIS-NN on the
  M55 vs host-side-only inference for the passive CNN.
