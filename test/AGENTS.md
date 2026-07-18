# test/ -- Host-Side Test Infrastructure

Containerized broker plus host tools that validate the device from
the outside.  These crates are `workspace.exclude`d and are NOT
built by CI; they run via mise tasks only.

## Contents

| Path | Purpose |
|------|---------|
| `broker/` | Mosquitto MQTT broker container (`mise run broker:start`) |
| `mqtt-subscriber/` | Host subscriber for captured telemetry |
| `smoke-validator/` | Cucumber-RS smoke/integration validator (tiered durations) |
| `scripts/` | Shared scripts (TLS cert generation) |

## Local Rules

- Shell scripts follow
  [`shell_style.md`](../.agents/rules/shell_style.md).
- Generated certs and keys live under gitignored `.local/`; never
  commit key material.
- After changing broker config, restart it:
  `mise run broker:stop && mise run broker:start`.
