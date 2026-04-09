//! Cucumber-RS smoke test validator for RTT and MQTT output.
//!
//! Reads captured RTT log files and MQTT message files, then
//! runs Gherkin feature specs to validate device behavior.
//!
//! # Usage
//!
//! ```bash
//! RTT_LOG_FILE=/tmp/rtt.log \
//! MQTT_MSG_FILE=/tmp/mqtt.jsonl \
//! smoke-validator
//! ```

#![deny(warnings)]
#![deny(unsafe_code)]

mod correlation;
mod mqtt;
mod rtt;

use std::path::PathBuf;

use cucumber::World;

/// Deserialized MQTT telemetry message from the device.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MqttMessage {
    pub msg_id: u32,
    pub timestamp: u64,
    pub micros: u32,
    pub temp_c: Option<f64>,
    pub humidity: Option<f64>,
    pub pm1_0: Option<f64>,
    pub pm2_5: Option<f64>,
    pub pm4_0: Option<f64>,
    pub pm10: Option<f64>,
    pub co2: Option<f64>,
    pub voc: Option<f64>,
    pub nox: Option<f64>,
}

/// State container for each Cucumber scenario.
///
/// Created fresh per-scenario from environment variables set by
/// the mise smoke test orchestrator.
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct SmokeTestWorld {
    /// Raw RTT log content.
    pub rtt_log: String,
    /// RTT log split into lines.
    pub rtt_lines: Vec<String>,
    /// Parsed MQTT messages (post stale-trimming).
    pub mqtt_messages: Vec<MqttMessage>,
    /// Device's SNTP-synced epoch (authoritative wall clock).
    pub device_epoch: Option<u64>,
    /// Number of "Publishing #N" lines in the RTT output.
    pub rtt_publish_count: Option<u32>,
    /// Number of stale messages trimmed by the orchestrator.
    pub stale_trimmed: u32,
    /// Smoke test duration in seconds.
    pub test_duration: u64,
    /// Expected publish interval in seconds.
    pub sample_interval: u64,
    /// Path for CSV artifact output.
    pub csv_output: Option<PathBuf>,
    /// When true, remaining steps in this scenario are skipped.
    pub skip_remaining: bool,
}

impl SmokeTestWorld {
    /// Initialize from environment variables.
    ///
    /// All paths and values are set by the mise smoke task
    /// orchestrator before invoking this binary.
    fn new() -> Result<Self, anyhow::Error> {
        let rtt_log = match std::env::var("RTT_LOG_FILE") {
            Ok(path) => std::fs::read_to_string(&path)?,
            Err(_) => String::new(),
        };
        let rtt_lines: Vec<String> = rtt_log
            .lines()
            .map(String::from)
            .collect();

        let mqtt_messages = match std::env::var("MQTT_MSG_FILE") {
            Ok(path) => {
                let content = std::fs::read_to_string(&path)?;
                content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| {
                        serde_json::from_str::<MqttMessage>(line)
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            Err(_) => Vec::new(),
        };

        let device_epoch = std::env::var("MQTT_DEVICE_EPOCH")
            .ok()
            .and_then(|v| v.parse().ok());

        let rtt_publish_count = std::env::var("MQTT_RTT_PUBLISH_COUNT")
            .ok()
            .and_then(|v| v.parse().ok());

        let stale_trimmed: u32 = std::env::var("MQTT_STALE_TRIMMED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let test_duration: u64 = std::env::var("SMOKE_TEST_DURATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(165);

        let sample_interval: u64 = std::env::var("SAMPLE_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let csv_output = std::env::var("MQTT_CSV_FILE")
            .ok()
            .map(PathBuf::from);

        Ok(Self {
            rtt_log,
            rtt_lines,
            mqtt_messages,
            device_epoch,
            rtt_publish_count,
            stale_trimmed,
            test_duration,
            sample_interval,
            csv_output,
            skip_remaining: false,
        })
    }
}

#[tokio::main]
async fn main() {
    // Write CSV artifact before running tests (cucumber exits
    // the process, so we must do this first).
    write_csv_artifact();

    let features_path = std::env::var("CUCUMBER_FEATURES_DIR")
        .unwrap_or_else(|_| {
            // Default: look for features/ relative to the binary,
            // falling back to test/features/ from the repo root.
            let exe = std::env::current_exe().unwrap_or_default();
            let repo_root = exe
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists() && p.join("test").exists())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            repo_root
                .join("test")
                .join("features")
                .to_string_lossy()
                .into_owned()
        });

    SmokeTestWorld::cucumber()
        .max_concurrent_scenarios(Some(1))
        .run_and_exit(&features_path)
        .await;
}

/// Write MQTT messages to a CSV artifact file if `MQTT_CSV_FILE`
/// is set.  This is called before the cucumber runner so the
/// artifact is available regardless of test outcome.
fn write_csv_artifact() {
    let csv_path = match std::env::var("MQTT_CSV_FILE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };

    let mqtt_file = match std::env::var("MQTT_MSG_FILE") {
        Ok(path) => path,
        Err(_) => return,
    };

    let content = match std::fs::read_to_string(&mqtt_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: cannot read {mqtt_file}: {e}");
            return;
        }
    };

    let non_empty_lines: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let mut messages = Vec::new();
    let mut skipped = 0u32;
    for (i, line) in non_empty_lines.iter().enumerate() {
        match serde_json::from_str::<MqttMessage>(line) {
            Ok(msg) => messages.push(msg),
            Err(e) => {
                if skipped == 0 {
                    eprintln!(
                        "warning: skipping malformed JSON at line {} \
                         of {mqtt_file}: {e}",
                        i + 1,
                    );
                }
                skipped += 1;
            }
        }
    }
    if skipped > 0 {
        eprintln!(
            "warning: {skipped} of {} MQTT line(s) skipped due to \
             parse errors",
            non_empty_lines.len(),
        );
    }

    if messages.is_empty() {
        return;
    }

    let file = match std::fs::File::create(&csv_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "warning: cannot create {}: {e}",
                csv_path.display()
            );
            return;
        }
    };

    let mut wtr = csv::Writer::from_writer(file);
    // Header
    #[allow(clippy::needless_borrows_for_generic_args)]
    if wtr
        .write_record(&[
            "msg_id",
            "timestamp",
            "micros",
            "temp_c",
            "humidity",
            "pm1_0",
            "pm2_5",
            "pm4_0",
            "pm10",
            "co2",
            "voc",
            "nox",
        ])
        .is_err()
    {
        return;
    }

    for msg in &messages {
        let row = [
            msg.msg_id.to_string(),
            msg.timestamp.to_string(),
            msg.micros.to_string(),
            opt_f64(msg.temp_c),
            opt_f64(msg.humidity),
            opt_f64(msg.pm1_0),
            opt_f64(msg.pm2_5),
            opt_f64(msg.pm4_0),
            opt_f64(msg.pm10),
            opt_f64(msg.co2),
            opt_f64(msg.voc),
            opt_f64(msg.nox),
        ];
        if wtr.write_record(&row).is_err() {
            break;
        }
    }

    let _ = wtr.flush();
    eprintln!(
        "CSV artifact: {} ({} messages)",
        csv_path.display(),
        messages.len()
    );
}

/// Format an optional f64 as a string or empty for CSV output.
fn opt_f64(val: Option<f64>) -> String {
    match val {
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
