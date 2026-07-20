---
paths:
  - "**/*.md"
  - "**/*.rs"
  - "**/*.toml"
  - "**/*.yaml"
  - "**/*.yml"
  - "**/*.sh"
  - "**/*.ld"
  - "**/*.x"
  - ".mise/tasks/**"
---

# Prose and Comment Style Guide

Register, structure, and craft for everything a reader reads: documentation
prose, Rust doc-comments and inline comments, and comments in TOML, YAML,
shell, and linker scripts.  [`markdown_style.md`](markdown_style.md) owns the
mechanics-- line wrap, sentence spacing, dashes, ASCII symbols; this file owns
what the words say and the order they say it in.  Where both apply, both apply.

## Register and Voice

- Write for a competent engineer meeting this file for the first time.  Assume
  fluency in embedded Rust and networking; assume no knowledge of this
  project's specifics.  Explain the project, not the fundamentals.
- Be direct and technical: state the fact, then the reason.  No marketing, no
  filler, no throat-clearing.  Delete openers like "In today's...", "It is
  worth noting that", "Simply", and "Basically".
- Do not use empty intensifiers.  These words are banned when they carry no
  measurable meaning: "leverage", "seamless", "robust", "cutting-edge",
  "powerful", "blazing-fast", "revolutionary", and "effortless".  An editor
  can grep for them.
- Match the claim to the evidence.  A scaffold is a scaffold, a provisional
  pin map is provisional, an unverified flow is unverified.  Never state a
  plan as a fact; mark open questions as open.  The ARS docs' "Status:",
  "provisional", and "unverified" markers are the model.
- Prefer the concrete to the abstract, and a strong verb to a nominalization:
  "the DMA overruns" beats "a DMA overrun condition occurs".

## Sentences and Paragraphs

- Documentation prose uses complete sentences, each ending in a period.
- Fragments are correct in tables, pin lists, and bullet labels, where the
  column header or list context supplies the verb.  "Clocks, pins, audio
  output, ADC setup" is a fine table cell; it need not be a sentence.
- Lead with the point.  The first sentence of a section or paragraph states
  its conclusion; supporting detail follows.  Do not bury the result under
  its own derivation.
- One idea per paragraph.  When the thought turns, start a new paragraph.
- Prefer the active voice.  The passive is acceptable when the actor is the
  hardware or is genuinely irrelevant: "the buffer is filled by DMA".

## Document Structure

- A README or AGENTS.md owes its reader three things, near the top: what this
  is (one or two sentences), how to build or use it, and where authority lives
  (which document wins when two disagree).
- Lead with status when status changes how the reader should treat the file:
  scaffold, provisional, deprecated, or active.  The nucleo-n657x0 README opens
  with "Status: scaffold only" for exactly this reason.
- This repo is a lazy-loaded index: each directory's AGENTS.md is the entry
  point and links downward.  State each fact in exactly one place-- its single
  source of truth-- and link to it from everywhere else.  ADRs live in
  decisions.md, risks in risk_register.md, and the ARS pin map in pinout.md.
- Documentation lives in the mdBook: board and project documentation belongs
  under `docs/src/` (board pages in `docs/src/boards/`, project docs in
  `docs/src/projects/`), never in `boards/`.  A `boards/*/` directory keeps
  only its `AGENTS.md` and a short README pointing at the board's page.
- A skill carries generic tool operation only-- how to drive the tool,
  anywhere.  Board wiring, bench setup, and project test criteria belong in
  the board page or project doc, with at most a pointer from the skill.
- Never inline-duplicate content that lives elsewhere.  Where you would restate
  it, link instead, and summarize only enough to orient the reader.
- When a summary and its source could drift apart, name the winner in the text:
  "pinout.md wins on any disagreement".
- A heading is a promise.  Name what is under it, and order sections so a
  scanning reader reaches the answer without reading the whole page.

## Rust Comments

rust_style.md decides which items need comments; this section is about writing
the comments it requires.

Inline comments (`//`):

- Comment the "why", never the "what".  State the constraints and invariants
  the code cannot show: hardware quirks, units, bit-width and overflow
  reasoning, safety contracts, and protocol or datasheet references.
- Do not narrate the next line.  A `// increment the counter` above
  `self.samples_processed += 1;` earns nothing; delete it.
- Cite the source of any magic number or non-obvious constant-- a datasheet
  table, an RFC section, a register field.  "GPDMA1 adc1_dma is REQSEL 7
  (RM0486 Table 98)" is worth more than the bare number.
- Attach units to quantities that have them: Hz, samples, milliseconds, Q15,
  Q1.30.

Doc-comments (`///`):

- Open with a one-line summary that stands on its own.  Rustdoc shows that
  first line in item listings, so it must read without the body.  Leave a
  blank line, then the detail.
- Document panics under a `# Panics` heading and error conditions under
  `# Errors` when a function returns a `Result` and its failure modes are not
  obvious from the signature.  `GoertzelBin::process_sample` is the model.
- Doc examples compile and run as doc-tests.  Keep them compiling.  Do not mark
  a broken example `ignore` to silence it-- an ignored example rots unnoticed.
  Use `no_run` when the code must compile but needs hardware or an executor to
  run, and `ignore` only when it genuinely cannot compile in the crate context,
  with a one-line comment saying why.

Module headers (`//!`):

- A `//!` header is warranted when the module embodies a non-obvious design or
  constraint worth stating once at the top: the Goertzel overflow analysis, or
  the `AdcCapture` trait's rationale.  Open with a one-line summary, then the
  reasoning.
- A module of obvious helpers needs only its one-line summary, not an essay.

## Comments in Other Languages

The constraint-stating philosophy is the same everywhere; only the syntax
changes.

- **TOML** (`Cargo.toml`, config): comment why a dependency is pinned,
  excluded, or feature-gated, not that it is a dependency.  "embassy-stm32
  0.6.0: N6 support starts at 0.5" earns its line.
- **YAML** (CI workflows, mise): comment the non-obvious step, or the reason a
  job is ordered or gated as it is; skip the self-evident ones.
- **Shell**: shell_style.md owns the mechanics; the comment states intent or a
  constraint, the same as in Rust.
- **Linker scripts** (`.ld`, `.x`): memory-region and section comments cite the
  hardware fact-- an `ORIGIN`/`LENGTH` from the memory map, or why a section is
  placed in CCM RAM.

## Terminology

- Spell a term out the first time you lean on it.  Define every project coinage
  at first use, per document, because a reader may enter on any page: "active
  acoustic resonance spectroscopy (ARS)", "first-stage boot loader (FSBL)".
- Established domain acronyms need no expansion.  The test: a competent
  embedded-Rust and networking engineer would not pause.  ADC, DAC, DMA, GPIO,
  I2C, SPI, PWM, EXTI, MCU, RTIC, HAL, BSP, TLS, MQTT, SNTP, DNS, FFT, and DSP
  all qualify.
- Never invent an abbreviation the reader cannot look up.  If a coinage is
  unavoidable, define it once and use it unchanged thereafter.
- Name a thing one way.  Do not call the same peripheral, task, or concept by
  two names; reuse the term the code already uses (capture window, dwell, bin).
- Expand or link project-internal shorthand on first use: gate labels (G0-G5),
  phase numbers, and ADR references (ADR-010) point to where they are defined.

## Anti-Patterns

Filler and empty intensifiers:

Before:

> Implementors handle their own errors gracefully (log and continue) rather
> than panicking, enabling robust operation in embedded systems.

After:

> Implementors log and continue on error rather than panicking.

The cut clause said nothing measurable, and "robust" is on the banned list.

Narrating the code instead of its constraints:

Before:

```rust
// multiply and shift
let doubled_cos_s1 = ((coeff * self.s1 as i128) >> 29) as i64;
```

After:

```rust
// coeff (up to 2^30) times |s1| (up to ~2^46) needs ~76 bits, so
// compute in i128 and narrow the shifted result back to i64.
let doubled_cos_s1 = ((coeff * self.s1 as i128) >> 29) as i64;
```

A marketing opener instead of a plain claim:

Before:

> In today's fast-paced additive-manufacturing landscape, this cutting-edge
> node leverages advanced acoustic techniques to deliver unparalleled insight.

After:

> An active acoustic resonance spectroscopy (ARS) device clamped to a Bambu
> Lab H2C toolhead.

Restating instead of linking:

Before: a board README copies the full pin table, which then drifts out of
sync with pinout.md.

After: the board README shows the header diagram, then defers-- "pinout.md is
the authority", "pinout.md wins on any disagreement"-- and the per-pin
rationale lives only in pinout.md.

## Editor's Checklist

Run this as a review rubric.  A "no" is a change to make.

- Does the opening say what this is, and (for a README) how to use it and where
  authority lives?
- Does each section and paragraph lead with its point?
- Is every claim matched to its evidence, with unverified things marked
  unverified?
- Are there banned intensifiers or throat-clearing openers to cut?  Grep:
  leverage, seamless, robust, cutting-edge, powerful, effortless.
- Is every acronym or coinage defined at first use, or plainly a known domain
  term?
- Do inline comments state constraints and sources rather than restate the
  code?
- Does each `///` open with a standalone one-line summary and document panics
  and errors where they apply?
- Do doc examples compile, with no needless `ignore`?
- Is any fact duplicated that should be stated once and linked?
- For each fact: does an ADR, board page, or project doc already own it?  If
  yes, link-- do not copy.  Is anything board- or project-specific sitting in
  a skill, a rule, or the root README?
- Read it back in your head: does it sound like a competent person talking, or
  like a brochure?
