# test/-- Host-Side Test Infrastructure

Containerized broker plus host tools that validate the device from
the outside.  These crates are `workspace.exclude`d and are NOT
built by CI; they run via mise tasks only.

## Contents

| Path | Purpose |
|------|---------|
| `broker/` | Mosquitto MQTT broker container (`mise run broker:start`) |
| `features/` | Cucumber BDD specs consumed by `smoke-validator/`, split into `mqtt/` and `rtt/` suites |
| `mqtt-subscriber/` | Host subscriber for captured telemetry |
| `smoke/` | Per-board smoke suites (`<board>.sh`), dispatched by the `test:smoke` router |
| `smoke-validator/` | Cucumber-RS smoke/integration validator (tiered durations) |
| `scripts/` | Shared scripts (TLS cert generation) |

## Test Tiers

The smoke and integration tests share the same three duration
tiers.  Each tier is a separate mise task name-- there is no
`--tier` flag:

| Tier | Smoke task | Integration task |
|------|------------|-------------------|
| Standard | `mise run test:smoke` | `mise run test:integration` |
| Extended | `mise run test:smoke-extended` | `mise run test:integration-extended` |
| Full | `mise run test:smoke-full` | `mise run test:integration-full` |

These tasks are file tasks under `.mise/tasks/test/`.  `test:host`
is the exception-- it is defined inline in `.mise.toml`, not as a
file task.  See
[`testing.md`](../docs/src/development/testing.md) for the full
five-layer test pyramid and what each tier adds.

## Host Crate Conventions

- Both host crates (`mqtt-subscriber/` and `smoke-validator/`)
  commit their `Cargo.lock`.
- Every new host crate copies the `.cargo/config.toml`
  boilerplate from an existing one.  This keeps the crate from
  inheriting the workspace's `thumbv7em-none-eabihf` target, so
  it builds for the host triple instead.
- No CI covers these crates, so contributors must run
  `cargo fmt` and `cargo clippy` manually inside each one.

## Local Rules

- Shell scripts follow
  [`shell_style.md`](../.agents/rules/shell_style.md).
- Generated certs and keys live under gitignored `.local/`; never
  commit key material.
- After changing broker config, restart it:
  `mise run broker:stop && mise run broker:start`.
