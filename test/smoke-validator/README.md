# smoke-validator

Cucumber-RS validator that checks captured RTT log output and MQTT
telemetry against Gherkin feature specs (`test/features/`).  It
covers boot milestones, WFI sleep behavior, MQTT protocol shape,
sensor conditioning windows, and RTT<->MQTT cross-correlation.  It
also writes a CSV telemetry artifact before running any scenarios.

## Invocation

`.mise/tasks/test/smoke` builds and runs this crate on the host
after flashing firmware and capturing RTT and MQTT output.  The
`test:smoke-extended` and `test:smoke-full` tasks exec the same
script with longer durations, and the `test:integration*` tasks run
it as part of the full pipeline.  The orchestrator sets every
environment variable below before invoking the binary.

## Environment Variables

- `RTT_LOG_FILE` -- Path to the captured RTT log (empty if unset).
- `MQTT_MSG_FILE` -- Path to captured MQTT messages, one JSON object
  per line (empty if unset).
- `MQTT_DEVICE_EPOCH` -- Device's SNTP-synced epoch, used as the
  authoritative wall clock.
- `MQTT_RTT_PUBLISH_COUNT` -- Count of "Publishing #" lines in the
  RTT log, for cross-validation against MQTT message count.
- `MQTT_STALE_TRIMMED` -- Number of stale messages the orchestrator
  trimmed from a prior firmware session (default: `0`).
- `SMOKE_TEST_DURATION` -- Test duration in seconds (default:
  `165`); gates `@extended` (>=300s) and `@full` (>=780s) scenarios.
- `SAMPLE_INTERVAL_SECS` -- Expected publish interval in seconds
  (default: `5`).
- `MQTT_CSV_FILE` -- Path to write the CSV telemetry artifact
  (skipped if unset).
- `CUCUMBER_FEATURES_DIR` -- Path to the feature files directory
  (default: auto-detected `test/features/` from the repo root).

## Manual Build

```sh
cargo build --release --manifest-path test/smoke-validator/Cargo.toml
```
