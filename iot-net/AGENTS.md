# iot-net/-- Shared Network Clients (`iot-net`)

Transport-agnostic MQTT/TLS/SNTP/DNS client logic over
`embassy-net`, extracted from the Feather board crate so any board
profile (F4, H7, future ARS reporting nodes) can reuse it.
`no_std`, `#![deny(unsafe_code)]`, no `embassy-stm32`, no PAC, no
board dependency.  Board couplings are injected by the caller:
client ID as `&str`, all working buffers (including TLS record
buffers, so placement such as CCM RAM stays a board concern), a
`now` timestamp closure, an `on_cycle` per-publish telemetry hook,
and an SNTP `on_sync` callback for RTC/wall-clock application.

## Contents

| Module | Purpose |
|--------|---------|
| `client.rs` | `NetworkClient` trait for protocol implementations |
| `config.rs` | `SntpConfig` (servers, timeout, retries, stratum) |
| `error.rs` | Re-exports of `iot_core::network::error` enums |
| `manager.rs` | DHCP wait/log helper for `embassy-net` stacks |
| `mqtt.rs` | MQTT v5 + TLS 1.3 client, reconnect backoff, PUBACK |
| `sntp.rs` | SNTP client (RFC 4330 validation lives in `iot-core`) |
| `socket.rs` | `AsyncTcpSocket` embedded-io-async adapter |
| `tls.rs` | TLS constants (cipher suite notes, backoff bounds) |

## Local Rules

- No hardware types: `embassy-net` socket/DNS APIs only; anything
  board-specific enters through function parameters or closures.
- `SimpleCryptoProvider` + `NoVerify` are intentional Phase 2
  choices; certificate verification lands with AWS IoT readiness.
- No host tests: `embassy-net` does not build for the host target,
  so this crate is excluded from the host-test workspace run and
  compile-gated by the embedded clippy/build workspace runs (see
  [`testing_gates.md`](../.agents/rules/testing_gates.md)).
- Behavior changes here affect every board; regressions must be
  re-verified with the Feather broker-smoke suite on hardware.
