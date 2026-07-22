# IoT Playground

An embedded Rust IoT framework for STM32 microcontrollers, with a focus on
real-time performance and security.  Microchip ATSAM support is planned.

## Quick Start

Install [mise](https://mise.jdx.dev/), then:

```bash
mise trust . && mise run setup   # install pinned tools, check the bench
mise run boards                  # list boards and their projects
mise run build                   # build the default board (feather-stm32f405)
mise run build nucleo-h753zi     # build one board
mise run build all               # build all workspace-member boards
mise run build nucleo-n657x0 --project net   # a project on a multi-project board
mise run flash                   # build + flash (+ attach RTT)
mise run test                    # host unit tests, no hardware needed
```

Every wrapper prints the exact `cargo` command it runs, and `cargo`,
`cargo embed`, and `probe-rs` stay usable directly from any board
directory (`cd boards/<board> && cargo build`).  `mise tasks` lists
everything else (docs, broker, TLS certs); setup detail lives under
[Prerequisites](#prerequisites).

Working on the same targets for a while?  Pin them:

```bash
export BOARDS='feather-stm32f405,nucleo-n657x0:net'
mise run build    # builds both, nucleo-n657x0 with the net project
mise run flash    # targets the first entry (feather-stm32f405)
```

`BOARDS` takes comma-separated `board[:project]` entries; a listed
project becomes that board's default while the variable is set, and
`--project` still overrides.  Explicit board arguments beat the pin.
Single-board tasks (`flash`, `test:device`) take the first entry, so
list your primary board first.

The pin reaches the primitives too: the first entry's chip (and its
`--speed` workaround, where one exists) exports as
`PROBE_RS_CHIP`/`PROBE_RS_SPEED`, which probe-rs reads natively-- so
bare `probe-rs attach`/`gdb`/`reset` need no flags in a
mise-activated shell.  Board-routed test tasks (`test:device`,
`test:smoke`) follow the pin as well; a board without a given suite
skips loudly.

## Overview

This project provides a multi-device capable embedded firmware framework using:
- **RTIC 2.x** for interrupt-driven task scheduling
- **Embassy HAL** for hardware abstraction (without executor)
- **embedded-tls** for secure communications
- **MQTT v5.0** for messaging
- **no_std** environment with zero heap allocation

## Supported Boards

The board roster and per-board documentation live in
[docs/src/boards/](docs/src/boards/README.md).

## Prerequisites

### Tool Installation with mise (Recommended)

[mise](https://mise.jdx.dev/) manages dev tools (Rust toolchain, flip-link,
mdBook) and provides task runner commands.  Install mise, then:

```bash
mise trust .
mise run setup
```

`setup` runs `mise install` (pinned versions of Rust with the
`thumbv7em-none-eabihf` cross-compilation target, `rustfmt`, and
`clippy`, plus flip-link, probe-rs-tools, and mdBook), then prints
an advisory bench report-- every gap comes with its fix command.
`mise tasks` lists every available task (docs, broker, TLS certs,
tests); the common ones appear in [Quick Start](#quick-start).

### Required Tools

1. **mise** (tool manager and task runner):
   ```bash
   curl https://mise.jdx.dev/install.sh | sh
   mise trust .
   mise install
   ```

   This installs Rust (with the ARM target), flip-link,
   probe-rs-tools (includes `probe-rs`, `cargo-embed`, and
   `cargo-flash`), and mdbook.

2. **Container runtime** (for the MQTT test broker)-- one of:
   - [Podman](https://podman.io/getting-started/installation)
     (preferred)
   - [Docker](https://docs.docker.com/get-docker/)

   ```bash
   # Fedora/RHEL (Podman)
   sudo dnf install podman

   # Debian/Ubuntu (Podman)
   sudo apt install podman

   # Or install Docker: https://docs.docker.com/get-docker/
   ```

### Optional Tools

- **cargo-binutils** (for binary inspection):
  ```bash
  cargo install cargo-binutils
  ```

## Building and Flashing

The `mise run build`/`flash` wrappers in [Quick Start](#quick-start)
front these commands; everything below also works directly.

### From the workspace root

The workspace default member is `feather-stm32f405`, so plain
`cargo run`/`cargo embed` from the root targets the feather:

```bash
cargo run --release     # build + flash + attach RTT (feather)
cargo embed --release   # same, via cargo-embed
```

This will:
1. Build the firmware for the board
2. Use the correct linker configuration automatically
3. Flash it via your connected debug probe
4. Attach to RTT for live log viewing

### Board Selection

Each active board directory carries its own `Embed.toml` with the right
chip (and, where safe, probe) settings.  To work on a non-default
board, run from its directory:

```bash
cd boards/<board>
cargo embed --release
```

Or stay at the root and be explicit:

```bash
cargo build -p <board> --release
probe-rs run --chip <CHIP> --probe <VID:PID> \
  target/thumbv7em-none-eabihf/release/<board>
```

The exact chip and probe values for each board live on its page
under [docs/src/boards/](docs/src/boards/README.md).

With more than one debug probe attached, always pin the probe
(`--probe VID:PID` or the `PROBE_RS_PROBE` environment variable);
an unpinned invocation falls into probe-rs's interactive
selection prompt.  Per-bench overrides go in an untracked
`Embed.local.toml` next to the relevant `Embed.toml`.

### Build Only

```bash
# Build the default member (feather) from the workspace root
cargo build --release

# Build a specific board
cargo build -p nucleo-h753zi --release

# Or build from a board directory
cd boards/feather-stm32f405
cargo build --release
```

The root `.cargo/config.toml` provides common settings for all boards:
- Generic runner: `probe-rs run`
- Target: `thumbv7em-none-eabihf` (Cortex-M4F/M7F with FPU)
- Linker scripts: `link.x` and `defmt.x`
- Linker flags: `--nmagic`

### Flash Only

Board-specific flash commands live in each board's page under
[docs/src/boards/](docs/src/boards/README.md).

## Debugging

### View Logs with RTT

The firmware uses `defmt` for efficient logging over RTT (Real-Time Transfer).
Board-specific attach commands live in each board's page under
[docs/src/boards/](docs/src/boards/README.md).

### Set Log Level

```bash
# Set via environment variable
export DEFMT_LOG=debug  # or: trace, info, warn, error

# Then run cargo embed
cargo embed --release
```

### Interactive Debugging with probe-rs

probe-rs provides a GDB server; the board-specific invocations live in
each board's page under [docs/src/boards/](docs/src/boards/README.md).

## Project Structure

```
iot-playground/
├── .mise.toml              # mise tool versions and task definitions
├── Cargo.toml              # Workspace root with shared dependencies & default-members
├── Embed.toml              # probe-rs defaults for the default member (feather)
├── .cargo/config.toml      # Root config with generic probe-rs runner
├── AGENTS.md               # Project architecture and constraints
├── .agents/                # Agent rules, skills, subagents (.claude/ symlinks here)
├── boards/                 # Board profiles (specific chip + peripherals + applications)
│   ├── feather-stm32f405/  # Feather STM32F405 board profile (active, flagship)
│   │   ├── Embed.toml      # Board-specific probe-rs config (optional)
│   │   ├── src/            # Board-specific firmware code
│   │   ├── tests/          # Layer 2 bring-up tests (embedded-test)
│   │   └── memory.x        # Memory layout for this board
│   ├── nucleo-h753zi/      # ST NUCLEO-H753ZI board profile (active workspace member)
│   └── nucleo-n657x0/      # ST NUCLEO-N657X0-Q scaffold (ARS toolhead sensor; workspace-excluded)
├── core/                   # Platform-agnostic business logic (no_std)
├── hal-abstractions/       # Hardware abstraction traits (no_std)
├── test/                   # Test infrastructure
│   ├── broker/             # Mosquitto MQTT test broker (containerized)
│   ├── features/           # Gherkin feature files (smoke and integration layers)
│   ├── mqtt-subscriber/    # Native MQTT subscriber used during smoke tests
│   ├── smoke-validator/    # Cucumber-RS smoke test validator
│   └── scripts/            # Shared test scripts (TLS cert generation)
└── docs/                   # Documentation (mdBook; docs/src/projects/ holds per-project docs)
```

### Configuration Files

**Workspace-level:**
- `.cargo/config.toml`: Root configuration with generic `probe-rs run`
  runner and common linker flags
- `Cargo.toml`: Sets `default-members = ["boards/feather-stm32f405"]` for
  cargo commands
- `Embed.toml`: probe-rs defaults for `cargo embed` from the root (default
  member: feather)

**Board-level:**
- `boards/*/Embed.toml`: Board-specific probe-rs config (chip, probe, RTT)
- `boards/*/memory.x`: Memory layout for the specific chip
- `boards/*/src/`: Board-specific application code and configuration

### Board Profiles vs. Boards

The board-profile concept and the profile roster are documented in
[docs/src/boards/](docs/src/boards/README.md).

## Development Workflow

1. **Make changes** to the source code in
   `boards/feather-stm32f405/src/`
2. **Build and flash** (requires a probe connected via
   USB):
   ```bash
   cargo run --release
   ```
3. **View logs** in real-time via RTT output
4. **Run tests** (see [Testing](#testing) below):
   ```bash
   mise run test
   ```

### Working with a Specific Board

To work on a specific board profile, change to its directory so
its `Embed.toml` (chip, probe, RTT settings) applies:

```bash
cd boards/nucleo-h753zi
cargo run --release
cargo build --release
```

From the root, use `-p <board>` for builds and explicit
`--chip`/`--probe` flags for probe-rs commands (see
[Board Selection](#board-selection)).

## Architecture

### RTIC-First Design

This framework uses **RTIC 2.x** (Real-Time Interrupt-driven Concurrency) for
task scheduling:
- Hardware interrupts trigger tasks
- `WFI` (Wait For Interrupt) for power efficiency
- Zero-cost abstractions for real-time guarantees
- No executor overhead

### Embassy HAL (No Executor)

Embassy crates are used for **hardware abstraction only**:
- `embassy-stm32` - STM32 peripheral drivers
- `embassy-net` - Network stack
- `embassy-time` - Time management
- `embassy-executor` - **NOT used** (RTIC handles scheduling)

### Memory Model

- **no_std** - No standard library
- **no heap** - All allocations are static
- `heapless` - Fixed-capacity collections
- `static_cell` - Static initialization patterns
- `panic = "abort"` - No unwinding

## Configuration

### Customize probe-rs Settings

Create `Embed.local.toml` or `.embed.local.toml` in the workspace root or
board directory:

```toml
[default.general]
chip = "STM32F405RGTx"
connect_under_reset = true  # Enable if you have connection issues

[default.probe]
protocol = "Swd"
speed = 1000  # Reduce speed if you have signal integrity issues

[default.rtt]
enabled = true

[default.gdb]
enabled = false
```

### Network Configuration

Board-specific network configuration is documented in each board's
page under [docs/src/boards/](docs/src/boards/README.md).

### Build Profiles

Defined in root `Cargo.toml`:
- **dev**: Optimized for debugging (opt-level = 1)
- **release**: Optimized for size (opt-level = "s")

Both profiles use `panic = "abort"` for embedded compatibility.

### Build-time Environment Variables

| Variable | Default (debug) | Default (release) | Range | Description |
|----------|----------------|-------------------|-------|-------------|
| `SAMPLE_INTERVAL_SECS` | 5 | 60 | 1-3600 | Sensor read and MQTT publish interval |

Override at build time:

```sh
SAMPLE_INTERVAL_SECS=10 cargo build --target thumbv7em-none-eabihf
```

## Testing

### Unit Tests

Platform-agnostic logic lives in the `core/` crate (`iot-core`)
and is tested on the host.  Because `.cargo/config.toml` sets
the workspace default target to `thumbv7em-none-eabihf` (bare-metal
ARM), a plain `cargo test` cannot link the test harness.  Use the
cargo alias or mise task instead:

```bash
# Recommended-- auto-detects host triple
mise run test

# Or explicitly (replace target with your host triple)
cargo test --workspace --target x86_64-unknown-linux-gnu
```

The mise task auto-detects the host triple via `rustc -vV`, so it
works on any platform (Linux x86_64/aarch64, macOS Intel/Apple
Silicon, etc.).  The explicit command is useful when mise is not
available.

Pass extra arguments after `--`:

```bash
mise run test -- --nocapture       # Print test stdout
mise run test -- --test-threads 1  # Single-threaded
mise run test -- -p iot-core       # Restrict to one crate
```

### On-Device Testing

On-device tests use
[`embedded-test`](https://crates.io/crates/embedded-test) over `probe-rs`
(`defmt-test` is deprecated and is not used).  Each test layer has its own
mise task; tier selection is a separate task name, not a flag:

```bash
mise run test:host                  # Layer 1: host unit tests
mise run test:device                # Layer 2: on-device bring-up tests
mise run test:smoke                 # Layer 4: standard smoke test (~3 min)
mise run test:smoke-extended        # Layer 4: extended smoke test (~5 min)
mise run test:smoke-full            # Layer 4: full smoke test (~13 min)
mise run test:integration           # Layer 5: host + device + smoke pipeline
mise run test:integration-extended  # Layer 5: pipeline with extended smoke
mise run test:integration-full      # Layer 5: pipeline with full smoke
```

See [`docs/src/development/testing.md`](docs/src/development/testing.md) for
the full five-layer test pyramid and tier detail.

### TLS Certificates

The test infrastructure uses a shared TLS certificate hierarchy.
Certificates are generated by `test/scripts/gen-tls-cert.sh` and
stored in gitignored directories:

```
.local/certs/ca/root.crt         # Root CA (shared across services)
.local/certs/server/broker.crt   # Mosquitto server cert
.local/certs/client/device.crt   # Device client cert
.local/private/                  # Private keys (never committed)
```

Generate certs manually or let mise handle it automatically:

```bash
mise run tls:ca              # Generate root CA
mise run tls:server broker   # Generate broker server cert
mise run tls:client device   # Generate device client cert
```

Certs are NOOP if they already exist-- safe to run repeatedly.

### MQTT Test Broker

Start the local Mosquitto MQTT broker with TLS support:

```bash
mise run broker:start    # Build and start container
mise run broker:stop     # Stop and remove
mise run broker:logs     # View live logs
```

See [`test/broker/README.md`](test/broker/README.md) for details.

## Troubleshooting

### Build Errors

**Error: `can't find crate for 'core'`**
```bash
# If using mise (handles this automatically):
mise install

# Or manually:
rustup target add thumbv7em-none-eabihf
rustup component add rust-src
```

**Error: `panic_handler function required`**
- Ensure `panic-probe` is in dependencies
- Verify `panic = "abort"` is set in profiles
- Make sure rust-src component is installed

**Linker errors**
- Check `memory.x` for correct memory regions
- Verify `.cargo/config.toml` linker flags

### probe-rs Connection Issues

**Probe not found**
```bash
# List available probes
probe-rs list

# If your probe needs specific permissions (Linux), you may need udev rules
# Check probe-rs documentation for your specific probe
```

**Target not found**
```bash
# List supported chips
probe-rs chip list | grep STM32F405

# Use exact chip name in Embed.toml
```

### Alternative Flashing Methods

Probe-free fallbacks (e.g., the Feather's DFU bootloader) are
documented in each board's page under
[docs/src/boards/](docs/src/boards/README.md).

## Documentation

Build and view the project documentation:

```bash
# Using mise (recommended)
mise run docs

# Or manually
cd docs
mdbook serve --open
```

View online at: https://lamawithonel.github.io/iot-playground

## Contributing

See `AGENTS.md` for:
- Architecture constraints
- Code style guidelines
- Testing requirements
- Development best practices

`AGENTS.md` files are lazy-loaded per-directory indices; each one
describes only its own directory, and detailed rules live in
`.agents/rules/`.

## License

Apache-2.0.  Copyright Lucas Yamanishi.  See [`LICENSE`](LICENSE) for the
full text.

## Resources

- [RTIC Book](https://rtic.rs/)
- [Embassy Project](https://embassy.dev/)
- [probe-rs Documentation](https://probe.rs/)
- [Embedded Rust Book](https://rust-embedded.github.io/book/)
- [STM32F4 Reference Manual](https://www.st.com/resource/en/reference_manual/dm00031020.pdf)
- [defmt Documentation](https://defmt.ferrous-systems.com/)

