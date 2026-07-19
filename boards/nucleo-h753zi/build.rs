use std::env;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // --- memory.x linker script ---
    std::fs::File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=memory.x");

    // NOTE: no `cargo::rustc-link-arg-tests=-Tembedded-test.x` yet
    // (as in the feather crate's build.rs): cargo rejects that
    // instruction when the package has no `[[test]]` target, and
    // no `tests/bringup.rs` exists yet (planned per ADR-009).  Add
    // it back alongside the `[[test]]` entry when that test lands.
}
