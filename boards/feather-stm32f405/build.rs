use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Minimum allowed sample interval in seconds
const MIN_INTERVAL: u64 = 1;

/// Maximum allowed sample interval in seconds
const MAX_INTERVAL: u64 = 3600;

/// Default sample interval for debug builds (seconds)
const DEBUG_DEFAULT: u64 = 5;

/// Default sample interval for release builds (seconds)
const RELEASE_DEFAULT: u64 = 60;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // --- memory.x linker script ---
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=memory.x");

    // --- embedded-test linker script for on-device tests ---
    // Only applies to [[test]] targets, not the main binary.
    println!("cargo::rustc-link-arg-tests=-Tembedded-test.x");

    // --- SAMPLE_INTERVAL_SECS ---
    println!("cargo::rerun-if-env-changed=SAMPLE_INTERVAL_SECS");

    let profile = env::var("PROFILE").unwrap_or_default();

    let interval = match env::var("SAMPLE_INTERVAL_SECS") {
        Ok(val) => {
            let parsed: u64 = val.parse().unwrap_or_else(|_| {
                panic!("SAMPLE_INTERVAL_SECS must be a positive integer, got: {val}");
            });
            assert!(
                (MIN_INTERVAL..=MAX_INTERVAL).contains(&parsed),
                "SAMPLE_INTERVAL_SECS={parsed} out of range ({MIN_INTERVAL}–{MAX_INTERVAL})"
            );
            println!("cargo::warning=SAMPLE_INTERVAL_SECS={parsed}s (env override)");
            parsed
        }
        Err(_) => {
            let default = if profile == "release" {
                RELEASE_DEFAULT
            } else {
                DEBUG_DEFAULT
            };
            println!("cargo::warning=SAMPLE_INTERVAL_SECS={default}s ({profile} default)");
            default
        }
    };

    fs::write(
        out.join("sample_interval.rs"),
        format!(
            "/// Sample interval in seconds (set at build time)\n\
             ///\n\
             /// Override with `SAMPLE_INTERVAL_SECS=N cargo build`.\n\
             /// Debug default: {DEBUG_DEFAULT} s, release default: {RELEASE_DEFAULT} s.\n\
             pub const SAMPLE_INTERVAL_SECS: u64 = {interval};\n"
        ),
    )
    .expect("failed to write sample_interval.rs");
}
