# Agent Framework Directory

This directory holds the project's agentic AI coding framework:

- `rules/`-- always-applicable project rules (style, testing,
  commits).  Referenced from `AGENTS.md` index files; never
  inlined into them.  Authored directly here.
- `skills/`-- per-skill symlinks that resolve onward to
  `.github/skills/<name>` (see the chain below).  Not authored
  here.
- `agents/`-- project subagent definitions.  Authored directly
  here.

Tool-specific directories symlink here so every tool sees one
source of truth.  Skills are a two-hop chain because
GitHub-native skill discovery requires the authored `SKILL.md`
to live under `.github/skills/`; rules and agents have no such
requirement and are authored directly under `.agents/`:

| Symlink or link         | Target                        |
|-------------------------|-------------------------------|
| `.claude/skills`        | `../.agents/skills`           |
| `.agents/skills/<name>` | `../../.github/skills/<name>` |
| `.claude/agents`        | `../.agents/agents`           |
| `.claude/rules`         | `../.agents/rules`            |
| `CLAUDE.md` (each)      | `AGENTS.md` (same dir)        |

So a given skill resolves as
`.claude/skills/<name>` -> `.agents/skills/<name>` ->
`.github/skills/<name>/SKILL.md`.  `.agents/rules` and
`.agents/agents` are real directories, not symlinks-- their
files are the authored source, and `.claude/rules` /
`.claude/agents` are the only hop.

`AGENTS.md` files are lazy-loaded directory indices: each one
describes only its own directory and links onward.  Do not use
`@`-style file inclusion in them.
