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
//! ├─ TLS read buffer:          16KB (.ccmram section)
//! ├─ TLS write buffer:          8KB (.ccmram section)
//! ├─ MQTT buffers:             16KB (.ccmram section)
//! └─ Critical variables:       24KB (.ccmram section)
//!     └─ TIME_SYNCED flag: 1 byte (in time.rs)
//! ```
//!
//! # Current Allocations
//!
//! - **TIME_SYNCED**: AtomicBool in `time.rs` (~1 byte)
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

// ============================================================================
// FUTURE CCM RAM ALLOCATIONS GO BELOW THIS LINE
// ============================================================================
//
// When adding new CCM RAM allocations:
// 1. Document size and justification
// 2. Update memory budget comment at top of file
// 3. Verify no DMA access required
// 4. Add safety documentation
//
// Example:
// /// TLS read buffer in CCM RAM (16 KB)
// ///
// /// # Safety
// /// - No DMA access required
// /// - Within CCM RAM budget
// #[link_section = ".ccmram"]
// pub static mut TLS_READ_BUFFER: [u8; 16384] = [0; 16384];
