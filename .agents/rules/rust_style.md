---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust Code Style

- All files MUST have `#![deny(warnings)]`
- All files MUST have `#![deny(unsafe_code)]` **unless** the file
  appears in the unsafe allowlist below
- All public items MUST have doc comments
- Use `defmt` for logging, not `log` or `println!`
- Use snake_case for all Rust files
- BSP crates named: `{board-name}` (e.g., `feather-stm32f405`)
- App crates named: `{descriptive-name}` (e.g., `mqtt-sensor-node`)

## Dependency Preference Order

1. RTIC ecosystem (`rtic`, `rtic-sync`, `rtic-monotonics`)
2. Embassy ecosystem (`embassy-stm32`, `embassy-net`, etc.)
3. `embedded-hal` ecosystem
4. Other well-maintained embedded crates

## Unsafe Code Isolation Policy

Isolating `unsafe` to specific files enables `#![deny(unsafe_code)]`
as a linter and early compiler check across the vast majority of
the codebase.

**Allowed unsafe files** (file-level `#![allow(unsafe_code)]`):

| File | Justification |
|------|---------------|
| `boards/feather-stm32f405/src/ccmram.rs` | `#[link_section]`, `static mut` for CCM RAM |

**Rules:**

- Allowed files MUST be minimal-- only the code that *requires*
  `unsafe` belongs there.  Business logic, protocol handling, and
  other safe code must live in separate modules that `use` the unsafe
  files.
- Every `unsafe` block or `unsafe fn` MUST have a `// SAFETY:`
  comment documenting why it is sound.
- When a safe module (`#![deny(unsafe_code)]`) must call an
  `unsafe fn` from an allowed file, use a **function-level**
  `#[allow(unsafe_code)]` on the narrowest possible scope:

  ```rust
  #![deny(unsafe_code)]

  #[allow(unsafe_code)]
  fn get_buffers() -> (&'static mut [u8], &'static mut [u8]) {
      // SAFETY: called once during init; no concurrent access
      unsafe { ccmram::tls_buffers() }
  }
  ```

- **Adding a new file to the allowlist requires explicit user
  approval.**  Ask before creating a new `#![allow(unsafe_code)]`
  file and update this table.
