# hal-abstractions/-- Hardware Abstraction Traits

Trait boundary between `core/` logic and board hardware.  `no_std`,
host-testable, minimal dependencies (`embedded-io`/
`embedded-io-async` behind feature `io`).

## Contents

| Module | Purpose |
|--------|---------|
| `sensor.rs` | `EnvironmentalReading` trait (deci-scaled accessors) |
| `adc_capture.rs` | `AdcCapture` trait (windowed Q15 sample capture) |
| `excitation.rs` | `ExcitationSink` trait (Q15 excitation block output) |
| `record_store.rs` | `RecordStore` trait (append-only serialized ARS record sink) |
| `message_port.rs` | Bounded inter-task message channel traits |
| `rng.rs` | Cryptographically secure random byte source trait |
| `rtc.rs` | Battery-backed wall-clock (`Rtc`) trait |
| `time.rs` | Wall-clock timestamp vocabulary for the `Rtc` trait |
| `test_support.rs` | Host-test doubles (`cfg(test)` / `mock` feature) |

A `network` trait module (DNS/TCP/TLS/MQTT session abstraction) was
once planned here (placeholder in `lib.rs`), but the framework took a
different shape: the shared, transport-agnostic client implementation
lives in the concrete [`iot-net/`](../iot-net/AGENTS.md) crate over
`embassy-net`, with board couplings injected via buffers and
closures.  Do not add a network trait module here without revisiting
that decision in the roadmap's framework track.

## Local Rules

- Traits only; no concrete hardware types from `embassy-*` or PACs.
- Every trait addition needs a host-side test exercising a mock
  implementation (see
  [`testing_gates.md`](../.agents/rules/testing_gates.md)).
