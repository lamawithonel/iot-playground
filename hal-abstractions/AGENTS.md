# hal-abstractions/-- Hardware Abstraction Traits

Trait boundary between `core/` logic and board hardware.  `no_std`,
host-testable, minimal dependencies (`embedded-io`/
`embedded-io-async` behind feature `io`).

## Contents

| Module | Purpose |
|--------|---------|
| `sensor.rs` | `EnvironmentalReading` trait (deci-scaled accessors) |
| `message_port.rs` | Bounded inter-task message channel traits |
| `rng.rs` | Cryptographically secure random byte source trait |
| `rtc.rs` | Battery-backed wall-clock (`Rtc`) trait |
| `time.rs` | Wall-clock timestamp vocabulary for the `Rtc` trait |
| `test_support.rs` | Host-test doubles (`cfg(test)` / `mock` feature) |

The `network` trait module (DNS/TCP/TLS/MQTT session abstraction) is
planned but not yet implemented (placeholder in `lib.rs`).  Adding it
is framework modularization work-- see the roadmap's framework track
before starting.

## Local Rules

- Traits only; no concrete hardware types from `embassy-*` or PACs.
- Every trait addition needs a host-side test exercising a mock
  implementation (see
  [`testing_gates.md`](../.agents/rules/testing_gates.md)).
