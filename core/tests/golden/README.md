# Golden-Vector Corpus for the ARS Fixed-Point Pipeline

Checked-in fixtures pairing described excitation inputs with the
bit-exact fixed-point outputs of `iot-core`'s DSP kernels and ARS
record encoders.  `core/tests/golden.rs` replays every fixture
through the current kernels and fails on any single-bit difference,
so a kernel behavior change cannot land silently.

## Regeneration

The corpus is generated-- never hand-edited-- by the
workspace-excluded host tool
[`tools/ars-synth`](../../../tools/ars-synth/AGENTS.md):

```sh
cargo run --manifest-path tools/ars-synth/Cargo.toml \
  --target "$(rustc -vV | sed -n 's/^host: //p')" -- gen
```

`ars-synth check` regenerates in memory and diffs against these
files, exiting non-zero on any mismatch, missing file, or stray
`.txt` file.  Generation is deterministic: fixed seeds, fixed
timestamp constants, no wall-clock reads.

## File Format (`ars-golden-v1`)

Plain text, three parts, in order:

1. `#` comment lines (description, regeneration pointer).
2. `key = value` parameters.  Every fixed-point coefficient,
   initial condition, seed, and count a replay needs is here as a
   decimal integer; a few values are words (`kind`, `input`).
   Frequencies are informational, in deci-Hz (`*_dhz`).
3. `[name]` sections: arrays of whitespace-separated decimal
   integers, wrapped before 80 columns.

Sections whose names start with `expected_` are kernel outputs and
are recomputed on replay; all other sections are inputs (e.g.
`input_samples_i16`, `tone_coeff_q30`, `goertzel_coeff_q30`) or
informational (`*_freq_dhz`, `tone_phase_urad`).

The parser, renderer, and integer replay engine live in one shared
module, [`driver.rs`](driver.rs), included via `#[path]` by both
`core/tests/golden.rs` and `tools/ars-synth`.  Replay is
integer-only: fixtures store all fixed-point coefficients, so
neither the test nor `check` mode ever recomputes trigonometry.
Only `ars-synth gen`'s signal *design* step uses `f64`.

## Fixture Kinds

| `kind` | Drives | Expected sections |
|--------|--------|-------------------|
| `mls` | `MlsGen` (LFSR orders 9/10/11) | descriptor, samples |
| `nco` | `Nco` resonator sine | samples |
| `biquad` | `Biquad` impulse/step response | output samples |
| `multitone` | Schroeder-phase `Nco` sum + `GoertzelBin` | samples, bin magnitudes |
| `goertzel_tone` | `Nco` tone into 3-bin straddle + off-bin | bin magnitudes |
| `ess_plant` | Stored Farina ESS through `Plant` + `GoertzelBin` | plant output, bin magnitudes |
| `capture` | MLS -> `Plant` -> `CaptureRecord`/`LabelRecord` | ARSC bytes, ARSL bytes (CRC included) |

Per the ARS research doc (section 8), the corpus covers the three
excitation families of record (MLS primary, Schroeder multitone,
Farina ESS) and the Goertzel bin magnitudes that feed the
interpolated-peak feature.  Q (half-power/ring-down), parabolic
interpolation, and per-bin coherence kernels do not exist in
`core/` yet; the corpus grows with them when they land.
