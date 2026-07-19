//! `ars-synth`-- golden-corpus generator and checker
//!
//! Host-side tool that generates (and verifies) the checked-in
//! golden-vector corpus under `core/tests/golden/`, pairing each
//! excitation input (MLS, Schroeder multitone, Farina ESS) with the
//! bit-exact fixed-point outputs of `iot-core`'s DSP kernels and ARS
//! record encoders.
//!
//! - `ars-synth gen [dir]`-- (re)write every fixture file
//! - `ars-synth check [dir]`-- regenerate in memory and diff
//!   against the checked-in files; non-zero exit on any mismatch,
//!   missing file, or stray fixture
//!
//! `dir` defaults to `core/tests/golden/` resolved relative to this
//! crate.  Output is fully deterministic: fixed seeds, fixed
//! timestamp constants, no wall-clock reads.

#![deny(warnings)]
#![deny(unsafe_code)]

#[path = "../../../core/tests/golden/driver.rs"]
#[allow(dead_code)]
mod driver;
mod fixtures;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../core/tests/golden");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
    match args.get(1).map(String::as_str) {
        Some("gen") => gen(&dir),
        Some("check") => check(&dir),
        _ => {
            eprintln!("usage: ars-synth <gen|check> [corpus-dir]");
            ExitCode::FAILURE
        }
    }
}

fn gen(dir: &Path) -> ExitCode {
    fs::create_dir_all(dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
    for (name, fixture) in fixtures::build_all() {
        let path = dir.join(name);
        fs::write(&path, fixture.render())
            .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        println!("wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

fn check(dir: &Path) -> ExitCode {
    let generated = fixtures::build_all();
    let mut failures = 0usize;
    for (name, fixture) in &generated {
        let path = dir.join(name);
        match fs::read_to_string(&path) {
            Ok(text) if text == fixture.render() => {}
            Ok(_) => {
                eprintln!("MISMATCH: {}", path.display());
                failures += 1;
            }
            Err(e) => {
                eprintln!("MISSING: {} ({e})", path.display());
                failures += 1;
            }
        }
    }
    // Stray .txt files would be loaded by core/tests/golden.rs but
    // never regenerated-- flag them too.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_txt = path.extension().is_some_and(|ext| ext == "txt");
            let known = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| generated.iter().any(|(g, _)| *g == n));
            if is_txt && !known {
                eprintln!("STRAY: {}", path.display());
                failures += 1;
            }
        }
    }
    if failures == 0 {
        println!("corpus OK ({} fixtures)", generated.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("{failures} corpus failure(s)-- regenerate with `ars-synth gen`");
        ExitCode::FAILURE
    }
}
