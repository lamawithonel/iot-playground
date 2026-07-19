# nucleo-h753zi/-- NUCLEO-H753ZI Bring-Up / Prototyping Rig

ST NUCLEO-H753ZI (STM32H753ZI, Cortex-M7 @ 64 MHz HSI in phase 1,
2 MB flash, 128 KB DTCM).  RTIC 2.x heartbeat blink today; planned
home for an ARS DAC/ADC loopback rig and the ADR-009 Layer-3
network trigger board.

## Module Map

| Module | Purpose |
|--------|---------|
| `main.rs` | RTIC `#[app]`: init, heartbeat, idle (WFI) |
| `audio_loopback/` (planned) | DAC1_OUT1 (PA4) -> jumper -> ADC (PA3) sweep loopback; synthesis logic lives in `core/` |
| `net/` (planned) | On-chip Ethernet MAC + on-board PHY over RMII + `embassy-net`; the ADR-009 Layer-3 trigger board |

See the
[ARS toolhead sensor project docs](../../docs/src/projects/ars-toolhead-sensor/README.md)
for the loopback rig's role in the wider ARS pipeline.

## Local Rules

- No unsafe allowlist entries here; nothing in this crate may be
  `unsafe`.
- RTIC-first applies: RTIC 2.x scheduling, Embassy HAL only (no
  embassy-executor).
- Phase 1 clock config is `embassy_stm32::Config::default()` (HSI
  64 MHz) deliberately: the board's HSE is an 8 MHz ST-LINK MCO
  with no crystal populated (UM2407 Rev 6 p.25-26 Sec 7.5.1); any
  crystal-mode HSE config hangs before the first log line.  An
  explicit HSE-bypass + PLL1 + LSE config is a planned follow-up.
- RTIC dispatchers are `UART4`/`UART5`/`UART7`, deliberately not
  `USART1`-`USART3`: `USART3` is the ST-LINK VCP (UM2407 Rev 6
  p.28 Sec 7.6.5) and must stay free for a real UART console
  later.
- Hardware access is fenced to the ST-LINK at `0483:374e` with an
  explicit `--probe` selector in every command; never open the
  J-Link (`1366:1020`) shared with another workstream.
- Promoted to `workspace.members` 2026-07-19 after
  hardware-verified bring-up; host tests exclude this bin crate
  (`--exclude nucleo-h753zi`), clippy/build gates cover it.
