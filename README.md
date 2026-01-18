# IoT Playground

An embedded Rust IoT framework for STM32 and Microchip ATSAM microcontrollers, designed for financial services applications requiring real-time guarantees and security.

## Overview

This project provides a multi-device capable embedded firmware framework using:
- **RTIC 2.x** for interrupt-driven task scheduling
- **Embassy HAL** for hardware abstraction (without executor)
- **embedded-tls** for secure communications
- **MQTT v5.0** for messaging
- **no_std** environment with zero heap allocation

## Supported Boards

- **Adafruit Feather STM32F405** (Tier 2: Connected device with TLS/MQTT)
- BBC micro:bit v2 (planned)
- STM32F3 Discovery (planned)

## Prerequisites

### Required Tools

1. **Rust toolchain** (stable):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **ARM target support**:
   ```bash
   rustup target add thumbv7em-none-eabihf  # For Cortex-M4F/M7F (STM32F405)
   rustup component add rust-src            # Required for building no_std targets
   rustup component add llvm-tools-preview  # For cargo-binutils
   ```

3. **probe-rs tools** (for building, flashing, and debugging):
   ```bash
   cargo install probe-rs-tools --locked
   cargo install cargo-embed --locked
   cargo install cargo-flash --locked
   ```

### Optional Tools

- **cargo-binutils** (for binary inspection):
  ```bash
  cargo install cargo-binutils
  ```

## Building and Flashing

### Quick Start (from workspace root)

Build and flash firmware using `cargo run` or `cargo embed` with board presets:

```bash
# Using cargo run with PROBE_RS_CONFIG_PRESET (recommended)
cargo run --release                            # Uses default (feather)
PROBE_RS_CONFIG_PRESET=feather cargo run --release     # Feather STM32F405
PROBE_RS_CONFIG_PRESET=microbit cargo run --release    # BBC micro:bit v2
PROBE_RS_CONFIG_PRESET=stm32f3 cargo run --release     # STM32F3 Discovery

# Or use cargo embed with --chip flag
cargo embed --release                          # Uses default (feather)
cargo embed --chip feather --release           # Feather STM32F405
cargo embed --chip microbit --release          # BBC micro:bit v2
cargo embed --chip stm32f3 --release           # STM32F3 Discovery
```

This will:
1. Build the firmware for the selected board
2. Use the correct linker configuration automatically
3. Flash it to your connected debug probe (with board-specific settings)
4. Attach to RTT for live log viewing

**Note:** The root `.cargo/config.toml` configures `probe-rs run` as the generic runner, and chip selection is handled via `Embed.toml` presets or the `PROBE_RS_CONFIG_PRESET` environment variable.

### Board Selection

You can select boards in two ways (in order of precedence):

1. **PROBE_RS_CONFIG_PRESET environment variable** (recommended):
   ```bash
   PROBE_RS_CONFIG_PRESET=microbit cargo run --release
   
   # Or set it for multiple commands:
   export PROBE_RS_CONFIG_PRESET=microbit
   cargo run --release
   cargo build --release
   ```

2. **cargo embed --chip flag**:
   ```bash
   cargo embed --chip microbit --release
   ```

3. **Default**: If nothing is specified, `feather` (Feather STM32F405) preset is used

**Available board presets:**
- `feather` - Adafruit Feather STM32F405 (default)
- `microbit` - BBC micro:bit v2 (nRF52833)
- `stm32f3` - STM32F3 Discovery (STM32F303VC)

### Build Only

```bash
# Build from workspace root
cargo build --release

# Build with specific board preset
PROBE_RS_CONFIG_PRESET=microbit cargo build --release

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

```bash
# Flash the release build
cargo flash --release --chip STM32F405RGTx
```

## Debugging

### View Logs with RTT

The firmware uses `defmt` for efficient logging over RTT (Real-Time Transfer):

```bash
# Build, flash, and view logs
cargo embed --release

# Or just attach to an already-running device
probe-rs attach --chip STM32F405RGTx
```

### Set Log Level

```bash
# Set via environment variable
export DEFMT_LOG=debug  # or: trace, info, warn, error

# Then run cargo embed
cargo embed --release
```

### Interactive Debugging with probe-rs

```bash
# Start GDB server
probe-rs gdb --chip STM32F405RGTx target/thumbv7em-none-eabihf/release/feather-stm32f405

# In another terminal, connect with GDB
# (Requires arm-none-eabi-gdb or gdb-multiarch installed separately)
arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/feather-stm32f405
(gdb) target remote :1337
(gdb) continue
```

## Project Structure

```
iot-playground/
├── Cargo.toml              # Workspace root with shared dependencies & default-members
├── Embed.toml              # probe-rs presets for all boards (feather, microbit, stm32f3)
├── .cargo/config.toml      # Root config with generic probe-rs runner
├── AGENTS.md               # Project architecture and constraints
├── boards/                 # Board profiles (specific chip + peripherals + applications)
│   └── feather-stm32f405/  # Example: Feather STM32F405 board profile
│       ├── Embed.toml      # Board-specific probe-rs config (optional)
│       ├── src/            # Board-specific firmware code
│       └── memory.x        # Memory layout for this board
├── core/                   # Platform-agnostic business logic (skeleton)
├── hal-abstractions/       # Hardware abstraction traits (skeleton)
├── apps/                   # Application binaries (future)
└── docs/                   # Documentation (mdBook)
```

### Configuration Files

**Workspace-level:**
- `.cargo/config.toml`: Root configuration with generic `probe-rs run` runner and common linker flags
- `Cargo.toml`: Sets `default-members = ["boards/feather-stm32f405"]` for cargo commands
- `Embed.toml`: Defines board presets (feather, microbit, stm32f3) for probe-rs configuration

**Board-level:**
- `boards/*/Embed.toml`: Optional board-specific probe-rs config overrides
- `boards/*/memory.x`: Memory layout for the specific chip
- `boards/*/src/`: Board-specific application code and configuration

### Board Profiles vs. Boards

A **board profile** is a specific configuration combining:
- A board type (e.g., Feather STM32F405)
- Peripheral components (e.g., Ethernet chip, sensors)
- Application purpose (e.g., sensor gateway, PTP server)

Examples of board profiles in `boards/`:
- `feather-eth-sensor/` - Feather STM32F405 + Ethernet + SEN66 sensor + CAN gateway
- `feather-ptp-server/` - Feather STM32F405 + Ethernet + GPS clock (IEEE 1588 PTP)
- `feather-m4-can/` - Feather M4 CAN Express + sensors (CAN-only device)

Each profile shares common code (like network stack) but has unique configuration and glue code.

## Development Workflow

1. **Make changes** to the source code in `boards/feather-stm32f405/src/`
2. **Build, flash, and test**:
   ```bash
   cargo run --release
   # or with a specific board preset:
   PROBE_RS_CONFIG_PRESET=microbit cargo run --release
   ```
3. **View logs** in real-time via RTT output

### Working with a Specific Board

To work on a specific board profile, you can either:

1. **Use environment variable** (recommended):
   ```bash
   export PROBE_RS_CONFIG_PRESET=microbit
   cargo run --release
   cargo build --release
   ```

2. **Change to board directory**:
   ```bash
   cd boards/feather-stm32f405
   cargo run --release
   ```

## Architecture

### RTIC-First Design

This framework uses **RTIC 2.x** (Real-Time Interrupt-driven Concurrency) for task scheduling:
- Hardware interrupts trigger tasks
- `WFI` (Wait For Interrupt) for power efficiency
- Zero-cost abstractions for real-time guarantees
- No executor overhead

### Embassy HAL (No Executor)

Embassy crates are used for **hardware abstraction only**:
- ✅ `embassy-stm32` - STM32 peripheral drivers
- ✅ `embassy-net` - Network stack
- ✅ `embassy-time` - Time management
- ❌ `embassy-executor` - **NOT used** (RTIC handles scheduling)

### Memory Model

- **no_std** - No standard library
- **no heap** - All allocations are static
- `heapless` - Fixed-capacity collections
- `static_cell` - Static initialization patterns
- `panic = "abort"` - No unwinding

## Configuration

### Customize probe-rs Settings

Create `Embed.local.toml` or `.embed.local.toml` in the workspace root or board directory:

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

Edit `boards/feather-stm32f405/src/network/config.rs` for:
- MQTT broker settings
- TLS certificates
- Network timeouts

### Build Profiles

Defined in root `Cargo.toml`:
- **dev**: Optimized for debugging (opt-level = 1)
- **release**: Optimized for size (opt-level = "s")

Both profiles use `panic = "abort"` for embedded compatibility.

## Testing

### On-Device Testing

```bash
# Using defmt-test (when available)
cargo test --target thumbv7em-none-eabihf
```

### Unit Tests (for core/ crate)

```bash
cargo test --manifest-path core/Cargo.toml
```

## Troubleshooting

### Build Errors

**Error: `can't find crate for 'core'`**
```bash
# Install the target and rust-src:
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

If you don't have a debug probe, the Feather STM32F405 has a built-in DFU bootloader:

1. **Enter DFU mode:**
   - Press and hold BOOT0 button
   - Press and release RESET button
   - Release BOOT0 button

2. **Flash via DFU** (requires dfu-util installed separately):
   ```bash
   cargo build --release --target thumbv7em-none-eabihf
   # Then use dfu-util (installation varies by OS)
   ```

**Note:** probe-rs is the recommended workflow. DFU mode is a fallback for boards without debug probe access.

## Documentation

Build and view the project documentation:

```bash
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

## License

MIT OR Apache-2.0

## Resources

- [RTIC Book](https://rtic.rs/)
- [Embassy Project](https://embassy.dev/)
- [probe-rs Documentation](https://probe.rs/)
- [Embedded Rust Book](https://rust-embedded.github.io/book/)
- [STM32F4 Reference Manual](https://www.st.com/resource/en/reference_manual/dm00031020.pdf)
- [defmt Documentation](https://defmt.ferrous-systems.com/)

