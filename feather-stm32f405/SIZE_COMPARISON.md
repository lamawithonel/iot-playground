# Binary Size Comparison: Chrono vs Custom Time Functions

## Purpose
Compare the binary size impact of using the `chrono` crate (no_std) versus custom time conversion functions.

## What's Being Compared

### Custom Implementation (current)
- `unix_to_datetime()` - ~100 lines of custom code
- `datetime_to_unix()` - ~40 lines of custom code  
- `is_leap_year()` - ~2 lines
- **Total**: ~142 lines of handwritten date/time math

### Chrono Implementation (test)
- Uses `chrono` crate with `default-features = false` (no_std compatible)
- Replaces all custom conversion logic with chrono's battle-tested implementations
- `unix_to_datetime_chrono()` using `NaiveDateTime::from_timestamp_opt()`
- `datetime_to_unix_chrono()` using `.timestamp()`

## How to Measure

### 1. Baseline (Custom Implementation)
```bash
cd feather-stm32f405
git checkout feature/sntp-client
cargo build --release
arm-none-eabi-size target/thumbv7em-none-eabihf/release/feather-stm32f405
```

### 2. With Chrono
```bash
git checkout test/chrono-size-comparison

# Modify src/time.rs to use chrono functions instead of custom ones
# Replace unix_to_datetime with unix_to_datetime_chrono
# Replace datetime_to_unix with datetime_to_unix_chrono

cargo build --release
arm-none-eabi-size target/thumbv7em-none-eabihf/release/feather-stm32f405
```

### 3. Compare Results
Look at the `.text` section size difference:
- `.text` = code size
- `.data` = initialized data
- `.bss` = uninitialized data

## Expected Trade-offs

### Chrono Advantages
- ✅ Battle-tested, handles all edge cases correctly
- ✅ Handles leap seconds, complex calendar rules
- ✅ Less maintenance burden
- ✅ Well-documented API

### Custom Implementation Advantages  
- ✅ Smaller binary (likely)
- ✅ No external dependencies
- ✅ Simpler for basic use case (just NTP sync)
- ✅ Faster compile times

## Size Estimation

Based on Rust community experience:
- Chrono (no_std, minimal features): typically adds **3-15 KB** to binary
- Custom implementation: **~500 bytes - 2 KB** depending on optimization

**Expected difference: ~5-10 KB**

## Recommendation Criteria

**Use Custom** if:
- Every kilobyte matters (< 128KB flash)
- Only need basic Unix ↔ DateTime conversion
- Simple calendar math is sufficient

**Use Chrono** if:
- Need complex timezone support later
- Want industry-standard date/time handling
- Binary size isn't critical concern (> 256KB flash available)
- Planning to add more time-based features

## STM32F405RG Flash Capacity
- **1 MB (1024 KB) total flash**
- Current baseline: TBD
- Chrono overhead: ~5-10 KB estimated
- **Impact**: < 1% of flash if estimation is correct

## Detailed Analysis Tool

For deeper analysis, use `cargo bloat`:
```bash
cargo install cargo-bloat

# See what functions take up space
cargo bloat --release -n 50

# Compare crate-level impact  
cargo bloat --release --crates
```

Look specifically for:
- `chrono::` symbols vs custom function sizes
- Total crate size contribution
