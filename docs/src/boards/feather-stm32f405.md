# Adafruit Feather STM32F405

Board profile for the Adafruit Feather STM32F405 Express (Tier 2:
Connected device with TLS/MQTT).  The pin map lives in
[System Requirements, section 3](../system_requirements.md); this
page owns the board-specific build, flash, debug, and configuration
detail.

## Flash Only

```bash
# Flash the release build
cargo flash --release --chip STM32F405RGTx
```

## Debugging

### View Logs with RTT

The firmware uses `defmt` for efficient logging over RTT (Real-Time
Transfer):

```bash
# Build, flash, and view logs
cargo embed --release

# Or just attach to an already-running device
probe-rs attach --chip STM32F405RGTx
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

## Network Configuration

Edit `boards/feather-stm32f405/src/network/config.rs` for:

- MQTT broker settings
- TLS certificates
- Network timeouts

## Alternative Flashing Methods

If you don't have a debug probe, the Feather STM32F405 has a built-in
DFU bootloader:

1. **Enter DFU mode:**
   - Press and hold BOOT0 button
   - Press and release RESET button
   - Release BOOT0 button

2. **Flash via DFU** (requires dfu-util installed separately):
   ```bash
   cargo build --release --target thumbv7em-none-eabihf
   # Then use dfu-util (installation varies by OS)
   ```

**Note:** probe-rs is the recommended workflow.  DFU mode is a
fallback for boards without debug probe access.
