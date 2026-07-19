# Agent Framework Directory

This directory holds the project's agentic AI coding framework:

- `rules/`-- always-applicable project rules (style, testing,
  commits).  Referenced from `AGENTS.md` index files; never
  inlined into them.
- `skills/`-- project skills (workflows an agent can invoke).
- `agents/`-- project subagent definitions.

Tool-specific directories symlink here so every tool sees one
source of truth:

| Symlink            | Target            |
|--------------------|-------------------|
| `.claude/skills`   | `../.agents/skills` |
| `.claude/agents`   | `../.agents/agents` |
| `.claude/rules`    | `../.agents/rules`  |
| `CLAUDE.md` (each) | `AGENTS.md` (same dir) |

`AGENTS.md` files are lazy-loaded directory indices: each one
describes only its own directory and links onward.  Do not use
`@`-style file inclusion in them.
