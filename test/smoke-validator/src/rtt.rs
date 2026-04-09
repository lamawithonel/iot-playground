//! RTT log parsing and Cucumber step definitions.

use cucumber::{given, then};
use regex::Regex;

use crate::SmokeTestWorld;

// ── Given steps ─────────────────────────────────────────

/// Loads RTT output; succeeds if any RTT data was captured.
#[given("I have captured RTT output from a smoke test")]
fn have_rtt_output(world: &mut SmokeTestWorld) {
    assert!(
        !world.rtt_log.is_empty(),
        "No RTT output was captured — device may not have booted"
    );
}

// ── Then steps: literal string matching ─────────────────

/// Asserts the RTT log contains a literal substring.
#[then(expr = "the RTT log should contain {string}")]
fn log_contains(world: &mut SmokeTestWorld, expected: String) {
    assert!(
        world.rtt_log.contains(&expected),
        "RTT log does not contain \"{expected}\""
    );
}

/// Asserts the RTT log contains at least one of two alternatives.
#[then(expr = "the RTT log should contain {string} or {string}")]
fn log_contains_either(
    world: &mut SmokeTestWorld,
    first: String,
    second: String,
) {
    assert!(
        world.rtt_log.contains(&first) || world.rtt_log.contains(&second),
        "RTT log contains neither \"{first}\" nor \"{second}\""
    );
}

// ── Then steps: regex matching ──────────────────────────

/// Asserts the RTT log matches a regex pattern.
#[then(expr = "the RTT log should match {string}")]
fn log_matches(world: &mut SmokeTestWorld, pattern: String) {
    let re = Regex::new(&pattern)
        .unwrap_or_else(|e| panic!("invalid regex \"{pattern}\": {e}"));
    assert!(
        re.is_match(&world.rtt_log),
        "RTT log does not match pattern /{pattern}/"
    );
}

// ── Then steps: absence checks ──────────────────────────

/// Asserts the RTT log does NOT contain a literal substring.
#[then(expr = "the RTT log should not contain {string}")]
fn log_does_not_contain(world: &mut SmokeTestWorld, unexpected: String) {
    assert!(
        !world.rtt_log.contains(&unexpected),
        "RTT log unexpectedly contains \"{unexpected}\""
    );
}

/// Asserts the RTT log does NOT match a regex pattern.
#[then(expr = "the RTT log should not match {string}")]
fn log_does_not_match(world: &mut SmokeTestWorld, pattern: String) {
    let re = Regex::new(&pattern)
        .unwrap_or_else(|e| panic!("invalid regex \"{pattern}\": {e}"));
    assert!(
        !re.is_match(&world.rtt_log),
        "RTT log unexpectedly matches pattern /{pattern}/"
    );
}

// ── Then steps: duration-aware conditioning ─────────────

/// Asserts the RTT log contains a pattern, but only if the
/// test ran long enough for the sensor to condition.
#[then(expr = "the RTT log should contain {string} given at least {int} seconds")]
fn log_contains_within(
    world: &mut SmokeTestWorld,
    expected: String,
    threshold: u64,
) {
    if world.test_duration < threshold {
        // Test too short — skip rather than fail.
        return;
    }
    assert!(
        world.rtt_log.contains(&expected),
        "RTT log does not contain \"{expected}\" \
         (test ran {}s, threshold {threshold}s)",
        world.test_duration
    );
}

/// Asserts the RTT log matches a regex, but only if the test
/// ran long enough.
#[then(expr = "the RTT log should match {string} given at least {int} seconds")]
fn log_matches_within(
    world: &mut SmokeTestWorld,
    pattern: String,
    threshold: u64,
) {
    if world.test_duration < threshold {
        return;
    }
    let re = Regex::new(&pattern)
        .unwrap_or_else(|e| panic!("invalid regex \"{pattern}\": {e}"));
    assert!(
        re.is_match(&world.rtt_log),
        "RTT log does not match /{pattern}/ \
         (test ran {}s, threshold {threshold}s)",
        world.test_duration
    );
}
