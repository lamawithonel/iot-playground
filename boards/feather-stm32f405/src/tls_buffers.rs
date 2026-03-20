//! TLS Buffer Allocations — CCM RAM Re-export
//!
//! This module re-exports the TLS buffer accessor from `ccmram.rs`
//! for API clarity.  The underlying function is `unsafe` because it
//! hands out `&'static mut` references to `static mut` buffers;
//! callers must use an `unsafe` block and uphold the safety contract.
//!
//! All `#[link_section = ".ccmram"]` attributes and `static mut`
//! declarations live exclusively in `ccmram.rs`.
//!
//! # Buffer Location
//!
//! TLS buffers reside in CCM RAM (Core-Coupled Memory) because:
//! 1. **Stack headroom**: Moving 34 KB out of main SRAM frees that
//!    space for the stack, which is critical during TLS handshake
//!    (embedded-tls ECDHE/HKDF requires significant stack depth)
//! 2. **Zero wait states**: CCM RAM has zero wait states, which
//!    benefits the compute-heavy TLS crypto path
//! 3. **CPU-only**: TLS buffers are never accessed by DMA, so the
//!    CCM RAM restriction (no DMA) does not apply
//!
//! # Buffer Sizing
//!
//! **Read Buffer (18 KB)**:
//! - TLS 1.3 maximum plaintext: 16384 bytes (16 KB)
//! - TLS record header: 5 bytes
//! - AEAD authentication tag: 16 bytes (AES-128-GCM-SHA256)
//! - Padding allowance: ~512 bytes for safety
//! - **Total**: 17 KB minimum, using 18 KB for alignment and safety
//!   margin
//!
//! **Write Buffer (16 KB)**:
//! - We control outgoing record sizes, so 16 KB is sufficient
//! - Matches TLS 1.3 maximum record size

#![deny(unsafe_code)]
#![deny(warnings)]

/// Re-export of the CCM RAM–backed TLS buffer accessor from
/// [`ccmram`](crate::ccmram).
#[doc(inline)]
pub use crate::ccmram::tls_buffers;
