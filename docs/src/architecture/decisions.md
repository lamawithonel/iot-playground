# Architecture Decision Records
## Embedded Rust IoT Firmware

This document captures key architectural decisions using a lightweight ADR format.

---

## ADR-001: RTIC vs Embassy Executor

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need to choose between RTIC and Embassy executor for task scheduling in a real-time embedded system.

**Decision:** Use RTIC 2.x for task scheduling, Embassy for HAL and drivers.

**Rationale:**
- RTIC provides formal verification via Stack Resource Policy (SRP)
- Hard real-time guarantees required for interrupt latency (<500μs)
- Embassy executor is cooperative and cannot provide hard deadlines

**Consequences:**
- Steeper learning curve combining two frameworks
- Some driver incompatibilities (e.g., cannot use Embassy executor-dependent drivers)
- Must use `rtic-sync` instead of `embassy-sync` for inter-task communication where possible
- Embassy HAL works well; Embassy executor features must be avoided

**Alternatives Considered:**
- Pure Embassy: Rejected due to lack of formal real-time guarantees
- Pure RTIC with PAC: Would work but loses Embassy HAL conveniences

---

## ADR-002: W5500 vs ESP32 WiFi

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need network connectivity for MQTT communication with AWS IoT Core.

**Decision:** Use W5500 hardwired Ethernet controller.

**Rationale:**
- Hardware TCP/IP offload reduces MCU load
- Deterministic behavior (no WiFi interference/reconnection issues)
- Easier debugging (Wireshark on physical network)
- Industrial reliability (no RF concerns)

**Consequences:**
- Requires physical Ethernet cable (no wireless mobility)
- Additional SPI peripheral usage
- Simpler power management (no WiFi radio)

**Alternatives Considered:**
- ESP32 as WiFi coprocessor: More complex, less deterministic
- STM32 with built-in Ethernet (F7/H7): Higher cost, different board

---

## ADR-003: Protocol Buffers vs JSON

**Date:** 2026-01-03
**Updated:** 2026-04-05
**Status:** Accepted (updated with implementation details)

**Context:**
Need to choose serialization format for MQTT payloads.

**Decision:** Use Protocol Buffers (protobuf) for telemetry
payloads via `micropb` 0.6.  Use JSON via `serde-json-core`
for AWS Device Shadows and IoT Jobs (required by AWS reserved
topics).

**Rationale:**

- 82% smaller telemetry payloads (32 B vs 220 B JSON)
- Deterministic encoding (~7–11 µs on Cortex-M4 at 168 MHz)
- Native AWS IoT Rules Engine `decode()` support
- Type-safe with compile-time schema validation
- Wide adoption in IoT and cloud ecosystems (AWS IoT
  Core supports Protobuf natively)
- `micropb` is purpose-built for no_std/no_alloc environments

**Consequences:**

- Requires `.proto` schema files in `proto/` and
  `micropb-gen` code generation in `core/build.rs`
- Requires MSRV 1.88.0 (micropb requirement)
- AWS Device Shadows still need JSON — dual format approach
- Less human-readable; `buf` tooling for schema management
- AWS Rules Engine needs `.desc` descriptor files in S3

**Alternatives Considered:**

- JSON: Human-readable but 82% larger, slower parsing
- CBOR: 42 B (vs 32 B Protobuf), no native AWS `decode()`
- MessagePack: Less tooling, no AWS Rules Engine support
- `prost`: Requires `alloc` — incompatible with no_std

---

## ADR-004: E-ink vs OLED Display

**Date:** 2026-01-03  
**Status:** Accepted

**Context:**
Need local display for device status and sensor readings.

**Decision:** Use E-ink display (SSD1681, 200x200).

**Rationale:**
- Ultra-low power consumption (power only during updates)
- Sunlight readable (high contrast)
- Persistent display without power (shows last state during sleep)
- Appropriate for sensor data that updates every 60 seconds

**Consequences:**
- Slow refresh (~10 seconds for full update)
- Limited to monochrome or limited color
- More complex update logic (partial refresh optimization)

**Alternatives Considered:**
- OLED: Faster updates but higher power, burn-in risk
- LCD: Middle ground but requires backlight power

---

## ADR-005: TLS Library Selection

**Date:** 2026-01-12  
**Status:** Accepted

**Context:**
Need TLS 1.3 support for secure MQTT connections without heap allocation.

**Decision:** Use `embedded-tls` library (commit dccd966).

**Rationale:**
- No allocator required - works in pure `no_std` environment
- TLS 1.3 support with AES-128-GCM-SHA256 cipher suite
- Compatible with Embassy async traits (`embedded-io-async`)
- Active maintenance and embedded-focused design

**Consequences:**
- **No RSA certificate support** - servers must use ECDSA
- Limited cipher suites (primarily AES-GCM)
- No SHA-512/SHA-3 support
- Certificate verification optional (enabled in production)

**Alternatives Considered:**
- `rustls`: Excellent library but requires allocator
- `wolfSSL` (via FFI): FIPS-certified but C library dependency
- `mbedTLS` (via FFI): Widely used but larger footprint, C dependency

**Future Reconsideration Triggers:**
- Hardware upgrade with more memory (can use `rustls`)
- FIPS 140-3 certification requirement (would need `wolfSSL`)
- RSA certificate requirement from cloud provider

---

## ADR-006: Cargo Workspace Architecture

**Date:** 2026-01-18  
**Status:** Accepted

**Context:**
Need to support multiple board profiles (board type + peripherals + application purpose) in a single repository while sharing common code like network stacks and HAL abstractions.

**Decision:** Use Cargo workspace with board profile architecture.

**Rationale:**
- Centralized dependency management via workspace.dependencies
- Shared code in `core/` (platform-agnostic logic) and `hal-abstractions/` (hardware traits)
- Board profiles in `boards/` directory (e.g., `feather-stm32f405`)
- Each board profile = specific hardware + peripherals + application
- Single `.cargo/config.toml` at workspace root with common linker flags
- Board selection via probe-rs native `PROBE_RS_CONFIG_PRESET` environment variable
- Board-specific configurations centralized in root `Embed.toml`

**Consequences:**
- All boards share common Cargo profile settings (panic = "abort")
- Workspace-level dependency versions ensure consistency
- Board profiles can share code via workspace crates
- `memory.x` linker scripts handled per-board via `build.rs`
- Scalable: easy to add new board profiles without duplication
- Build from workspace root: `cargo run --release` or `cargo embed --release`

**Alternatives Considered:**
- Separate repositories per board: Would duplicate network/HAL code
- Git submodules: Complex dependency management, harder to refactor
- Monolithic single crate: Doesn't scale to multiple board types

**Board Profile Examples:**
- `boards/feather-stm32f405/` - STM32F405 + Ethernet + sensors + MQTT gateway
- Future: `boards/feather-ptp-server/` - STM32F405 + Ethernet + GPS + PTP server
- Future: `boards/feather-m4-can/` - SAMD51 + CAN + sensors

---

## ADR-007: CloudEvents `binary_data` Over `proto_data`

**Date:** 2026-04-05
**Status:** Accepted

**Context:**
The CloudEvents Protobuf Event Format v1.0.3-wip defines three
data variants: `binary_data` (bytes), `text_data` (string), and
`proto_data` (`google.protobuf.Any`).  The spec states that
`proto_data` MUST be used "where the data is a protobuf message."
However, `google.protobuf.Any` requires runtime type URLs and
dynamic dispatch, which are fundamentally incompatible with
no_std/no_alloc embedded systems.  Furthermore, `micropb` — the
only viable Protobuf crate for our constraints — does not
support `google.protobuf.Any`.

**Decision:** Use the `binary_data` (bytes) field to carry
Protobuf-encoded sensor telemetry inside the CloudEvents
envelope.  Identify the inner schema via the CloudEvents `type`
field (e.g., `iot.sen66.v1`).

**Rationale:**

- micropb 0.6 does not support `google.protobuf.Any`
- `binary_data` saves 12–20 bytes per message (no type URL
  overhead), reducing CloudEvents PB from ~96 B to ~80 B
- Encoding is simpler: two-pass (inner message → bytes →
  envelope) with no `Any::pack()` complexity
- Cloud-side routing uses the CloudEvents `type` field, not
  type URLs — functionally equivalent
- AWS IoT Rules Engine double-decode works with both variants
- Widely practiced in production IoT systems

**Consequences:**

- Technically non-compliant with CloudEvents Protobuf spec
  §3.2 ("data MUST be stored in `proto_data`" for protobuf)
- Cloud consumers must know the inner schema out-of-band
  (via `type` field) rather than via `type_url`
- The `datacontenttype` attribute should be set to
  `application/protobuf` to signal the binary encoding

**Alternatives Considered:**

- `proto_data` with manual `Any` encoding: Possible but adds
  complexity, 12–20 B overhead, and requires string formatting
  for type URLs on the MCU
- Custom envelope (skip CloudEvents): Loses interoperability
  with EventBridge and cross-system consumers
- CloudEvents JSON: 3.5× larger than CloudEvents PB; negates
  the benefit of Protobuf telemetry

---

## ADR-008: micropb for Protobuf Encoding

**Date:** 2026-04-05
**Status:** Accepted

**Context:**
Need a Protobuf library that works in `no_std` without an
allocator on Cortex-M4 (STM32F405, 192 KB SRAM, 1 MB flash).

**Decision:** Use `micropb` 0.6 with `micropb-gen` for
build-time code generation from `.proto` files.

**Rationale:**

- Purpose-built for no_std/no_alloc embedded systems
- Uses `heapless` containers for bounded collections
- TOML configuration files for per-field capacity budgets
- ~8–12 KB flash, <1 KB RAM for typical sensor schemas
- Deterministic encoding: ~1,200–1,800 cycles per message
- Proto3 support with full encode/decode
- Actively maintained and widely used in the Rust ecosystem

**Consequences:**

- MSRV 1.88.0 — must track recent stable Rust
- No `google.protobuf.Any` support (see ADR-007)
- No Protobuf Editions — proto3 only
- No defmt integration — requires manual `Format` impls
- Every `string`/`bytes`/`repeated`/`map` field needs explicit
  `max_len()` configuration in build.rs or TOML config
- `micropb-gen` requires `protoc` on the build host

**Alternatives Considered:**

- `prost`: Mature, widely used — requires `alloc` (disqualified)
- `femtopb`: Smallest footprint, zero-panic — borrow-based API
  complicates ownership across RTIC tasks, API still evolving
- `nanopb` (C via FFI): Proven but adds C toolchain dependency

---

## ADR-009: Test Strategy and the Embedded Test Pyramid

**Date:** 2026-04-06
**Updated:** 2026-07-18
**Status:** Accepted (updated: sixth bring-up test, Layer 5 gate)

**Context:**

The project has 29 host unit tests covering `core/` logic on
x86_64, three on-device tests via `embedded-test`, and a 45-second
RTT-based smoke test.  A review of the on-device tests revealed a
structural problem: all three tests (`sanity_check`,
`core_conditioning_on_device`, `heapless_string_on_device`) exercise
pure logic that already passes on the host.  `embedded-test` 0.7.x
replaces the RTIC dispatcher entirely — it cannot run async tasks,
`rtic_sync` channels, monotonic timers, or real peripheral
interactions.  The on-device tests prove the test harness works,
not the hardware.

Meanwhile, the only mechanism that validates actual hardware
integration — PLL configuration, I2C bus timing, SPI
initialization, interrupt routing, and the full sensor-to-MQTT data
path — is the smoke test, which amounts to "flash and hope the RTT
output looks right for 45 seconds."

For an IoT device with real-time constraints, this gap is
unacceptable.  The test strategy must be restructured
to spend hardware test cycles on things that *only hardware can
answer*, while expanding host-side coverage to catch logic and data
integrity bugs cheaply.

**Decision:** Adopt a five-layer embedded test pyramid with strict
layer ownership and an A+B hybrid on-device strategy.  Layer 2
uses `embedded-test` for peripheral bring-up validation (clock
tree, I2C, SPI, RNG, timer tick) — not pure logic.  Layer 3
uses custom `#[app]` test binaries for RTIC integration, deferred
until explicit trigger criteria are met.  Expand host-side
property and adversarial tests.  Formalize `defmt` milestones as
a machine-parseable test contract.  Invest in type-driven design
to eliminate classes of runtime tests.  Defer QEMU/Renode
emulation.

**Test Pyramid:**

| Layer | What It Tests | Where It Runs | Gate |
|-------|--------------|---------------|------|
| 1. Host unit tests | Pure logic: state machines, formatting, time math | CI (x86_64) | Required, every PR |
| 2. On-device peripheral smoke | Hardware init: clock tree, I2C probe, SPI/W5500, RNG, TIM2 tick | MCU via probe-rs | Required with hardware |
| 3. RTIC integration | Inter-task channels, monotonic timers, vertical data slices | Custom `#[app]` test binaries | Deferred (trigger-based) |
| 4. System smoke test | Full firmware boot, milestone ordering, error detection | MCU via probe-rs + RTT | Required with hardware |
| 5. End-to-end | Device → TLS → MQTT broker → subscriber validation | MCU + Mosquitto container | Required with hardware |

**Layer 1 — Host Unit Tests (expand):**

Add adversarial and property-based tests that exercise data
integrity across transformation boundaries:

- **Channel backpressure property test:** Mock `Sender`/`Receiver`
  pair, simulate 200 sensor reads with randomized MQTT stalls
  (0–300 s).  Assert no panic, receiver gets the most recent
  reading after drain, and dropped-reading count matches
  channel-full events.
- **Payload size budget:** Construct `Sen66Reading` with all 9
  fields at `i32::MAX` worst-case values.  Assert the formatted
  JSON fits in the `String<256>` buffer.
- **End-to-end data chain:** Trace a simulated sensor value from
  `to_deci()` through `Sen66Reading` through
  `format_json_payload()` and assert the final JSON field is
  correct.
- **NaN/Inf handling:** Assert that `to_deci(NaN)` and
  `to_deci(f32::INFINITY)` propagate as `None` through the
  pipeline (not as the plausible value `0.0`).
- **Schema contract test:** Define expected JSON field names,
  types, and optionality as a fixture.  Any change to
  `format_json_payload()` that alters the schema breaks this test.

**Layer 2 — On-Device Peripheral Tests (`bringup.rs`):**

Six `embedded-test` tests in `bringup.rs` validate peripheral
bring-up in `#[init]` context:

- **Clock tree validation:** Read back `SYSCLK` via `RCC->CFGR`,
  assert 84 MHz.  Read `PLLQ`, assert 48 MHz.  Catches PLL
  divider regressions that break RNG and TLS.
- **I2C bus probe:** Initialize I2C1 at 400 kHz, send SEN66
  `GetSerialNumber` (0xD033), assert valid 48-bit response.
  Proves bus electrical functionality and pull-up configuration.
- **SPI/W5500 version register:** Read W5500 register `0x0039`,
  assert `0x04`.  Proves SPI bus, chip select, and W5500 are
  alive.
- **RNG entropy check:** Read 32 bytes from hardware RNG, assert
  non-zero and not all-identical.  TLS depends on a functioning
  RNG — a stuck RNG is security-critical.
- **TIM2 monotonic tick sanity:** Start TIM2, read counter, spin
  with `cortex_m::asm::delay`, read again, assert delta within
  ±5%.  Catches prescaler misconfigurations.
- **W5500 INT pin (EXTI2):** Verify the W5500 INT line on PC2
  asserts and is observable via EXTI2.  Proves the wiring that
  interrupt-driven packet reception depends on.

**Layer 3 — RTIC Integration Tests (deferred):**

Custom `#[app]` test binaries are deferred until the firmware
topology grows beyond what the smoke test can cover.  Build
Layer 3 when any of the following trigger criteria are met:

- `Shared` gains real members (currently an empty struct — no
  shared resources means no priority inversions to test).
- A second board enters the workspace (cross-board regressions
  require isolated integration tests).
- Phase 4 adds TLS certificate verification (cert-chain
  validation interacts with RNG, timers, and network stack).
- Channel or priority topology changes (additional channels,
  split priority levels, or new inter-task dependencies).

**Layer 4 — Smoke Test (harden):**

- Define expected `defmt` milestones and assert their order
  (e.g., `"System initialized"` → `"TIM2 monotonic"` →
  `"I2C1 initialized"` → `"SEN66 initialized"` → `"Network
  stack initialized"` → `"SNTP sync successful"`).
- Add per-milestone timeouts, not just a single wall-clock
  timeout.
- Add a `TELEMETRY_JSON:` structured log prefix to the firmware
  that emits the exact JSON string passed to `mqtt.publish()`.
  Host scripts can parse RTT output and run schema validation.

**Type-Driven Testing (reduce test surface):**

Invest in newtypes and typestates to push runtime invariants
into the type system, eliminating entire classes of tests:

- `DeciCelsius(i32)`, `Ppm(u16)`, etc. — prevent field mixups
  at compile time.
- `Reading::Ready(T)` vs `Reading::Conditioning` — distinguish
  temporal absence from structural absence, replacing `Option<T>`
  ambiguity.
- `Phase` enum for conditioning stages — prevent out-of-bounds
  phase indices.
- `BoundedMicros(u32)` — enforce `< 1_000_000` at construction.
- `ValidatedTopic`/`ClientId` newtypes — validate once, carry
  proof in the type.

These are not part of the test infrastructure itself but they
reduce the test burden by making wrong states unrepresentable.

**CI/CD Strategy:**

- Host unit tests (`test:host`) are a required merge gate today.
- On-device and smoke tests require hardware; skip gracefully
  when no probe is detected.
- `Tested-on:` git trailer and `hw-verified` label provide
  human attestation for hardware-dependent changes.
- Self-hosted runner (Pi + J-Link, ~$200): start as nightly
  non-blocking, promote to required after 30-day reliability bake.
- Skip QEMU/Renode emulation — negative ROI at one board.
  Revisit at 3+ target boards.

**Rationale:**

- A 10-persona cross-functional review (RTIC specialist, safety
  architect, observability engineer, type-system advocate, CI/CD
  engineer, hardware engineer, security reviewer, protocol
  specialist, fleet operator, firmware QA) converged on the A+B
  hybrid approach.  7 of 10 supported the strategy; consensus
  covered both approach and timing — bring-up now, RTIC
  integration when trigger criteria fire.
- Key insight (Holloway): "three tasks at identical priority,
  empty Shared struct, single 2-slot channel — there are no
  priority inversions to test."  This justifies deferring Layer 3
  until the firmware topology actually demands it.
- The highest-risk untested scenario is the 2-slot `rtic_sync`
  channel under TLS stall: if the network task blocks for longer
  than 2× the sample interval, every subsequent sensor reading is
  silently discarded.  No current test exercises this.
- `embedded-test` cannot fill the RTIC integration gap, but it
  *can* validate that peripherals initialize and respond.
  Restructuring the on-device tests to do this is high-value and
  low-effort.
- Type-driven design eliminates tests that exist only because
  the type system permits nonsense inputs.  On a device where
  every test requires a flash cycle, this leverage is enormous.

**Consequences:**

- `bringup.rs` replaces the three former pure-logic on-device
  tests with six peripheral validation tests (clock tree PLL,
  RNG entropy, I2C SEN66 probe, SPI W5500 version register,
  TIM2 tick rate, W5500 INT/EXTI2 wiring)-- a breaking change
  to test output.
- Pure logic tests are removed from on-device; host tests
  already cover them.
- The smoke test becomes the primary integration gate until
  Layer 3 trigger criteria fire and RTIC test binaries are
  built.
- New host tests will catch data integrity bugs (NaN handling,
  buffer overflow, schema drift) that currently have zero
  coverage.
- The `defmt` milestone contract formalizes log output as
  testable behavior, which constrains future refactoring — log
  message changes become test-breaking changes.
- Type system improvements (newtypes, typestates) will require
  API changes across `core/`, `hal-abstractions/`, and the BSP.

**Alternatives Considered:**

- **Keep on-device tests as-is:** Rejected.  They consume flash
  time and CI complexity while providing no coverage beyond host
  tests.
- **QEMU/Renode emulation:** Negative ROI at one board.  STM32F405
  peripheral models are incomplete, RTIC interrupt dispatch has
  subtle timing dependencies, and defmt integration is immature.
- **RTIC integration test binaries now:** Deferred until trigger
  criteria fire.  The current RTIC topology — three tasks at
  identical priority, empty `Shared`, single 2-slot channel —
  does not warrant the tooling investment.
- **Full HIL framework now:** Deferred to Phase 3+.  Requires
  self-hosted runner, USB passthrough, and container networking
  that are not yet in place.

---

## ADR-010: ARS Toolhead-Sensor Project Track on STM32N657 (Scaffold)

**Date:** 2026-07-18
**Status:** Proposed (scaffold landed; accept at branch merge)

**Context:**
The framework needs a second consumer to prove its abstractions
generalize beyond the STM32F405 air-quality node.  The candidate is
an active acoustic resonance spectroscopy (ARS) device clamped to a
Bambu Lab H2C toolhead to detect filament presence in the hot-end
and cold-end.  Labeled positive/negative ARS captures will later
train a passive CNN that detects filament at cold-pull start from
printer sound alone; generalized fault detection is explicitly out
of scope.  Prototype hardware: NUCLEO-N657X0-Q (STM32N657X0:
Cortex-M55 @ 800 MHz, 4.2 MB RAM, flashless-- boots via signed
FSBL from external NOR, Neural-ART NPU), an Adafruit MAX9744 20 W
class-D amp driving a Dayton Audio EX25VT2-4 exciter (25 mm,
4 ohm) as the acoustic source, and a SparkFun SPH8878LR5H-1 MEMS
mic breakout (BOB-19389) modified with a TI OPA345NA op-amp and a
30 kOhm resistor replacing C3.  The hardware is not in hand yet;
today's work is scaffold only.

**Decision:** Add the ARS toolhead sensor as a second project track
in this repository: a `boards/nucleo-n657x0` crate, excluded from
the workspace until it compiles in CI, with project documentation
under `docs/src/projects/ars-toolhead-sensor/`.  The RTIC-first +
Embassy-HAL constraints (ADR-001) apply to the ARS firmware
unchanged.

**Rationale:**

- A second, dissimilar consumer (acoustic DSP and NPU inference vs
  periodic sensor telemetry) pressure-tests the `core/` and
  `hal-abstractions/` split from ADR-006 far better than a clone
  of the first board would.
- Workspace exclusion keeps CI green while the STM32N6 Rust
  ecosystem matures; embassy-n6 youth, the unverified probe-rs
  flashless-boot flow, and C-centric Neural-ART tooling are all
  tracked as risk R10.
- The Cortex-M55's Helium (MVE) vector extension and the
  Neural-ART NPU give DSP and CNN-inference headroom no current
  board in the workspace has.
- 4.2 MB of SRAM holds chirp excitation buffers, response
  captures, and FFT working sets without external RAM.
- A docs-first scaffold records the signal chain and requirements
  now, so hardware bring-up starts from a plan instead of ad-hoc
  experiments.

**Consequences:**

- The excluded crate gets zero CI coverage until it is promoted to
  workspace membership; API drift against `core/` is possible
  until it first compiles.
- Flashless boot (signed FSBL from external NOR) introduces a
  flash-and-debug workflow unlike any existing board; the probe-rs
  flow is unverified (R10).
- Neural-ART deployment will likely require generated C artifacts
  linked into the Rust firmware, adding a C toolchain to this
  board's build.
- Framework changes must now be validated against two consumers
  with very different duty cycles (periodic telemetry vs streaming
  audio).
- ADR-006's board-profile architecture gains its second concrete
  profile, exercising its "scalable" claim.

**Alternatives Considered:**

- **Separate repository:** Rejected.  The framework must co-evolve
  with a second consumer; a separate repo would duplicate the
  shared code that ADR-006 exists to centralize.
- **Immediate workspace membership:** Rejected.  With no hardware
  and no proven embassy-n6 build, an uncompilable member crate
  breaks CI for every board.
- **Reusing STM32F4-class hardware:** Rejected.  No NPU and weak
  DSP headroom for CNN inference; the M55 + Neural-ART capability
  is the reason this project exists.
- **Adopting embassy-executor for this board:** Rejected.  It
  violates the framework's RTIC-first constraint (ADR-001) and
  would split the stack into two scheduling models.

---

## Template for New ADRs

```markdown
## ADR-XXX: [Title]

**Date:** YYYY-MM-DD
**Status:** [Proposed | Accepted | Deprecated | Superseded]

**Context:**
[What is the issue we're addressing?]

**Decision:** [What did we decide?]

**Rationale:**
[Why did we make this decision?]

**Consequences:**
[What are the implications?]

**Alternatives Considered:**
[What other options were evaluated?]
```

---

*Update this document when significant architectural decisions
are made.*
