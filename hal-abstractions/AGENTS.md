# hal-abstractions/-- Hardware Abstraction Traits

Trait boundary between `core/` logic and board hardware.  `no_std`,
host-testable, minimal dependencies (`embedded-io`/
`embedded-io-async` behind feature `io`).

## Contents

| Module | Purpose |
|--------|---------|
| `sensor.rs` | `EnvironmentalReading` trait (deci-scaled accessors) |

`rtc`, `rng`, and `network` trait modules are planned but not yet
implemented (placeholders in `lib.rs`).  Adding them is framework
modularization work-- see the roadmap's framework track before
starting.

## Local Rules

- Traits only; no concrete hardware types from `embassy-*` or PACs.
- Every trait addition needs a host-side test exercising a mock
  implementation (see
  [`testing_gates.md`](../.agents/rules/testing_gates.md)).
