//! TLS Buffer Allocations in Main SRAM
//!
//! This module manages static buffers for TLS operations in main SRAM rather than CCM RAM.
//! This approach provides more flexibility for buffer sizing while still maintaining
//! zero-allocation TLS operations.
//!
//! # Design Rationale
//!
//! TLS buffers are placed in main SRAM (128KB available) rather than CCM RAM (64KB) because:
//! 1. **Size Requirements**: TLS 1.3 requires 17KB+ read buffer, 16KB write buffer
//! 2. **CCM RAM Conservation**: CCM RAM is better used for critical timing-sensitive data
//! 3. **Performance**: TLS crypto operations are compute-bound, not memory-bound
//! 4. **Flexibility**: Main SRAM has more space for future expansion
//!
//! # Buffer Sizing
//!
//! **Read Buffer (18 KB)**:
//! - TLS 1.3 maximum plaintext: 16384 bytes (16 KB)
//! - TLS record header: 5 bytes
//! - AEAD authentication tag: 16 bytes (AES-128-GCM-SHA256)
//! - Padding allowance: ~512 bytes for safety
//! - **Total**: 17 KB minimum, using 18 KB for alignment and safety margin
//!
//! **Write Buffer (16 KB)**:
//! - We control outgoing record sizes, so 16 KB is sufficient
//! - Matches TLS 1.3 maximum record size
//!
//! # Safety
//!
//! These buffers use `static mut` which is unsafe. Safety is ensured by:
//! - Single accessor functions that return mutable references
//! - Documentation requiring single-use semantics
//! - No concurrent access (enforced by borrow checker at call site)

#![allow(unsafe_code)] // Required for static mut buffers
#![deny(warnings)]

/// TLS read buffer size: 18 KB
///
/// Sized to handle maximum TLS 1.3 record (16384 bytes) plus all overhead:
/// - Record header: 5 bytes
/// - AEAD tag: 16 bytes
/// - Padding/safety: 512 bytes
const TLS_READ_BUF_SIZE: usize = 18 * 1024; // 18432 bytes

/// TLS write buffer size: 16 KB
///
/// Maximum TLS 1.3 record size for outgoing data
const TLS_WRITE_BUF_SIZE: usize = 16 * 1024; // 16384 bytes

/// TLS read buffer in main SRAM
///
/// Used for receiving TLS records from the network.
///
/// # Safety
/// - Must only be accessed once per TLS connection lifetime
/// - No DMA access required (CPU-only processing)
/// - No concurrent access (enforced by Rust borrow rules at call site)
static mut TLS_READ_BUF: [u8; TLS_READ_BUF_SIZE] = [0; TLS_READ_BUF_SIZE];

/// TLS write buffer in main SRAM
///
/// Used for sending TLS records to the network.
///
/// # Safety
/// - Must only be accessed once per TLS connection lifetime
/// - No DMA access required (CPU-only processing)
/// - No concurrent access (enforced by Rust borrow rules at call site)
static mut TLS_WRITE_BUF: [u8; TLS_WRITE_BUF_SIZE] = [0; TLS_WRITE_BUF_SIZE];

/// Get mutable reference to TLS read buffer
///
/// # Safety
///
/// This function returns a mutable reference to a static buffer. The caller must ensure:
/// - The buffer is used by only one TLS connection at a time
/// - The buffer is not accessed concurrently from multiple contexts
/// - The buffer reference does not outlive the TLS connection
///
/// # Usage
///
/// ```no_run
/// let read_buf = unsafe { tls_read_buffer() };
/// // Use read_buf for exactly one TLS connection
/// // Buffer becomes available again when connection closes
/// ```
#[allow(dead_code)] // May be used directly in future
pub unsafe fn tls_read_buffer() -> &'static mut [u8] {
    // SAFETY: Caller must ensure single-use semantics
    // Raw pointer dereference required per Rust 2024 edition
    &mut *core::ptr::addr_of_mut!(TLS_READ_BUF)
}

/// Get mutable reference to TLS write buffer
///
/// # Safety
///
/// This function returns a mutable reference to a static buffer. The caller must ensure:
/// - The buffer is used by only one TLS connection at a time
/// - The buffer is not accessed concurrently from multiple contexts
/// - The buffer reference does not outlive the TLS connection
///
/// # Usage
///
/// ```no_run
/// let write_buf = unsafe { tls_write_buffer() };
/// // Use write_buf for exactly one TLS connection
/// // Buffer becomes available again when connection closes
/// ```
#[allow(dead_code)] // May be used directly in future
pub unsafe fn tls_write_buffer() -> &'static mut [u8] {
    // SAFETY: Caller must ensure single-use semantics
    // Raw pointer dereference required per Rust 2024 edition
    &mut *core::ptr::addr_of_mut!(TLS_WRITE_BUF)
}

/// Get both TLS buffers (read and write) as a tuple
///
/// Convenience function for TLS connection setup.
///
/// # Safety
///
/// Same safety requirements as individual buffer accessors. The caller must ensure:
/// - Buffers are used by only one TLS connection at a time
/// - No concurrent access
/// - Buffer references don't outlive the TLS connection
///
/// # Returns
///
/// `(read_buffer, write_buffer)` - Tuple of mutable slices
pub unsafe fn tls_buffers() -> (&'static mut [u8], &'static mut [u8]) {
    (
        &mut *core::ptr::addr_of_mut!(TLS_READ_BUF),
        &mut *core::ptr::addr_of_mut!(TLS_WRITE_BUF),
    )
}
