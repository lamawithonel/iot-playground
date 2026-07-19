---
name: saleae-logic
description: >-
  Operate the Saleae Logic MSO 2x100 mixed-signal analyzer for HIL
  bench measurements via the Logic 2 automation API (Python,
  logic2-automation package): digital capture with I2C/SPI decode,
  analog channel capture for PWM-carrier and RC-filtered analog
  measurements, timed captures, and .sal/CSV export.  Use when the
  user says "saleae", "logic analyzer", "capture", "decode", "MSO",
  "logic 2 automation", or asks to record/inspect I2C, SPI, PWM, or
  analog bus traces on the bench.
---

# Saleae Logic MSO

Bench automation for the Saleae Logic MSO 2x100 using the official
Logic 2 desktop application's automation (gRPC) server and the
`logic2-automation` Python client.  This skill targets HIL
measurements for the ARS toolhead-sensor audio path (I2C volume
control, PWM-audio carrier, RC-filtered line signal) as well as
general I2C/SPI bus decode.

## Bench Status (read this first)

As of the last check on this machine:

- The Saleae Logic MSO 2x100 enumerates over USB
  (`21a9:1007`, confirmed via `lsusb`).
- The Logic 2 desktop application is **not installed** on this
  machine (`which logic2` found nothing).
- The `logic2-automation` Python package is **not installed**
  (`pip show logic2-automation` and `python3 -c "import
  logic2_automation"` both failed).
- **No probes are attached to any board.**  Every recipe below is
  documented and cited from the official API references, but none
  has been executed against real hardware in this session.  Treat
  code in this skill as unverified-until-run: validate device IDs,
  channel indices, and analyzer setting keys against the live UI
  before trusting captured data.

See [Limits](#limits-and-what-was-actually-validated) for the exact
scope of what was and was not checked.

## Setup

### 1. Install the Logic 2 desktop application

The automation API is a feature of the Logic 2 desktop application
(not a headless daemon)-- the app must be running, with the
automation server enabled, for any Python script below to connect.
Minimum version per the getting-started guide: Logic 2 (2.4.0+).
Source:
[Getting Started](https://docs.saleae.com/automation/guides/getting-started).

This machine has no `logic2` binary on `PATH`.  Install the desktop
app from <https://www.saleae.com/downloads/> (Linux ships as an
AppImage) before continuing.  Do not install this system-wide from
inside an agent session-- hand the download/install step to the
user or a session with permission to do so.

### 2. Enable the automation server

Launch Logic 2 with the `--automation` flag.  Source:
[Launching the Logic 2 Software and Starting the Socket Interface](https://docs.saleae.com/automation/guides/launching-logic2).

```sh
# Linux (AppImage)
./Logic-2.4.0-master.AppImage --automation
```

The automation (gRPC) server listens on port **10430** by default.
To use a different port:

```sh
./Logic-2.4.0-master.AppImage --automation --automationPort 10555
```

UNVERIFIED: the docs also mention enabling automation via the UI
("following the instructions in the Getting Started guide") but do
not spell out the exact menu path in the fetched text-- confirm in
the running app's Preferences/Options before relying on it.

### 3. Install the Python automation client

```sh
pip install logic2-automation
```

Source:
[Getting Started](https://docs.saleae.com/automation/guides/getting-started).
Package requirements stated there: Logic 2 (2.4.0+), the
`logic2-automation` (1.0.0+) package, and Python 3.8, 3.9, or 3.10.

Caution: this machine runs Python 3.14.6.  The PyPI listing for
`logic2-automation` (latest 1.0.11, released 2025-11-18) only
declares a generic `Python :: 3` classifier, not an explicit upper
bound, so whether it installs cleanly under 3.14 is UNVERIFIED--
confirm with a scratch virtualenv before depending on it, and
prefer a pinned 3.10/3.11 interpreter if install fails.  This
package was **not installed** during this session (no system-wide
installs performed per bench policy); the step above is documented,
not executed.

### 4. Verify USB presence

```sh
lsusb | grep 21a9
```

Expected output on this bench:

```text
Bus 003 Device 006: ID 21a9:1007 Saleae, Inc. Logic MSO
```

This confirms the device is powered and enumerated at the USB
level only.  It does not confirm the Logic 2 app can see or claim
the device (the app must not already be attached elsewhere, and no
other process should hold the device open).

## Capture Recipes

Every snippet assumes Logic 2 is already running with
`--automation` and connects with the default port.  Replace
`device_id` with the value returned by `manager.get_devices()` for
your unit (the getting-started example uses a placeholder,
`'F4241'`).

### Recipe 1: Digital capture + I2C decode + export

Sources:
[Getting Started](https://docs.saleae.com/automation/guides/getting-started)
(worked example) and
[Automation API](https://saleae.github.io/logic2-automation/automation.html)
(`add_analyzer`, `export_data_table`).

```python
from saleae import automation

with automation.Manager.connect(port=10430) as manager:
    device_configuration = automation.LogicDeviceConfiguration(
        enabled_digital_channels=[0, 1],  # e.g. SDA, SCL
        digital_sample_rate=10_000_000,
        digital_threshold_volts=3.3,
    )

    capture_configuration = automation.CaptureConfiguration(
        capture_mode=automation.TimedCaptureMode(duration_seconds=2.0)
    )

    with manager.start_capture(
        device_id="<YOUR_DEVICE_ID>",
        device_configuration=device_configuration,
        capture_configuration=capture_configuration,
    ) as capture:
        capture.wait()

        # UNVERIFIED: the fetched docs confirm the I2C analyzer
        # exposes SDA/SCL channel-role settings (Saleae's I2C user
        # guide: "specify which input channels are used for the
        # I2C signals SDA and SCL") but the exact dict KEY STRINGS
        # were not shown in any fetched page (only the SPI keys
        # were, see Recipe 2).  Open the Logic 2 UI, add an I2C
        # analyzer by hand, and read the field labels verbatim
        # before trusting a settings dict like the one below.
        i2c_analyzer = capture.add_analyzer(
            "I2C",
            label="i2c-bus",
            settings={
                "SDA": 0,  # UNVERIFIED key name-- confirm in UI
                "SCL": 1,  # UNVERIFIED key name-- confirm in UI
            },
        )

        capture.export_data_table(
            filepath="/path/to/i2c_export.csv",
            analyzers=[i2c_analyzer],
        )
```

### Recipe 2: SPI decode

Source:
[Getting Started](https://docs.saleae.com/automation/guides/getting-started)
worked example (settings dict quoted verbatim from the fetched
page).

```python
from saleae import automation

with automation.Manager.connect(port=10430) as manager:
    device_configuration = automation.LogicDeviceConfiguration(
        enabled_digital_channels=[0, 1, 2, 3],
        digital_sample_rate=10_000_000,
        digital_threshold_volts=3.3,
    )

    capture_configuration = automation.CaptureConfiguration(
        capture_mode=automation.TimedCaptureMode(duration_seconds=5.0)
    )

    with manager.start_capture(
        device_id="<YOUR_DEVICE_ID>",
        device_configuration=device_configuration,
        capture_configuration=capture_configuration,
    ) as capture:
        capture.wait()

        spi_analyzer = capture.add_analyzer(
            "SPI",
            label="spi-bus",
            settings={
                "MISO": 0,
                "Clock": 1,
                "Enable": 2,
                "Bits per Transfer": "8 Bits per Transfer (Standard)",
            },
        )

        capture.export_data_table(
            filepath="/path/to/spi_export.csv",
            analyzers=[spi_analyzer],
        )
```

Note: this example omits MOSI, matching the fetched doc example
exactly.  Add a `"MOSI"` key (channel index) if your bus needs it--
UNVERIFIED against the live UI, since the fetched page's example
did not include it.

### Recipe 3: Analog capture (PWM carrier + RC-filtered output)

Source:
[Automation API](https://saleae.github.io/logic2-automation/automation.html)
(`LogicDeviceConfiguration` field list: `enabled_analog_channels`,
`analog_sample_rate`).  Device analog spec (2 channels, 100 MHz
bandwidth, 1.0 GS/s, 9-bit at the standard tier) per the
[Logic MSO 2x100 product page](https://www.saleae.com/products/logic-mso-2x100).

Use this recipe to capture both the raw PWM carrier (before the RC
filter) and the filtered line signal on the other analog channel,
simultaneously, at a rate well above the PWM carrier frequency
(Nyquist plus margin for the RC roll-off shape, not just the
carrier fundamental).

```python
from saleae import automation

with automation.Manager.connect(port=10430) as manager:
    device_configuration = automation.LogicDeviceConfiguration(
        enabled_analog_channels=[0, 1],  # e.g. 0=PWM-in, 1=RC-out
        analog_sample_rate=10_000_000,   # 10 MS/s: >=50x a 200 kHz carrier
    )

    capture_configuration = automation.CaptureConfiguration(
        capture_mode=automation.TimedCaptureMode(duration_seconds=1.0)
    )

    with manager.start_capture(
        device_id="<YOUR_DEVICE_ID>",
        device_configuration=device_configuration,
        capture_configuration=capture_configuration,
    ) as capture:
        capture.wait()

        capture.export_raw_data_csv(
            directory="/path/to/output",
            analog_channels=[0, 1],
            analog_downsample_ratio=1,
        )
```

UNVERIFIED: `digital_threshold_volts` is documented as "valid for
Logic Pro 8 and Logic Pro 16 only" in the fetched API reference--
whether an equivalent threshold field applies to the Logic MSO
2x100's analog-only mode was not confirmed, so it is omitted
above.  Confirm the field requirements for this exact model in the
live UI or SDK error messages before assuming defaults are correct.

### Recipe 4: Timed capture + save .sal + full CSV export

Source:
[Getting Started](https://docs.saleae.com/automation/guides/getting-started)
worked example, and
[Automation API](https://saleae.github.io/logic2-automation/automation.html)
(`export_raw_data_csv`, `save_capture`).

This is the combined recipe for acceptance-test loopback runs: keep
a raw `.sal` (native Logic 2 project file, reloadable in the UI or
via `load_capture()`) alongside CSV exports for any downstream
Python assertion logic.

```python
import os
from datetime import datetime

from saleae import automation

with automation.Manager.connect(port=10430) as manager:
    device_configuration = automation.LogicDeviceConfiguration(
        enabled_digital_channels=[0, 1, 2, 3],
        enabled_analog_channels=[0, 1],
        digital_sample_rate=10_000_000,
        analog_sample_rate=10_000_000,
        digital_threshold_volts=3.3,
    )

    capture_configuration = automation.CaptureConfiguration(
        capture_mode=automation.TimedCaptureMode(duration_seconds=5.0)
    )

    with manager.start_capture(
        device_id="<YOUR_DEVICE_ID>",
        device_configuration=device_configuration,
        capture_configuration=capture_configuration,
    ) as capture:
        capture.wait()

        output_dir = os.path.join(
            os.getcwd(),
            f"output-{datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}",
        )
        os.makedirs(output_dir)

        capture.export_raw_data_csv(
            directory=output_dir,
            digital_channels=[0, 1, 2, 3],
            analog_channels=[0, 1],
        )

        capture.save_capture(
            filepath=os.path.join(output_dir, "capture.sal")
        )
```

`Capture.stop()` is the documented call for ending a
`ManualCaptureMode` capture early; `Capture.wait()` is for
timed/triggered modes and blocks until the configured duration or
trigger condition completes.  Source:
[Automation API](https://saleae.github.io/logic2-automation/automation.html).

## ARS Audio-Path Measurements (gate G3)

Grounded in
[`docs/src/projects/ars-toolhead-sensor/pinout.md`](../../../docs/src/projects/ars-toolhead-sensor/pinout.md)
and
[`docs/src/roadmap.md`](../../../docs/src/roadmap.md) section 7 item
4 (HIL test automation via the Logic 2 gRPC automation API).

| Measurement | What it validates | Recipe |
|---|---|---|
| PWM carrier residue | TIM1_CH1 PWM (~200 kHz carrier, ~10-bit duty ceiling) before the RC low-pass, versus the filtered line signal into the MAX9744-- the SNR/THD comparison that gate G3 exists to answer. | Recipe 3 (analog, dual-channel: raw PWM in, RC-filtered out) |
| Loopback sine fidelity | End-to-end sweep/tone playback through the audio path (SAI1 fallback or TIM1-PWM path), captured on the RC-filtered analog channel and compared against the source sweep for THD/flatness. | Recipe 3 or Recipe 4 (analog capture, offline FFT/THD analysis on the exported CSV) |
| I2C volume transactions | MAX9744 volume/mute register writes over I2C1 (SCL on PH9, SDA on PC1, device address `0x4B`; mute on PD0, per `spike-audio-board.md`). Confirms the amplifier receives the expected register writes during a measurement run. | Recipe 1 (I2C decode-- verify the exact analyzer settings dict keys against the live UI first; marked UNVERIFIED above) |

Roadmap section 7 item 4 also lists SPI timing (W5500, ePaper, SD
card), CAN bus signaling, and interrupt latency as future HIL
targets outside the audio path-- Recipe 2 (SPI) covers the SPI
case; CAN and interrupt-latency recipes are out of scope for this
skill revision.

## NUCLEO-H753ZI probe hookup

Probe plan for the STM32H753ZI Nucleo-144 board's NETWORK bring-up
(the ADR-009 Layer-3 trigger board; see
`boards/nucleo-h753zi/AGENTS.md`), using the Logic MSO 2x100 (2
analog channels, 8 digital channels expandable to 20).  Facts below
are cited to the datasheet (`logic_mso_data_sheet.pdf`) and user
manual (`logic_mso_user_manual.pdf`) at
`~/downloads/datasheets/saleae/`.

This board has no W5500 (that part is on `feather-stm32f405`); the
STM32H753ZI drives an on-chip Ethernet MAC through an on-board
LAN8742 PHY over RMII instead, so the probe points below are RMII
management/clock lines, not an SPI bus.  The repo's `net/` module
that will configure those pins is still planned, not written
(`boards/nucleo-h753zi/AGENTS.md` Module Map).  So every board-side
pin identity below is the standard NUCLEO-H753ZI (UM2407, MB1364
baseboard) reference pinout that ST's own Ethernet HAL examples use
for this board family, not something read back from this repo's
code or re-verified against the UM2407 PDF in this session.  Two
exceptions are cited to the exact UM2407 location already
established elsewhere in this repo: LD1 (green, PB0, UM2407 Rev 6
p.27 Sec 7.6.1) and the USART3 ST-LINK VCP (UM2407 Rev 6 p.28 Sec
7.6.5).  Confirm every other pin against UM2407's pinout tables
before probing.

### Digital channel plan (NETWORK bring-up)

| Channel | Signal | Purpose |
|---|---|---|
| D0 | USART3 console TX (PD8, typical Nucleo-144 VCP pin, UM2407 Rev 6 p.28 Sec 7.6.5) | The debug-UART console falls back to this pin once `net/` lands; confirms firmware is alive and logging. |
| D1 | RMII_REF_CLK (PA1, typical) | 50 MHz PHY reference clock; presence confirms the clock tree feeding the LAN8742 is up before any SMI traffic is expected. |
| D2 | RMII_MDC (PC1, typical) | SMI management clock; toggling confirms the MAC is driving PHY register reads/writes. |
| D3 | RMII_MDIO (PA2, typical) | SMI management data; correlate with D2 to read back PHY register transactions (link status, autonegotiation state) by eye or with a custom decoder. |
| D4 | RMII_CRS_DV (PA7, typical) | Carrier-sense/data-valid-- the simplest single-line "is there RMII receive activity" probe point, i.e. the RMII activity channel. |
| D5 | LD1 (green, PB0, UM2407 Rev 6 p.27 Sec 7.6.1) | Heartbeat/general liveness, already wired in `main.rs`. |
| D6 | LD2 (blue, PE1, typical) | Free for a link-status indicator once `net/` lands. |
| D7 | LD3 (red, PB14, typical) | Free for an error/activity indicator once `net/` lands. |

That fills the 8-channel base kit.  NRST (board reset) is a 9th
signal of interest-- swap it into D0's slot for a dedicated
power-on-reset/warm-reset timing capture (the UART console has
nothing to say during reset anyway), or add a third digital probe
cable: the digital channel count expands to 20 across up to 5 probe
cables, 4 channels each (`logic_mso_user_manual.pdf`, Digital
Measurements).

### Analog channel plan (power/reference integrity)

| Channel | Signal | Purpose |
|---|---|---|
| Analog 1 | 3V3 rail | Confirms the regulator holds 3.3 V under RMII/PHY current transients (autonegotiation, TX bursts); AC-couple to inspect switching-regulator ripple, DC-couple to check absolute droop. |
| Analog 2 | VREF | Confirms the ADC/DAC reference rail is stable and matches 3V3 (typical Nucleo-144 default, tied via a solder bridge-- verify against UM2407 before assuming continuity). |

Both channels are 100 MHz bandwidth, 1 GS/s, 1 MOhm || 16 pF input
impedance, AC/DC-selectable coupling, +/-30 V max at the SMB input
(`logic_mso_data_sheet.pdf`, Performance Characteristics).  A 3.3 V
DC rail falls in the ">=75 mV, <7.5 V" DC offset bucket (+/-20 V
offset range at 1x SMB scale), well within range for both channels
simultaneously (`logic_mso_data_sheet.pdf`, DC Offset Range table).
Vertical sensitivity down to 200 uV lets AC-coupled ripple captures
resolve well below the rail's own noise floor
(`logic_mso_data_sheet.pdf`, Analog Vertical Specifications).

### Grounding

- The Logic MSO's ground reference floats until established by the
  circuit under test; do not energize the ground measurement
  terminal (`logic_mso_user_manual.pdf`, Ground the Measurements
  Properly).
- All digital and analog channel grounds are tied together and to
  the PC/laptop ground, with no isolation between channels
  (`logic_mso_user_manual.pdf`, Safety Related Specifications (All
  Models); Digital Measurements > Safety Considerations).  Use one
  common ground point on the Nucleo board (a GND pin near the
  signals under test) for both the digital probe's black wire and
  the analog probe's ground clip, rather than grounding to two
  physically distant GND pins-- a potential difference between them
  shows up as noise on every channel at once.
- "Always connect ground first and remove it last" on every probe,
  digital and analog alike (`logic_mso_user_manual.pdf`, Connect the
  Probe to the Circuit).
- For the RMII lines (100 MHz max digital input frequency, 5 ns
  minimum pulse width), use the analog probe's spring-ground
  accessory or the digital probe's shortest ground wire-- the manual
  documents this as the accessory providing a "short ground path for
  high frequency measurements" (`logic_mso_user_manual.pdf`, Analog
  Measurements accessory list, item 4).

### Digital threshold voltage

Set the digital probe threshold to 1.65 V, the manual's documented
preset for 3.3 V logic (`logic_mso_user_manual.pdf`, Set the
Threshold Voltage): "3.3V logic: Set threshold to 1.65V."  The
STM32H753's GPIO domain runs at VDD (~3.3 V) for every signal in the
table above, so 1.65 V-- not the 0.9 V 1.8V-logic preset-- is the
correct choice here; 0.9 V only applies if a future signal lives on
a separate 1.8 V I/O rail.  The threshold is per-probe, adjustable
from 0.6 V to 3.1 V if a mixed-voltage signal ever needs a custom
value (`logic_mso_data_sheet.pdf`, Digital Vertical Specifications).

### Trigger recommendations

Trigger sources can be any analog or digital channel, with Edge,
Pulse, or Slope trigger types (`logic_mso_data_sheet.pdf`, Trigger
Specifications).  For a NETWORK bring-up capture, an edge trigger on
NRST's rising edge (end of reset) or on D0's UART console TX (first
falling edge, i.e. the first UART start bit) anchors the capture to
the start of firmware execution; from there, the REF_CLK/MDC/MDIO/
CRS_DV channels show whether the PHY clock, SMI transactions, and
link activity come up in the expected order.

### Future ARS loopback reuse

Once the ARS `audio_loopback` module lands
(`boards/nucleo-h753zi/AGENTS.md` Module Map: DAC1_OUT1 on PA4,
jumpered to the ADC on PA3), the same pod covers that HIL loop with
a different channel assignment:

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
capture.  This reuse plan is speculative until `audio_loopback`
exists; no pin beyond PA3/PA4 is fixed yet.

### Verification status

Nothing above has been probed against real hardware.  Logic 2 is
not installed on this machine (see
[Bench Status](#bench-status-read-this-first)), the NUCLEO-H753ZI's
`net/` module has not been written, and this session did not
re-open the UM2407 PDF to confirm RMII/LED/reset pin numbers-- they
are stated from the standard Nucleo-144 reference design and
flagged "typical" throughout.  Treat every channel index and pin
identity in this section as on-paper until it is run against the
actual board and confirmed in the Logic 2 UI.

## Limits and What Was Actually Validated

This skill was authored with **no probes attached to any board**.
The following is the complete, honest scope of what this session
checked, and nothing more:

- `lsusb | grep 21a9`-- **confirmed present**: `Bus 003 Device 006:
  ID 21a9:1007 Saleae, Inc. Logic MSO`.  This is USB-enumeration
  evidence only (device powered and visible to the host), not proof
  that Logic 2 or the automation server can open it.
- `which logic2`-- **not found**.  The Logic 2 desktop application
  is not installed on this machine.
- `pip show logic2-automation` / `python3 -c "import
  logic2_automation"`-- **not found / import error**.  The Python
  automation client is not installed.
- **No capture was started, run, or asserted.**  Every code snippet
  above is a documentation-grounded recipe, not a tested script.
  Device IDs, channel-to-signal mappings, analyzer setting keys
  (especially the I2C ones), and sample-rate adequacy for the
  200 kHz PWM carrier must all be confirmed against the live Logic
  2 UI and a real capture before any assertion in a HIL test relies
  on this skill's code.
- No hardware other than the Saleae was touched in the process of
  writing this skill.  The J-Link (`1366:1020`) and ST-LINK
  (`0483:374e`) enumerate on this bench for unrelated firmware
  workstreams and were not opened, attached, or flashed.
- The [NUCLEO-H753ZI probe hookup](#nucleo-h753zi-probe-hookup)
  section carries the same caveats plus one more: its RMII/LED/reset
  pin identities are the standard Nucleo-144 reference pinout, not
  something re-verified against the UM2407 PDF or this repo's
  (not-yet-written) `net/` module in this session.

## References

- [Getting Started](https://docs.saleae.com/automation/guides/getting-started)
- [Launching the Logic 2 Software and Starting the Socket Interface](https://docs.saleae.com/automation/guides/launching-logic2)
- [Automation API reference](https://saleae.github.io/logic2-automation/automation.html)
- [Automation API overview (support site)](https://www.saleae.com/support/extensions-api/automation-api/automation)
- [I2C Analyzer User Guide](https://www.saleae.com/support/protocol-analyzers/analyzer-user-guides/using-i2c)
- `logic_mso_data_sheet.pdf` (`~/downloads/datasheets/saleae/`)-- Logic
  MSO 2x100 performance characteristics (analog/digital vertical and
  horizontal specs, trigger specs).
- `logic_mso_user_manual.pdf` (`~/downloads/datasheets/saleae/`)--
  probe connection, grounding, and threshold-voltage procedures.
- [Logic MSO 2x100 product page](https://www.saleae.com/products/logic-mso-2x100)
- [`logic2-automation` on PyPI](https://pypi.org/project/logic2-automation/)
