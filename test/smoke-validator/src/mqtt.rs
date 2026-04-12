//! MQTT message parsing and Cucumber step definitions.

use cucumber::{given, then, when};

use crate::SmokeTestWorld;

// ── Given steps ─────────────────────────────────────────

/// Loads MQTT messages; succeeds if any messages were captured.
#[given("I have captured MQTT messages from a smoke test")]
fn have_mqtt_messages(world: &mut SmokeTestWorld) {
    assert!(
        !world.mqtt_messages.is_empty(),
        "No MQTT messages were captured"
    );
}

// ── When steps ──────────────────────────────────────────

/// Condition: at least N messages must be captured for this
/// scenario to be meaningful.  Skips remaining steps if not.
#[when(expr = "at least {int} messages have been captured")]
fn at_least_n_messages(world: &mut SmokeTestWorld, min_count: usize) {
    if world.mqtt_messages.len() < min_count {
        world.skip_remaining = true;
    }
}

/// Sets up context: a named field is present in at least one message.
/// Subsequent Then steps operate on messages where that field exists.
/// If the field is absent in all messages, remaining steps in this
/// scenario are silently skipped.
#[when(expr = "the field {string} is present in any message")]
fn field_present_in_any(world: &mut SmokeTestWorld, field: String) {
    let present = world.mqtt_messages.iter().any(|m| field_value(m, &field).is_some());
    if !present {
        // Not an error — the field just hasn't conditioned yet.
        // Skip remaining Then steps in this scenario.
        world.skip_remaining = true;
    }
}

/// Condition: the test ran longer than a given threshold.
/// If the test is too short, remaining steps in this scenario
/// are silently skipped.
#[when(expr = "the test ran longer than {int} seconds")]
fn test_ran_longer_than(world: &mut SmokeTestWorld, threshold: u64) {
    if world.test_duration < threshold {
        world.skip_remaining = true;
    }
}

// ── Then steps: structural validation ───────────────────

/// Every message must be valid JSON (guaranteed by serde
/// deserialization in World::new, so this always passes if
/// we got here).
#[then("every message should be valid JSON")]
fn every_message_valid_json(world: &mut SmokeTestWorld) {
    // Already validated during World initialization.
    // If any message failed to parse, World::new() would have
    // returned an error.
    assert!(
        !world.mqtt_messages.is_empty(),
        "No messages to validate"
    );
}

/// Every message must have a specific required field.
#[then(expr = "every message should have a {string} field")]
fn every_message_has_field(world: &mut SmokeTestWorld, field: String) {
    for (i, msg) in world.mqtt_messages.iter().enumerate() {
        assert!(
            field_value(msg, &field).is_some(),
            "Message {} (msg_id={}) is missing required field \"{field}\"",
            i + 1,
            msg.msg_id
        );
    }
}

// ── Then steps: range validation ────────────────────────

/// All values of a named field must be within [min, max].
/// Only checks messages where the field is present.
#[then(expr = "all {string} values should be between {float} and {float}")]
fn all_values_between(
    world: &mut SmokeTestWorld,
    field: String,
    min: f64,
    max: f64,
) {
    if world.skip_remaining {
        return;
    }
    let mut checked = 0;
    for (i, msg) in world.mqtt_messages.iter().enumerate() {
        if let Some(val) = field_value(msg, &field) {
            assert!(
                val >= min && val <= max,
                "Message {} (msg_id={}): {field}={val} outside [{min}, {max}]",
                i + 1,
                msg.msg_id,
            );
            checked += 1;
        }
    }
    if checked == 0 {
        // Field never appeared — this is OK for unconditioned sensors.
        // The conditioning tests handle that separately.
    }
}

/// The last captured message must contain the named field.
/// Used for conditioning timeline validation.
#[then(expr = "the last message should have a {string} value")]
fn last_message_has_field(world: &mut SmokeTestWorld, field: String) {
    if world.skip_remaining {
        return;
    }
    let last = world.mqtt_messages.last()
        .expect("No messages captured");
    assert!(
        field_value(last, &field).is_some(),
        "Last message (msg_id={}) is missing \"{field}\" — \
         sensor may not have conditioned in time",
        last.msg_id,
    );
}

// ── Then steps: msg_id sequencing ───────────────────────

/// The first message's msg_id must be 1.
#[then("the first msg_id should be 1")]
fn first_msg_id_is_one(world: &mut SmokeTestWorld) {
    let first = world.mqtt_messages.first()
        .expect("No messages captured");
    assert_eq!(
        first.msg_id, 1,
        "First msg_id is {} (expected 1) — stale messages may not have been trimmed",
        first.msg_id,
    );
}

/// msg_id values must be strictly increasing.
#[then("msg_id values should be strictly increasing")]
fn msg_id_strictly_increasing(world: &mut SmokeTestWorld) {
    for window in world.mqtt_messages.windows(2) {
        let (prev, curr) = (&window[0], &window[1]);
        assert!(
            curr.msg_id > prev.msg_id,
            "msg_id not strictly increasing: {} → {} \
             (messages {} and {})",
            prev.msg_id, curr.msg_id,
            prev.msg_id, curr.msg_id,
        );
    }
}

/// Timestamps must be non-decreasing.
#[then("timestamps should be non-decreasing")]
fn timestamps_non_decreasing(world: &mut SmokeTestWorld) {
    for window in world.mqtt_messages.windows(2) {
        let (prev, curr) = (&window[0], &window[1]);
        assert!(
            curr.timestamp >= prev.timestamp,
            "Timestamp decreased: {} → {} (msg_id {} → {})",
            prev.timestamp, curr.timestamp,
            prev.msg_id, curr.msg_id,
        );
    }
}

// ── Then steps: timestamp plausibility ──────────────────

/// All timestamps must be plausible Unix timestamps
/// (between 2023-11-14 and 2033-05-18).
#[then("every timestamp should be plausible")]
fn timestamps_plausible(world: &mut SmokeTestWorld) {
    const MIN_TS: u64 = 1_700_000_000;
    const MAX_TS: u64 = 2_000_000_000;
    for msg in &world.mqtt_messages {
        assert!(
            msg.timestamp >= MIN_TS && msg.timestamp <= MAX_TS,
            "msg_id={}: timestamp {} outside plausible range [{MIN_TS}, {MAX_TS}]",
            msg.msg_id, msg.timestamp,
        );
    }
}

/// All micros values must be in [0, 999999].
#[then("every micros value should be between 0 and 999999")]
fn micros_in_range(world: &mut SmokeTestWorld) {
    for msg in &world.mqtt_messages {
        assert!(
            msg.micros <= 999_999,
            "msg_id={}: micros={} exceeds 999999",
            msg.msg_id, msg.micros,
        );
    }
}

/// All msg_id values must be positive (≥ 1).
#[then("every msg_id should be positive")]
fn msg_id_positive(world: &mut SmokeTestWorld) {
    for msg in &world.mqtt_messages {
        assert!(
            msg.msg_id >= 1,
            "msg_id={} is not positive",
            msg.msg_id,
        );
    }
}

// ── Then steps: inter-message timing ────────────────────

/// Compute inter-message intervals from timestamps.
fn inter_message_intervals(messages: &[crate::MqttMessage]) -> Vec<f64> {
    messages
        .windows(2)
        .map(|w| {
            let dt_secs = w[1].timestamp as f64 - w[0].timestamp as f64;
            let dt_micros = w[1].micros as f64 - w[0].micros as f64;
            dt_secs + dt_micros / 1_000_000.0
        })
        .collect()
}

/// The median inter-message interval should be within a
/// percentage of the expected sample interval.
#[then(regex = r"^the median inter-message interval should be within (\d+)% of the sample interval$")]
fn median_interval_near_sample(
    world: &mut SmokeTestWorld,
    tolerance_pct: u64,
) {
    if world.skip_remaining {
        return;
    }
    let mut intervals = inter_message_intervals(&world.mqtt_messages);
    if intervals.is_empty() {
        return;
    }
    intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = intervals[intervals.len() / 2];
    let expected = world.sample_interval as f64;
    let tolerance = expected * tolerance_pct as f64 / 100.0;

    assert!(
        (median - expected).abs() <= tolerance,
        "Median inter-message interval {median:.1}s is not within \
         {tolerance_pct}% of expected {expected}s \
         (tolerance: {tolerance:.1}s)",
    );
}

/// No single gap between consecutive messages should exceed
/// a multiple of the sample interval.
#[then(regex = r"^no inter-message gap should exceed (\d+) times the sample interval$")]
fn no_excessive_gap(
    world: &mut SmokeTestWorld,
    multiplier: u64,
) {
    if world.skip_remaining {
        return;
    }
    let intervals = inter_message_intervals(&world.mqtt_messages);
    let max_gap = world.sample_interval as f64 * multiplier as f64;

    for (i, &gap) in intervals.iter().enumerate() {
        assert!(
            gap <= max_gap,
            "Gap between msg_id={} and msg_id={} is {gap:.1}s \
             (exceeds {multiplier}x sample interval = {max_gap:.0}s)",
            world.mqtt_messages[i].msg_id,
            world.mqtt_messages[i + 1].msg_id,
        );
    }
}

// ── Helper: dynamic field access ────────────────────────

/// Extract a named field's value from an MqttMessage as f64.
fn field_value(msg: &crate::MqttMessage, field: &str) -> Option<f64> {
    match field {
        "msg_id" => Some(msg.msg_id as f64),
        "timestamp" => Some(msg.timestamp as f64),
        "micros" => Some(msg.micros as f64),
        "temp_c" => msg.temp_c,
        "humidity" => msg.humidity,
        "pm1_0" => msg.pm1_0,
        "pm2_5" => msg.pm2_5,
        "pm4_0" => msg.pm4_0,
        "pm10" => msg.pm10,
        "co2" => msg.co2,
        "voc" => msg.voc,
        "nox" => msg.nox,
        _ => None,
    }
}
