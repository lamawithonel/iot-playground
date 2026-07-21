use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // --- memory.x linker script ---
    fs::File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=memory.x");

    // --- MQTT broker endpoint (net feature only) ---
    generate_mqtt_endpoint(out);
}

/// Emit `OUT_DIR/mqtt_endpoint.rs` with the bench broker address when
/// the `net` feature (which compiles the MQTT task) is active.
///
/// The broker hostname/IP and port are bench-network topology: they
/// are injected at build time via `BROKER_HOST`/`BROKER_PORT` and
/// must never be committed (mirrors feather's `SAMPLE_INTERVAL_SECS`
/// build-time-env mechanism).  A missing value under `net` is a loud
/// panic, deliberately not a silent placeholder default-- a wrong
/// default silently pointed at the wrong broker is worse than a build
/// failure for bench data.
///
/// Gated on `CARGO_FEATURE_NET` (cargo sets `CARGO_FEATURE_<NAME>`
/// per active feature) so a bare `cargo check`-- which hits the
/// scaffold `compile_error!`-- and a `--features g1-spike` build
/// never require these vars.  `src/config.rs`, the only `include!`er
/// of this file, is itself `net`-gated, so nothing reads it otherwise.
fn generate_mqtt_endpoint(out: &Path) {
    println!("cargo::rerun-if-env-changed=BROKER_HOST");
    println!("cargo::rerun-if-env-changed=BROKER_PORT");

    if env::var_os("CARGO_FEATURE_NET").is_none() {
        return;
    }

    let host = env::var("BROKER_HOST").unwrap_or_else(|_| {
        panic!(
            "BROKER_HOST must be set for a `--features net` build \
             (bench broker hostname/IP, injected at build time, never committed)"
        )
    });
    let port: u16 = env::var("BROKER_PORT")
        .unwrap_or_else(|_| panic!("BROKER_PORT must be set for a `--features net` build"))
        .parse()
        .unwrap_or_else(|e| panic!("BROKER_PORT must be a u16: {e}"));

    // Confirm injection without echoing the endpoint: the host and
    // port are bench topology and must not land in build logs.  The
    // panic on missing vars above already reports the failure case.
    println!("cargo::warning=MQTT endpoint set from BROKER_HOST/BROKER_PORT");

    fs::write(
        out.join("mqtt_endpoint.rs"),
        format!(
            "/// MQTT broker hostname or IP (set at build time via `BROKER_HOST`).\n\
             pub const BROKER_HOST: &str = \"{host}\";\n\
             /// MQTT broker port (set at build time via `BROKER_PORT`).\n\
             pub const BROKER_PORT: u16 = {port};\n"
        ),
    )
    .expect("failed to write mqtt_endpoint.rs");
}
