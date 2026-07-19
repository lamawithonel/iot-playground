# mqtt-subscriber

Minimal MQTT subscriber used to capture device telemetry during
smoke and integration testing.  It connects to the test broker over
TLS, subscribes to the telemetry topic, and writes each received
payload to an output file, one JSON message per line.

## Invocation

`.mise/tasks/test/smoke` builds this crate in release mode and
starts it in the background-- but only when `BROKER_HOST_IP` is set
and a CA certificate is present at `.local/certs/ca/root.crt`.  It
runs for the smoke test's configured duration, then the orchestrator
kills it and hands the captured file to `smoke-validator`.  The
`test:smoke-extended` and `test:smoke-full` tasks exec the same
script with a longer duration, so they invoke it the same way.

## Environment Variables

- `BROKER_HOST_IP` -- Broker IP address (required).
- `BROKER_PORT` -- Broker port (default: `8883`).
- `MQTT_CA_FILE` -- CA certificate PEM file (default:
  `.local/certs/ca/root.crt`).
- `MQTT_TOPIC` -- Subscribe topic (default: `device/+/telemetry`).

## Manual Build

```sh
cargo build --release --manifest-path test/mqtt-subscriber/Cargo.toml
```

Run it directly with `mqtt-subscriber <output-file> [duration-secs]`
(duration defaults to 120 seconds).
