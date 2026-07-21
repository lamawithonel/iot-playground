---
name: debug-probe
description: >-
  Flash, attach, and debug the embedded Rust firmware in this repo
  over SWD using probe-rs and defmt-RTT logging.  Use when asked to
  flash firmware, debug on-device behavior, attach RTT, read defmt
  output, list or select a probe, reset the target, or start a GDB
  session against hardware.  Trigger phrases: flash, debug, RTT,
  attach, probe, probe-rs, defmt, on-device, gdb.
---

# Debug Probe

Flash, attach, and debug the on-device firmware in this workspace
using `probe-rs` 0.31.0 (installed via `mise`, `cargo:probe-rs-tools`)
and the `defmt`-over-RTT log channel.  This skill covers probe
inventory and selection, the flash/attach/reset/info flows, bounded
RTT capture for automation, and the GDB fallback path.

## When to Use This Skill

- Asked to flash firmware, run firmware on hardware, or "see what
  the device is doing"
- Asked to debug a HardFault, panic, or unexpected on-device
  behavior
- Asked to read or capture RTT/defmt log output
- Asked to attach to a running target without reflashing it
- More than one debug probe may be connected and the correct one
  must be selected explicitly

## Prerequisites

- `mise install` has installed `probe-rs-tools` 0.31.0 (provides
  `probe-rs`, `cargo-embed`, `cargo-flash`).  The BSP target
  `thumbv7em-none-eabihf` runner is set to `probe-rs run` in
  `.cargo/config.toml`, so `cargo run` flashes through `probe-rs`
  automatically.
- **USB access requires an unsandboxed shell in this environment.**
  See "Sandbox Note" below-- try the plain command first, and only
  disable the sandbox if it fails with a USB/device-open error.
- A GDB binary with ARM support (`arm-none-eabi-gdb` or
  `gdb-multiarch`) is required for the GDB fallback flow.  Neither
  is installed in this environment as of this writing; check with
  `which arm-none-eabi-gdb gdb-multiarch` before relying on it.

## 1. Probe Inventory and Board Mapping

List connected probes before doing anything else:

```console
$ probe-rs list
The following debug probes were found:
[0]: STLink V3 -- 0483:374e:<serial> (ST-LINK)
[1]: J-Link -- 1366:1020:<serial> (J-Link)
```

(Validated live on 2026-07-19; both probes were connected.)

| Probe | VID:PID | Board | Chip | Ownership |
|-------|---------|-------|------|-----------|
| J-Link | `1366:1020` | Feather STM32F405 (`boards/feather-stm32f405`) | `STM32F405RGTx` | This skill's default.  Safe to use. |
| ST-LINK V3 | `0483:374e` | NUCLEO-H753ZI | `STM32H753ZITx` | Another workstream's.  **Never touch.** |

Notes on the ST-LINK entry:

- The board attached to the ST-LINK does not correspond to this
  repo's `boards/nucleo-n657x0` profile (that profile targets
  `STM32N657X0`/Cortex-M55 and is workspace-excluded scaffold with
  no hardware attached-- see `boards/nucleo-n657x0/AGENTS.md`).  The
  physical NUCLEO-H753ZI on the bench is out-of-scope hardware for
  a different task; treat it as such rather than assuming it is the
  N6 profile's board.
- `probe-rs list` is read-only USB enumeration and is always safe
  to run, including against probes you must not otherwise touch.

**Because more than one probe is connected, every `probe-rs`
invocation MUST include an explicit `--probe VID:PID`.**  Omitting
`--probe` with two or more probes attached drops `probe-rs` into an
interactive numbered prompt; in a non-interactive agent shell this
fails outright rather than picking a safe default (validated: see
"Gotchas" below).  Never rely on positional/interactive selection.

### Embed.toml Layout and the Retired Preset Mechanism

The repo ships per-board `Embed.toml` files plus a root default:

- `Embed.toml` (root)-- a single `default` table for the workspace
  default member (feather: chip `STM32F405RGTx`, SWD, RTT channel 0
  named `defmt`).  No probe pin; multi-probe benches pin via
  `Embed.local.toml` or `PROBE_RS_PROBE`.
- `boards/feather-stm32f405/Embed.toml`-- board-local config, same
  shape as the root default.
- `boards/nucleo-h753zi/Embed.toml`-- chip `STM32H753ZITx` with the
  on-board ST-LINK pinned (`usb_vid`/`usb_pid` 0483:374e) so a
  multi-probe bench never grabs the J-Link.

**History (validated 2026-07-19):** the root `Embed.toml` used to
define named preset tables (`feather`, `microbit`, ...) and its
header advertised `PROBE_RS_CONFIG_PRESET=<name>` for board
selection.  With `probe-rs-tools` 0.31.0 as pinned in `.mise.toml`,
that mechanism fails outright:

```console
$ PROBE_RS_CONFIG_PRESET=feather probe-rs run --dry-run \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405
Error: Config preset 'feather' not found.
```

`probe-rs`'s own `--preset`/`PROBE_RS_CONFIG_PRESET` mechanism reads
a separate preset config that this repo does not define; it is not
the same thing as `cargo-embed`'s `Embed.toml` profile tables.  The
preset tables and header comments were removed 2026-07-19; if you
see `PROBE_RS_CONFIG_PRESET` referenced anywhere, treat it as stale.

**What actually works (validated):** set `PROBE_RS_CHIP` (or pass
`--chip` directly).  This is what `.mise/tasks/test/device` and
`.mise/tasks/test/smoke` already do:

```console
$ PROBE_RS_CHIP=STM32F405RGTx probe-rs run --dry-run \
    --probe 1366:1020 target/thumbv7em-none-eabihf/debug/feather-stm32f405
Read 0xe0042004 = 0
Write 0xe0042004 = 0x00000007
...
```

(`--dry-run` exercises the full connect/init path against a fake
probe with no hardware touched-- safe to use for sanity-checking
flags before running for real.)

Chip names per board, for direct `--chip`/`PROBE_RS_CHIP` use:

| Board | Chip |
|-------|------|
| `boards/feather-stm32f405` | `STM32F405RGTx` |
| `boards/nucleo-h753zi` | `STM32H753ZITx` |
| `boards/nucleo-n657x0` (scaffold, no hardware) | `STM32N657X0` |

## 2. Flows

All examples target the J-Link + Feather STM32F405 (the only board
this skill may touch).  Build first if there is no fresh ELF:

```console
$ cargo build -p feather-stm32f405 --target thumbv7em-none-eabihf
```

### Flash and Run With RTT

Either of these flashes the board and streams decoded `defmt` RTT
output until interrupted:

```console
$ cargo run -p feather-stm32f405 --release -- \
    --probe 1366:1020 --chip STM32F405RGTx
```

```console
$ probe-rs run --probe 1366:1020 --chip STM32F405RGTx \
    target/thumbv7em-none-eabihf/release/feather-stm32f405
```

`cargo run` invokes `probe-rs run` via the `runner` set in
`.cargo/config.toml`; flags after `--` are appended to that
invocation, so `--probe`/`--chip` reach `probe-rs` either way.

### Attach Without Reflashing

Use `attach` to open RTT against firmware that is already running,
without touching flash.  This was validated live against the Feather
STM32F405 while it was running previously-flashed firmware:

```console
$ timeout 8 probe-rs attach --chip STM32F405RGTx --probe 1366:1020 \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405
6692590.114447 [INFO ] SEN66: PM2.5=3 CO2=779 T=274 RH=390 NOx=10 (feather_stm32f405 src/sensor/sen66.rs:103)
6692590.114564 [WARN ] Sensor channel full — dropping newest reading (feather_stm32f405 feather-stm32f405/src/main.rs:247)
...
6692638.945009 [INFO ] Connecting to MQTT broker at 192.168.1.1:8883... (feather_stm32f405 src/network/mqtt.rs:141)
6692638.946725 [ERROR] MQTT session failed: SocketError (feather_stm32f405 src/network/mqtt.rs:163)
6692638.946771 [INFO ] Reconnecting in 60 seconds (backoff)... (feather_stm32f405 src/network/mqtt.rs:167)
...
Received SIGTERM, exiting
Exited by user request
```

(Full command run 2026-07-19; exit code 124 from `timeout`, which
`probe-rs attach` handled cleanly.)

The ELF passed to `attach` must match the binary currently on the
device (same debug/release build) so defmt symbol decoding lines up
with the running firmware.  Pick the more recently built ELF under
`target/thumbv7em-none-eabihf/{debug,release}/feather-stm32f405` if
unsure which was last flashed.

`attach`'s own `--help` text is misleadingly copy-pasted from `run`
("the binary will be flashed and run normally")-- in practice it does
**not** reflash; it only opens RTT on the currently running target.

### Reset

```console
$ probe-rs reset --chip STM32F405RGTx --probe 1366:1020
```

(Not exercised live in this session-- flag set is a direct
parameterization of the validated `probe-rs reset --help` PROBE
CONFIGURATION options, identical to `attach`/`run`/`info`.)

### Info

Query the probe and target without flashing:

```console
$ probe-rs info --probe 1366:1020 --chip STM32F405RGTx
Probing target via JTAG
-----------------------

Failed to identify target using protocol JTAG: An error with the usage of the probe occurred

Caused by:
    Invalid data length. IR bits: 0, expected: 0
Probing target via SWD
----------------------

ARM Chip with debug port Default:

Debug Port: DPv1, Designer: ARM Ltd
└── V1(0) MemoryAP
    └── 0 MemoryAP (AmbaAhb3)
        ├── 0xe00ff000 ROM Table (Class 1), Designer: STMicroelectronics
        ├── 0xe0001000 Generic
        ├── 0xe0000000 Peripheral test block
        ├── 0xe0040000 Generic
        └── 0xe0041000 Cortex-M4 ETM   (Coresight Component)

Debug port version DPv1 does not support SWD multidrop. Stopping here.
```

(Validated live 2026-07-19.) The JTAG probe attempt failing first is
expected and benign-- the Feather STM32F405 only exposes SWD (see
`boards/feather-stm32f405/Embed.toml`'s `protocol = "Swd"` comment).
Add `--protocol swd` to skip the JTAG attempt and its harmless error
if the noise is undesirable in automation logs.

### Capturing Bounded RTT Output in Automation

Prefix with `timeout <seconds>` so an agent or CI job never blocks
waiting on a live device.  This is the exact pattern used by
`.mise/tasks/test/smoke`:

```bash
_rtt_file="/tmp/rtt-capture.$$"
timeout 60 probe-rs run --chip STM32F405RGTx --probe 1366:1020 \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405 \
    > "$_rtt_file" 2>&1 || true
```

`timeout` sends SIGTERM; `probe-rs` exits 124 under `timeout` and
prints "Exited by user request"-- treat that as a clean bounded
capture, not a failure, and check `$_rtt_file` for panics/HardFaults
per the same grep checks `test/smoke` uses.  When attaching (not
flashing) instead, swap `probe-rs run` for `probe-rs attach` and keep
the same `timeout` prefix, as validated above.

### DEFMT_LOG Levels

`DEFMT_LOG` is a build-time env var baked into the firmware binary
by the `defmt` macros, not a probe-rs runtime flag-- set it before
`cargo build`/`cargo run`, then reflash:

```console
$ DEFMT_LOG=debug cargo run -p feather-stm32f405 --release -- \
    --probe 1366:1020 --chip STM32F405RGTx
```

Valid levels (least to most verbose): `off`, `error`, `warn`,
`info`, `debug`, `trace`.  The repo default is `info`, set in both
`.cargo/config.toml` and `.mise.toml`'s `[env]` table-- this matches
the `[INFO ]`/`[WARN ]`/`[ERROR]` levels seen in the validated
`attach` output above (no `DEBUG`/`TRACE` lines because the flashed
build used the default).  Per-module filters use the usual defmt
syntax, e.g. `DEFMT_LOG=warn,feather_stm32f405::network=trace`.

### Connect-Under-Reset Recovery

If a probe can't attach because the target is stuck in a bad state
(stuck in `WFI`, brown-out, corrupted clock config), add
`--connect-under-reset` to force the reset line low while attaching:

```console
$ probe-rs run --chip STM32F405RGTx --probe 1366:1020 \
    --connect-under-reset \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405
```

(Not exercised live-- the flag is a direct parameterization of the
validated PROBE CONFIGURATION options shared across `run`/`attach`/
`info`/`reset`/`gdb`, confirmed present via `--help`.) The root
`Embed.toml` has a commented-out `connect_under_reset = true` under
`[default.general]` as a standing reminder that this knob exists,
but since `Embed.toml` presets are not read by this probe-rs version
(see above), pass the flag explicitly rather than uncommenting it.

## 3. GDB Fallback

Prefer RTT/defmt for normal debugging-- it is already wired into
every board and needs no extra setup.  Reach for GDB when RTT alone
can't answer the question: inspecting register/memory state at a
breakpoint, single-stepping through a HardFault handler, or setting
watchpoints on a specific address.

Start the GDB server (validated via `probe-rs gdb --help`; default
bind is `localhost:1337`):

```console
$ probe-rs gdb --chip STM32F405RGTx --probe 1366:1020 \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405
```

Then, from another shell, connect with an ARM-aware GDB:

```console
$ arm-none-eabi-gdb -ex "target extended-remote :1337" \
    target/thumbv7em-none-eabihf/debug/feather-stm32f405
```

`gdb-multiarch` works identically if `arm-none-eabi-gdb` isn't
installed. **Neither is installed in this environment as of this
writing** (only host `gdb` is present)-- the connect step above
follows the standard GDB remote-target syntax but was not exercised
live here; install one of the two ARM-aware GDB builds before
relying on this flow.  `probe-rs gdb --gdb <path>` can also spawn GDB
directly instead of connecting from a second shell.

## 4. Safety Rules for Agents

- **Never flash, attach, reset, or otherwise open a board another
  task owns.** In this environment that means the ST-LINK
  (`0483:374e`) is off-limits entirely-- `probe-rs list` may show it,
  but no other `probe-rs` subcommand may reference it.
- **Always pass explicit `--probe VID:PID` and `--chip <CHIP>`** on
  every invocation, even when only one probe is connected.  Relying
  on default/interactive probe selection is what causes
  cross-board accidents, and it fails hard in a non-interactive
  agent shell anyway (validated above).
- Prefer `--dry-run` to sanity-check flags (chip resolution, probe
  selection) before touching real hardware, especially when unsure
  which ELF matches the running firmware.
- Always bound RTT sessions with `timeout <seconds>` in automation;
  never leave `probe-rs run`/`attach` running unbounded in a
  scripted or agent context.
- Use `attach`, not `run`, when the goal is to observe already-
  running firmware-- `run` reflashes.

## Sandbox Note

`probe-rs` needs direct USB device access.  In this environment, the
default sandboxed `Bash` tool call blocks that:

```
Error while probing target: Failed to open the debug probe.
Caused by:
    0: USB Communication Error
    1: error while opening the USB device: device not found (errno 2)
```

`probe-rs list` (pure enumeration) worked fine sandboxed in this
session, but `info`/`attach`/`run`/`reset`/`gdb` all need
`dangerouslyDisableSandbox: true`.  Try the plain command first; only
disable the sandbox after seeing this exact USB error, and still
follow the safety rules above (explicit `--probe`/`--chip`, never
the ST-LINK).

## Gotchas

- With two or more probes connected and no `--probe` given,
  `probe-rs` drops into an interactive numbered prompt.  In a
  non-interactive shell this fails immediately:

  ```console
  $ probe-rs run --dry-run target/thumbv7em-none-eabihf/debug/feather-stm32f405
  Available Probes:
  0: STLink V3 -- 0483:374e:<serial> (ST-LINK)
  1: J-Link -- 1366:1020:<serial> (J-Link)
  Selection: Error: Failed to parse probe index
  ```

  This is a live demonstration of why `--probe` is mandatory here,
  not just a style preference.
- `PROBE_RS_CONFIG_PRESET`/`--preset` does not read `Embed.toml`'s
  named tables with `probe-rs-tools` 0.31.0 (see above)-- use
  `--chip`/`PROBE_RS_CHIP` instead, matching the existing
  `test:device`/`test:smoke` mise tasks.
- `probe-rs attach`'s `--help` text says the binary "will be flashed
  and run normally"; this is inherited boilerplate from `run` and is
  wrong for `attach`-- `attach` does not reflash.
- `attach` may emit a quick burst of backlogged RTT lines before
  settling into live streaming if the target produced output while
  nothing was consuming the RTT buffer; this is expected, not a bug.
