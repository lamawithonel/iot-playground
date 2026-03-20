//! CCM RAM Memory Allocations Module
//!
//! This module is the **ONLY** place in the codebase where CCM RAM (Core-Coupled Memory)
//! section attributes are used. All `#[link_section = ".ccmram"]` attributes must live here.
//!
//! # Why This Module Exists
//!
//! The `#[link_section]` attribute is considered unsafe because it:
//! 1. Modifies linker behavior without compile-time verification
//! 2. Can place data at memory locations that may violate safety assumptions
//! 3. Requires understanding of hardware memory architecture
//!
//! By isolating this unsafe code in a dedicated module, we:
//! - Enable `#![deny(unsafe_code)]` in all other modules
//! - Centralize memory placement for easy auditing
//! - Document safety requirements in one place
//! - Make it obvious what needs review when memory layout changes
//!
//! # CCM RAM Characteristics (STM32F405RG)
//!
//! - **Size**: 64 KB (0x1000_0000 - 0x1000_FFFF)
//! - **Access**: CPU only (no DMA access)
//! - **Performance**: Zero wait states
//! - **Use Cases**: Critical variables, frequently accessed data, stack-local buffers
//!
//! # Memory Budget (from design_goals_condensed.md)
//!
//! ```text
//! CCM RAM (64KB) - CPU-only, zero wait states:
//! ├─ TLS buffers:               34KB
//! │   ├─ TLS_READ_BUF:  18KB (max TLS 1.3 record + overhead)
//! │   └─ TLS_WRITE_BUF: 16KB (max TLS 1.3 record)
//! ├─ Critical variables:        <1KB
//! │   ├─ TIME_SYNCED flag:   1 byte
//! │   ├─ BASE_UNIX_SECS:    4 bytes
//! │   ├─ BASE_UNIX_MICROS:  4 bytes
//! │   └─ BASE_MONO_MICROS:  4 bytes
//! └─ Reserved for future use:   ~30KB
//!     └─ Potential uses: hot-path variables, timing-critical buffers
//! ```
//!
//! # Current Allocations
//!
//! - **TLS_READ_BUF**: 18 KB
//!   - Maximum TLS 1.3 record (16384 bytes) plus record header, AEAD tag,
//!     and safety margin
//!   - CPU-only access (no DMA); placed in CCM RAM to free main SRAM for
//!     stack
//! - **TLS_WRITE_BUF**: 16 KB
//!   - Maximum TLS 1.3 record size for outgoing data
//!   - CPU-only access (no DMA)
//! - **TIME_SYNCED**: AtomicBool (~1 byte)
//!   - Tracks whether RTC has been synchronized with NTP
//!   - Placed in CCM RAM for zero-wait-state access
//!   - Time itself is stored in hardware RTC peripheral
//!
//! # Safety Requirements
//!
//! When adding new CCM RAM allocations:
//! 1. **Verify total usage < 64 KB**
//! 2. **No DMA**: Data must not be used with DMA peripherals
//! 3. **Alignment**: Respect Rust's alignment requirements
//! 4. **Static lifetime**: Only `static` items (not stack allocations)
//! 5. **Document**: Update this module's header with new allocations
//!
//! # Example Usage
//!
//! ```no_run
//! // From time.rs:
//! #[link_section = ".ccmram"]
//! static TIME_SYNCED: AtomicBool = AtomicBool::new(false);
//! ```
//!
//! # References
//!
//! - STM32F405RG Reference Manual, Section 2.3 "Memory Map"
//! - [Rust Unsafe Code Guidelines](https://rust-lang.github.io/unsafe-code-guidelines/)

// NOTE: This is the ONLY module in the codebase that should NOT have
// #![deny(unsafe_code)] because it intentionally uses linker sections.
// The #[link_section] attribute is considered unsafe.

#![allow(unsafe_code)]
#![deny(warnings)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// System time synchronization status in CCM RAM
#[allow(dead_code)]
#[link_section = ".ccmram"]
pub static TIME_SYNCED: AtomicBool = AtomicBool::new(false);

// ============================================================================
// CLOCK_REALTIME IMPLEMENTATION (Linux-style wall-clock time)
// ============================================================================
//
// This implements a high-precision wall-clock time system similar to Linux:
// - CLOCK_MONOTONIC: TIM2 running at 1MHz (microsecond precision)
// - CLOCK_REALTIME: CLOCK_MONOTONIC + Unix time offset from NTP calibration
// - RTC: Backup only (not used for primary timekeeping)
//
// When NTP sync occurs, we capture:
// 1. Unix time from NTP (seconds and microseconds parts, stored separately)
// 2. Monotonic timer value at that exact moment (microseconds)
//
// Note: Using 32-bit AtomicU32 instead of AtomicU64 since ARMv7-M (Cortex-M4)
// doesn't have native 64-bit atomics. This limits us but is workable:
// - Seconds stored as u32 (wraps in 2106, acceptable for embedded systems)
// - Microseconds within second stored separately (0-999999)
// - Monotonic ticks in microseconds (wraps after ~71.6 minutes, but handled via wrapping arithmetic)

/// Base Unix time in seconds at calibration
///
/// Set during NTP synchronization to the Unix epoch seconds
/// at the moment of calibration. Zero means not yet calibrated.
/// Using u32 limits us to year 2106 (acceptable for embedded).
#[link_section = ".ccmram"]
static BASE_UNIX_SECS: AtomicU32 = AtomicU32::new(0);

/// Base microseconds within second at calibration
///  
/// The fractional part of Unix time (0-999999 microseconds)
/// captured at calibration moment.
#[link_section = ".ccmram"]
static BASE_UNIX_MICROS: AtomicU32 = AtomicU32::new(0);

/// Base monotonic timer value at calibration (microseconds)
///
/// Set during NTP synchronization to the monotonic timer ticks (microseconds)
/// at the moment of calibration. This is captured at the same instant as BASE_UNIX_SECS/MICROS.
/// Using u32 for microseconds means it wraps every ~71.6 minutes, but wrapping_sub handles this correctly.
#[link_section = ".ccmram"]
static BASE_MONO_MICROS: AtomicU32 = AtomicU32::new(0);

/// Calibrate the wall-clock time system
///
/// Called after successful NTP synchronization with the Unix time and
/// the monotonic timer value captured at the same instant.
///
/// # Arguments
/// * `unix_secs` - Unix epoch time in seconds from NTP
/// * `unix_micros` - Microseconds within the second (0-999999)
/// * `mono_micros` - Monotonic timer ticks in microseconds at calibration moment
#[allow(dead_code)]
pub fn calibrate_wallclock(unix_secs: u32, unix_micros: u32, mono_micros: u32) {
    BASE_UNIX_SECS.store(unix_secs, Ordering::Release);
    BASE_UNIX_MICROS.store(unix_micros, Ordering::Release);
    BASE_MONO_MICROS.store(mono_micros, Ordering::Release);
    TIME_SYNCED.store(true, Ordering::Release);
}

/// Get current Unix time in seconds and microseconds
///
/// Computes CLOCK_REALTIME as: base_unix + (current_mono - base_mono)
///
/// Returns (0, 0) if not yet calibrated (TIME_SYNCED == false).
///
/// # Arguments
/// * `current_mono_micros` - Current monotonic timer value in microseconds
///
/// # Returns
/// Tuple of (unix_seconds, microseconds) or (0, 0) if not calibrated
#[allow(dead_code)]
pub fn now_unix_time(current_mono_micros: u32) -> (u32, u32) {
    if !TIME_SYNCED.load(Ordering::Acquire) {
        return (0, 0); // Not yet calibrated
    }

    let base_secs = BASE_UNIX_SECS.load(Ordering::Acquire);
    let base_micros = BASE_UNIX_MICROS.load(Ordering::Acquire);
    let base_mono = BASE_MONO_MICROS.load(Ordering::Acquire);

    // Compute elapsed time since calibration (handling wrap-around)
    let elapsed_micros = current_mono_micros.wrapping_sub(base_mono);

    // Convert elapsed microseconds to seconds and remaining microseconds
    let elapsed_secs = elapsed_micros / 1_000_000;
    let elapsed_micros_remainder = elapsed_micros % 1_000_000;

    // Add to base time
    let total_micros = base_micros + elapsed_micros_remainder;
    let micros_overflow = total_micros / 1_000_000;
    let final_micros = total_micros % 1_000_000;

    let final_secs = base_secs
        .wrapping_add(elapsed_secs)
        .wrapping_add(micros_overflow);

    (final_secs, final_micros)
}

/// Check if wall-clock time is calibrated
#[allow(dead_code)]
pub fn is_wallclock_calibrated() -> bool {
    TIME_SYNCED.load(Ordering::Acquire)
}

// ============================================================================
// TLS BUFFERS (34 KB total)
// ============================================================================
//
// TLS read/write buffers are placed in CCM RAM because:
// 1. They are CPU-only (no DMA access needed)
// 2. Placing them here frees ~34 KB of main SRAM for the stack
// 3. CCM RAM has zero wait states, beneficial for TLS crypto
//
// These buffers were originally in main SRAM but moved here to resolve
// stack overflows during TLS handshake (the embedded-tls handshake
// state machine uses significant stack for ECDHE and HKDF operations).

/// TLS read buffer size: 18 KB
///
/// Sized to handle maximum TLS 1.3 record (16384 bytes) plus all
/// overhead:
/// - Record header: 5 bytes
/// - AEAD tag: 16 bytes
/// - Padding/safety: 512 bytes
const TLS_READ_BUF_SIZE: usize = 18 * 1024; // 18432 bytes

/// TLS write buffer size: 16 KB
///
/// Maximum TLS 1.3 record size for outgoing data
const TLS_WRITE_BUF_SIZE: usize = 16 * 1024; // 16384 bytes

/// TLS read buffer in CCM RAM
///
/// Used for receiving TLS records from the network.
///
/// # Safety
/// - Must only be accessed once per TLS connection lifetime
/// - No DMA access (CCM RAM is CPU-only)
/// - No concurrent access (enforced by Rust borrow rules at call site)
#[link_section = ".ccmram"]
static mut TLS_READ_BUF: [u8; TLS_READ_BUF_SIZE] = [0; TLS_READ_BUF_SIZE];

/// TLS write buffer in CCM RAM
///
/// Used for sending TLS records to the network.
///
/// # Safety
/// - Must only be accessed once per TLS connection lifetime
/// - No DMA access (CCM RAM is CPU-only)
/// - No concurrent access (enforced by Rust borrow rules at call site)
#[link_section = ".ccmram"]
static mut TLS_WRITE_BUF: [u8; TLS_WRITE_BUF_SIZE] = [0; TLS_WRITE_BUF_SIZE];

/// Get both TLS buffers (read and write) from CCM RAM
///
/// # Safety
///
/// The caller must ensure:
/// - Buffers are used by only one TLS connection at a time
/// - No concurrent access
/// - Buffer references don't outlive the TLS connection
///
/// # Returns
///
/// `(read_buffer, write_buffer)` — Tuple of mutable slices
pub unsafe fn tls_buffers() -> (&'static mut [u8], &'static mut [u8]) {
    // SAFETY: Returns exclusive `&'static mut` references to
    // `static mut` buffers in CCM RAM.  Sound because:
    // - Only the TLS/network client logic obtains these buffers
    // - Higher-level code ensures at most one TLS connection
    //   uses them at a time (no concurrent access or aliasing)
    // - Buffers are `'static` and outlive any TLS connection
    (
        &mut *core::ptr::addr_of_mut!(TLS_READ_BUF),
        &mut *core::ptr::addr_of_mut!(TLS_WRITE_BUF),
    )
}

// ============================================================================
// FUTURE CCM RAM ALLOCATIONS GO BELOW THIS LINE
// ============================================================================
//
// After TLS buffers, approximately 30 KB of CCM RAM remains available
// for timing-critical variables and buffers.
