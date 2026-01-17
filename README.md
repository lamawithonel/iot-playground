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

1. **Rust toolchain** (stable or nightly):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **ARM target support**:
   ```bash
   rustup target add thumbv7em-none-eabihf  # For Cortex-M4F/M7F (STM32F405)
   rustup component add rust-src            # Required for building no_std targets
   ```

3. **cargo-binutils** (for binary inspection):
   ```bash
   cargo install cargo-binutils
   rustup component add llvm-tools-preview
   ```

4. **probe-rs** or **dfu-util** (for flashing):
   ```bash
   # Option 1: probe-rs (recommended for debugging)
   cargo install probe-rs --features cli
   
   # Option 2: dfu-util (for STM32 bootloader)
   # On Ubuntu/Debian:
   sudo apt-get install dfu-util
   
   # On macOS:
   brew install dfu-util
   ```

### Optional Tools

- **OpenOCD** (for debugging with GDB):
  ```bash
  # Ubuntu/Debian:
  sudo apt-get install openocd
  
  # macOS:
  brew install openocd
  ```

- **GDB for ARM**:
  ```bash
  # Ubuntu/Debian:
  sudo apt-get install gdb-multiarch
  
  # macOS:
  brew install arm-none-eabi-gdb
  ```

## Building

This is a Cargo workspace with multiple crates. The main board support packages are in the `boards/` directory.

### Build for Feather STM32F405

```bash
# Development build (optimized for debugging)
cargo build --manifest-path boards/feather-stm32f405/Cargo.toml

# Release build (optimized for size)
cargo build --manifest-path boards/feather-stm32f405/Cargo.toml --release

# Check without building (fast)
cargo check --manifest-path boards/feather-stm32f405/Cargo.toml
```

The target architecture is automatically selected via `.cargo/config.toml` (defaults to `thumbv7em-none-eabihf`).

### Binary Output

Built binaries are located at:
```
target/thumbv7em-none-eabihf/debug/feather-stm32f405
target/thumbv7em-none-eabihf/release/feather-stm32f405
```

Generate a flashable binary:
```bash
cargo objcopy --manifest-path boards/feather-stm32f405/Cargo.toml --release -- -O binary feather-stm32f405.bin
```

## Flashing

### Option 1: DFU Bootloader (STM32F405)

The Feather STM32F405 has a built-in DFU bootloader:

1. Connect the board via USB
2. Put the board in DFU mode:
   - Press and hold BOOT0 button
   - Press and release RESET button
   - Release BOOT0 button
3. Flash:
   ```bash
   cargo run --manifest-path boards/feather-stm32f405/Cargo.toml --release
   ```

The `.cargo/config.toml` is configured to use `dfu-util` as the runner.

### Option 2: probe-rs

If you have a debug probe (ST-Link, J-Link, etc.):

```bash
probe-rs run --chip STM32F405RGTx target/thumbv7em-none-eabihf/release/feather-stm32f405
```

### Option 3: OpenOCD + GDB

For interactive debugging:

1. Start OpenOCD (in one terminal):
   ```bash
   openocd -f interface/stlink.cfg -f target/stm32f4x.cfg
   ```

2. Connect GDB (in another terminal):
   ```bash
   gdb-multiarch target/thumbv7em-none-eabihf/debug/feather-stm32f405
   (gdb) target remote :3333
   (gdb) load
   (gdb) continue
   ```

## Debugging

### View logs via defmt

The firmware uses `defmt` for efficient logging over RTT (Real-Time Transfer):

```bash
# With probe-rs (recommended)
probe-rs run --chip STM32F405RGTx target/thumbv7em-none-eabihf/debug/feather-stm32f405

# With defmt-print
cargo install defmt-print
defmt-print target/thumbv7em-none-eabihf/debug/feather-stm32f405
```

### Set log level

```bash
# In your shell or .cargo/config.toml
export DEFMT_LOG=debug  # or: trace, info, warn, error
```

### Binary size analysis

```bash
cargo size --manifest-path boards/feather-stm32f405/Cargo.toml --release -- -A
```

## Project Structure

```
iot-playground/
├── Cargo.toml              # Workspace root with shared dependencies
├── AGENTS.md               # Project architecture and constraints
├── boards/                 # Board Support Packages (BSPs)
│   └── feather-stm32f405/  # Adafruit Feather STM32F405 board
│       ├── .cargo/         # Board-specific cargo config
│       ├── src/            # Firmware source code
│       └── memory.x        # Memory layout
├── core/                   # Platform-agnostic business logic (skeleton)
├── hal-abstractions/       # Hardware abstraction traits (skeleton)
├── apps/                   # Application binaries (future)
└── docs/                   # Documentation (mdBook)
```

## Development Workflow

1. **Make changes** to the source code in `boards/feather-stm32f405/src/`
2. **Check compilation**:
   ```bash
   cargo check --manifest-path boards/feather-stm32f405/Cargo.toml
   ```
3. **Build and test**:
   ```bash
   cargo build --manifest-path boards/feather-stm32f405/Cargo.toml --release
   ```
4. **Flash to hardware**:
   ```bash
   cargo run --manifest-path boards/feather-stm32f405/Cargo.toml --release
   ```
5. **View logs** via probe-rs or defmt-print

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
cargo test --manifest-path boards/feather-stm32f405/Cargo.toml --target thumbv7em-none-eabihf
```

### Unit Tests (for core/ crate)

```bash
cargo test --manifest-path core/Cargo.toml
```

## Troubleshooting

### Build Errors

**Error: `can't find crate for 'core'`**
```bash
# Install the target:
rustup target add thumbv7em-none-eabihf
```

**Error: `panic_handler function required`**
- Ensure `panic-probe` is in dependencies
- Verify `panic = "abort"` is set in profiles

**Linker errors**
- Check `memory.x` for correct memory regions
- Verify `.cargo/config.toml` linker flags

### Flash/DFU Issues

**DFU device not found**
```bash
# List USB devices:
lsusb
# Look for: "0483:df11 STMicroelectronics STM Device in DFU Mode"

# Check DFU devices:
dfu-util -l
```

**Permission denied (Linux)**
```bash
# Add udev rules for STM32 DFU:
echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="df11", MODE="0666"' | sudo tee /etc/udev/rules.d/50-stm32-dfu.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

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
- [Embedded Rust Book](https://rust-embedded.github.io/book/)
- [STM32F4 Reference Manual](https://www.st.com/resource/en/reference_manual/dm00031020.pdf)
- [defmt Documentation](https://defmt.ferrous-systems.com/)
