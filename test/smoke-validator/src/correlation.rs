//! Cross-stream correlation step definitions.
//!
//! Validates consistency between RTT log events and captured
//! MQTT messages using the device's SNTP epoch as the
//! authoritative time reference.

use cucumber::{then, when};
use regex::Regex;

use crate::SmokeTestWorld;

// ── When steps ──────────────────────────────────────────

/// Extracts publish count from the RTT log for correlation.
#[when("the RTT log shows publish events")]
fn rtt_shows_publishes(world: &mut SmokeTestWorld) {
    let count = world
        .rtt_lines
        .iter()
        .filter(|line| line.contains("Publishing #"))
        .count();
    // Store in the world for downstream Then steps if not
    // already set from the environment variable.
    if world.rtt_publish_count.is_none() {
        world.rtt_publish_count = Some(count as u32);
    }
}

/// Checks that the device has SNTP epoch available.
#[when("the device has synchronized via SNTP")]
fn device_has_sntp(world: &mut SmokeTestWorld) {
    if world.device_epoch.is_none() {
        // Try to extract from RTT log.
        let re = Regex::new(r"SNTP sync successful: (\d+)")
            .expect("valid regex");
        if let Some(caps) = re.captures(&world.rtt_log) {
            if let Some(epoch_str) = caps.get(1) {
                world.device_epoch = epoch_str
                    .as_str()
                    .parse()
                    .ok();
            }
        }
    }
    assert!(
        world.device_epoch.is_some(),
        "Device SNTP epoch not available — \
         SNTP sync may have failed"
    );
}

// ── Then steps: publish count correlation ───────────────

/// MQTT message count should be within tolerance of RTT count.
///
/// QoS 0 may lose messages, so MQTT ≤ RTT is acceptable.
/// More than 10% loss is a warning-level concern; the step
/// fails only on egregious mismatch.
#[then(regex = r"^the MQTT message count should be within (\d+)% of the RTT publish count$")]
fn mqtt_count_within_tolerance(
    world: &mut SmokeTestWorld,
    tolerance_pct: u32,
) {
    let rtt_count = world.rtt_publish_count
        .expect("RTT publish count not available");
    let mqtt_count = world.mqtt_messages.len() as u32;

    if rtt_count == 0 {
        assert_eq!(
            mqtt_count, 0,
            "RTT shows 0 publishes but received {mqtt_count} MQTT messages"
        );
        return;
    }

    if mqtt_count == rtt_count {
        return;
    }

    if mqtt_count > rtt_count {
        // More MQTT than RTT is unexpected but not necessarily
        // an error (could be retained messages).
        return;
    }

    let loss = rtt_count - mqtt_count;
    let loss_pct = (loss * 100) / rtt_count;
    assert!(
        loss_pct <= tolerance_pct,
        "MQTT received {mqtt_count}/{rtt_count} messages \
         ({loss} lost, {loss_pct}% > {tolerance_pct}% threshold)"
    );
}

// ── Then steps: SNTP epoch validation ───────────────────

/// Every MQTT timestamp must be at or after the device epoch.
#[then("every MQTT timestamp should be at or after the device epoch")]
fn timestamps_after_epoch(world: &mut SmokeTestWorld) {
    let epoch = world.device_epoch
        .expect("Device epoch not set");
    for msg in &world.mqtt_messages {
        assert!(
            msg.timestamp >= epoch,
            "msg_id={}: timestamp {} is before SNTP epoch {epoch}",
            msg.msg_id, msg.timestamp,
        );
    }
}

/// The last MQTT timestamp should be within N seconds of epoch.
#[then(regex = r"^the last MQTT timestamp should be within (\d+) seconds of the device epoch$")]
fn last_timestamp_within_window(
    world: &mut SmokeTestWorld,
    window_secs: u64,
) {
    let epoch = world.device_epoch
        .expect("Device epoch not set");
    let last = world.mqtt_messages.last()
        .expect("No MQTT messages captured");
    let elapsed = last.timestamp.saturating_sub(epoch);
    assert!(
        elapsed <= window_secs,
        "Last message is {elapsed}s after SNTP epoch \
         (expected ≤{window_secs}s)"
    );
}

/// The last MQTT timestamp should fall within the test window.
///
/// Uses `test_duration` as the window.  The actual elapsed
/// time is always less because probe-rs flash overhead (~40 s)
/// and SNTP synchronization (~2 s) consume part of the budget.
#[then("the last MQTT timestamp should be within the test window")]
fn last_timestamp_within_test_window(world: &mut SmokeTestWorld) {
    let epoch = world.device_epoch
        .expect("Device epoch not set");
    let last = world.mqtt_messages.last()
        .expect("No MQTT messages captured");
    assert!(
        last.timestamp >= epoch,
        "Last message timestamp ({}) is before the SNTP epoch ({epoch}) \
         — clock skew or corrupt data",
        last.timestamp,
    );
    let elapsed = last.timestamp - epoch;
    let window = world.test_duration;
    assert!(
        elapsed <= window,
        "Last message is {elapsed}s after SNTP epoch \
         (expected ≤{window}s = test_duration)"
    );
}
