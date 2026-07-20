# Agent Framework Directory

This directory holds the project's agentic AI coding framework.
Project rules, skills, and agents are all authored directly here:

- `rules/`-- always-applicable project rules (style, testing,
  commits).  Referenced from `AGENTS.md` index files; never
  inlined into them.
- `skills/`-- project skills, one directory per skill with its
  `SKILL.md`.
- `agents/`-- project subagent definitions.

Tool-specific directories symlink here so every tool sees one
source of truth:

| Symlink or link    | Target              |
|--------------------|---------------------|
| `.claude/skills`   | `../.agents/skills` |
| `.claude/agents`   | `../.agents/agents` |
| `.claude/rules`    | `../.agents/rules`  |
| `CLAUDE.md` (each) | `AGENTS.md` (same dir) |

All three `.agents/` subdirectories are real directories; the
`.claude/` entries are the only hop.  There is no `.github/skills`
directory: GitHub Copilot discovers `.agents/skills/` directly, so
no Copilot-specific copy or link exists.

`AGENTS.md` files are lazy-loaded directory indices: each one
describes only its own directory and links onward.  Do not use
`@`-style file inclusion in them.
