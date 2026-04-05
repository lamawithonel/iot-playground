# Agent Instructions for iot-playground

## Project Overview

This is an embedded Rust IoT framework for STM32 and Microchip ATSAM MCUs.
It is designed for applications requiring real-time guarantees and security.

## Critical Constraints

### RTIC-First Architecture

- **REQUIRED:** Use RTIC 2.x for all task scheduling
- **REQUIRED:** Use `rtic-sync` for inter-task communication
- **FORBIDDEN:** Do not use `embassy-executor` crate
- **FORBIDDEN:** Do not use Embassy's async executor features

### Embassy Usage

- **ALLOWED:** Embassy HAL crates (`embassy-stm32`, `embassy-nrf`, etc.)
- **ALLOWED:** Embassy PAC crates
- **ALLOWED:** Embassy network crates (`embassy-net`, `embassy-net-wiznet`)
- **FORBIDDEN:** `embassy-executor`
- **PREFER:** `rtic-sync` over `embassy-sync` where possible

### Interrupt-Driven Design

- **REQUIRED:** Use WFI/Sleep between interrupt events
- **REQUIRED:** Hardware timers for periodic interrupts
- **REQUIRED:** EXTI for external peripheral interrupts
- **FORBIDDEN:** Busy-wait loops (except brief hardware delays)

### Memory Model

- **REQUIRED:** `no_std` environment
- **REQUIRED:** No heap allocation (no `alloc` crate)
- **ALLOWED:** `heapless` collections
- **ALLOWED:** `static_cell` for static allocation

## Directory Structure

```
iot-playground/
├── core/               # Platform-agnostic business logic (NO hardware deps)
├── hal-abstractions/   # Traits for hardware abstraction
├── boards/             # Board Support Packages (BSPs)
│   └── {board-name}/   # One BSP per supported board
├── apps/               # Application binaries
└── docs/               # Framework documentation (mdBook)
```

## Device Tiers

- **Tier 1 (Minimal):** ≤128KB RAM, no TLS, basic I/O only
- **Tier 2 (Connected):** ≥192KB RAM, TLS/MQTT capable, primary target
- **Tier 3 (Gateway):** ≥512KB RAM, multi-protocol, edge compute

## Rust Code Style

- All files MUST have `#![deny(warnings)]`
- All files MUST have `#![deny(unsafe_code)]` **unless** the file
  appears in the unsafe allowlist below
- All public items MUST have doc comments
- Use `defmt` for logging, not `log` or `println!`

### Unsafe Code Isolation Policy

Isolating `unsafe` to specific files enables `#![deny(unsafe_code)]`
as a linter and early compiler check across the vast majority of
the codebase.

**Allowed unsafe files** (file-level `#![allow(unsafe_code)]`):

| File | Justification |
|------|---------------|
| `boards/feather-stm32f405/src/ccmram.rs` | `#[link_section]`, `static mut` for CCM RAM |

**Rules:**

- Allowed files MUST be minimal — only the code that *requires*
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

## Testing

- Platform-agnostic code in `core/` MUST have unit tests
- BSP code is tested via integration tests on hardware
- Use `defmt-test` for on-device tests

## Dependencies

Prefer crates in this order:
1. RTIC ecosystem (`rtic`, `rtic-sync`, `rtic-monotonics`)
2. Embassy ecosystem (`embassy-stm32`, `embassy-net`, etc.)
3. `embedded-hal` ecosystem
4. Other well-maintained embedded crates

## File Naming

- Use snake_case for all Rust files
- BSP crates named: `{board-name}` (e.g., `feather-stm32f405`)
- App crates named: `{descriptive-name}` (e.g., `mqtt-sensor-node`)

## Markdown and Writing Style Guide

- Wrap text after the first word extended beyond 72 characters, and do not
  exceed 80 characters.  Wrap before 72 characters if the last word would extend
  beyond 80 characters.  Exceptions: URLs, long paths, and code examples.
- Use two spaces after sentences for readability.  Markdown is frequently read
  in monospace text editors where kerning is not used, and the extra spacing
  helps visually separate sentences.
- Use an oxford comma in lists of three or more items for clarity, e.g., "A, B,
  and C".
- Do not use forced line breaks (two or more spaces at the end of a line.)

## Shell Script Style Guide

- Wrap comments at the first word extended beyond 72 characters, and do
  not exceed 80 characters.  Wrap before 72 characters if the last word
  would extend beyond 80 characters.  Exceptions: URLs, long paths, and
  code examples.
- Prefer POSIX syntax over Bash- or Zsh-specific syntax, except in the following
  cases:
    - Where shell-specific features are faster to execute, e.g.,
       `[[ "abc123" =~ c1 ]` instead of `echo abc123 | grep -q c1`.
    - Where shell-specific features are significantly more readable, e.g.,
       `<<-` heredocs with indented content and `source` insead of `.`.
- Use shell-specific features that add safety, e.g., `set -o pipefail`, `local`,
 `readonly`, `typeset`, etc.
- Quote strings with 'hard quotes' unless variable expansion is needed.
- Enclose all variables in curly braces when they are part of a larger string,
  e.g., `"this ${string}"`, but not when they are a standalone, e.g.,
  `"$solitary_variable"`.
- Use `UPPER_SNAKE_CASE` for variables intended to be used across multiple
  functions, e.g., configuration variables.
- Use `_underscore_lead_lower_snake_case` for file-local variables and
  functions, regardless of whether they are declared with `local` or not.
- Use `lower_snake_case` for functions intended to be used across multiple
  files, e.g., utility functions.
- Unset unneeded functions and variables not needed after use, e.g., temporary
  variables and helper functions.
- Batch `export` and `unset` calls as a micro-optimization-- speed matters!
- Use XDG Base Directory Specification wherever possible.
- Use tabs for indentation.  Spaces may be used after tabs for `<<-` heredoc
  content, e.g. to indent the output file with its native intendation, but
  the initial indentation must be tabs.
- Always use `set -o <option>` for shell options, one per line.
  - `errexit` must be the first option if used.
  - POSIX-compatible options come second (e.g. `nounset`, `noexec`).
  - Common non-POSIX options come third (e.g. `pipefail`).
  - Bash-specific options come last.

## Git Commit Style Guide

- One logical change per commit
- Use Markdown formatting for body if needed (e.g., lists, code blocks)
- Present tense: "add" not "added"
- Imperative mood: "fix bug" not "fixes bug"
- Reference issues: `Closes: #123`, `Refs: #456`
- Keep headline concise, under 50 characters if possible, no more than 72
- The body should focus on the "why" and "how" more than the "what" (which
  should be in the headline)
- Wrap text at 72 characters except for URLs
- Use Git trailers for metadata and references: `Co-authored-by:`, `See-also:`,
  etc.
