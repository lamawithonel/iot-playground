# tools/ars-synth-- Golden-Corpus Generator (`ars-synth`)

Host-side `std` binary that generates and verifies the golden-vector
corpus under [`core/tests/golden/`](../../core/tests/golden/README.md).
Workspace-`exclude`d (see root `Cargo.toml`); `Cargo.lock` is
committed per the host-crate convention.

## Invocation

Run from the repo root; the `--target` override is required because
the workspace `.cargo/config.toml` defaults to
`thumbv7em-none-eabihf` (same pattern as `test/smoke-validator/`):

```sh
# (Re)generate the corpus in place:
cargo run --manifest-path tools/ars-synth/Cargo.toml \
  --target "$(rustc -vV | sed -n 's/^host: //p')" -- gen

# Verify the checked-in corpus matches a fresh in-memory run:
cargo run --manifest-path tools/ars-synth/Cargo.toml \
  --target "$(rustc -vV | sed -n 's/^host: //p')" -- check
```

An optional trailing argument overrides the corpus directory.

## Layout

| Path | Purpose |
|------|---------|
| `src/main.rs` | Arg parsing (hand-rolled, no CLI dep), `gen`/`check` modes |
| `src/fixtures.rs` | Fixture definitions; the only f64 signal-design code |
| `../../core/tests/golden/driver.rs` | Shared (via `#[path]`) format parser/renderer and integer kernel-replay engine |

## Local Rules

- Expected outputs MUST come from `iot-core`'s own kernels via the
  shared driver-- never a reimplementation-- so the corpus is
  bit-exact against the on-device fixed-point math by construction.
- Determinism: fixed seeds, fixed timestamp constants, no
  wall-clock or unseeded randomness anywhere.
- After any deliberate kernel change: run `gen`, review the corpus
  diff, and re-run `cargo test -p iot-core --all-features`.
- No CI covers this crate; run `cargo fmt` and `cargo clippy`
  manually here (with the host `--target` override).
