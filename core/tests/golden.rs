//! Golden-corpus regression test for the ARS fixed-point pipeline
//!
//! Loads every fixture under `core/tests/golden/` and asserts that
//! the current `iot-core` kernels reproduce the stored `expected_*`
//! sections bit-exactly, using the same integer replay engine
//! (`golden/driver.rs`) that `tools/ars-synth` uses to generate the
//! corpus.  If a kernel's behavior changes, this test fails until
//! the corpus is deliberately regenerated with `ars-synth gen`.

#![cfg(feature = "ars")]
#![deny(warnings)]
#![deny(unsafe_code)]

#[path = "golden/driver.rs"]
#[allow(dead_code)]
mod driver;

use std::fs;
use std::path::PathBuf;

/// Number of fixture files the corpus is expected to contain
const FIXTURE_COUNT: usize = 10;

fn corpus_files() -> Vec<PathBuf> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"));
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    files.sort();
    files
}

#[test]
fn test_corpus_has_expected_fixture_count() {
    assert_eq!(
        corpus_files().len(),
        FIXTURE_COUNT,
        "fixture count changed-- regenerate with `ars-synth gen` and update FIXTURE_COUNT"
    );
}

#[test]
fn test_every_fixture_replays_bit_exact() {
    let files = corpus_files();
    assert!(!files.is_empty(), "golden corpus is missing");
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("fixture file name")
            .to_string();
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: read: {e}"));
        let fixture =
            driver::Fixture::parse(&text).unwrap_or_else(|e| panic!("{name}: parse: {e}"));
        let recomputed =
            driver::compute_expected(&fixture).unwrap_or_else(|e| panic!("{name}: replay: {e}"));

        let stored: Vec<&(String, Vec<i64>)> = fixture
            .sections
            .iter()
            .filter(|(n, _)| n.starts_with("expected_"))
            .collect();
        assert_eq!(
            stored.len(),
            recomputed.len(),
            "{name}: expected-section count mismatch"
        );
        for ((stored_name, stored_vals), (name_r, vals_r)) in stored.iter().zip(&recomputed) {
            assert_eq!(stored_name, name_r, "{name}: section name/order mismatch");
            assert_eq!(
                stored_vals.len(),
                vals_r.len(),
                "{name}: [{stored_name}] length mismatch"
            );
            for (i, (a, b)) in stored_vals.iter().zip(vals_r).enumerate() {
                assert_eq!(
                    a, b,
                    "{name}: [{stored_name}] index {i} not bit-exact-- kernel behavior \
                     changed; if intended, regenerate the corpus with `ars-synth gen`"
                );
            }
        }
    }
}
