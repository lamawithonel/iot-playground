# Agent Instructions for iot-playground

This file is a lazy-loaded index.  Read the linked file for a topic
when -- and only when -- the task touches it.  Do not inline-include
other files from here.

## Project Overview

A modular embedded Rust framework for STM32 and Microchip ATSAM
MCUs, emphasizing real-time performance and security.  Built as a
Cargo workspace: shared platform-agnostic logic in `core/`, traits
in `hal-abstractions/`, and one board profile per directory under
`boards/`.  Firmware for projects built on the framework lives in
`boards/`; per-project docs (currently the ARS toolhead sensor)
live in `docs/src/projects/`.

## Critical Constraints

These apply to every change; violations fail review.

### RTIC-First Architecture

- **REQUIRED:** RTIC 2.x for all task scheduling
- **REQUIRED:** `rtic-sync` for inter-task communication
- **FORBIDDEN:** `embassy-executor` (crate or executor features)
- **ALLOWED:** Embassy HAL, PAC, and network crates
  (`embassy-stm32`, `embassy-net`, `embassy-net-wiznet`, etc.)
- **PREFER:** `rtic-sync` over `embassy-sync` where possible

### Interrupt-Driven Design

- **REQUIRED:** WFI/Sleep between interrupt events
- **REQUIRED:** Hardware timers for periodic interrupts
- **REQUIRED:** EXTI for external peripheral interrupts
- **FORBIDDEN:** Busy-wait loops (except brief hardware delays)

### Memory Model

- **REQUIRED:** `no_std`; no heap allocation (no `alloc` crate)
- **ALLOWED:** `heapless` collections, `static_cell`

## Device Tiers

- **Tier 1 (Minimal):** ≤128KB RAM, no TLS, basic I/O only
- **Tier 2 (Connected):** ≥192KB RAM, TLS/MQTT capable, primary target
- **Tier 3 (Gateway):** ≥512KB RAM, multi-protocol, edge compute

## Directory Index

Each directory below has its own `AGENTS.md` with local detail.

| Path | Contents |
|------|----------|
| [`core/`](core/AGENTS.md) | Platform-agnostic business logic (no hardware deps) |
| [`hal-abstractions/`](hal-abstractions/AGENTS.md) | Hardware abstraction traits |
| [`boards/`](boards/AGENTS.md) | Board profiles (one BSP + app per directory) |
| [`test/`](test/AGENTS.md) | Host-side test infra (broker, subscriber, smoke validator) |
| [`docs/`](docs/AGENTS.md) | mdBook documentation site |

## Rules, Skills, and Agents

Project rules live in `.agents/rules/`, skills in
`.agents/skills/`, and subagent definitions in `.agents/agents/`
(see [`.agents/README.md`](.agents/README.md) for the symlink
wiring).

| Rule | When to read |
|------|--------------|
| [`rust_style.md`](.agents/rules/rust_style.md) | Writing or reviewing any Rust code; unsafe policy |
| [`testing_gates.md`](.agents/rules/testing_gates.md) | Before any commit; choosing which checks to run |
| [`git_commit_style.md`](.agents/rules/git_commit_style.md) | Writing commit messages |
| [`markdown_style.md`](.agents/rules/markdown_style.md) | Editing any Markdown |
| [`shell_style.md`](.agents/rules/shell_style.md) | Editing shell scripts (`.mise/tasks/`, `test/scripts/`) |
