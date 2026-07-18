# docs/ -- Documentation Site (mdBook)

mdBook sources in `src/`, built to gitignored `book/`
(`mise run docs` to serve, `mise run docs:build` to build).
Deployed to GitHub Pages on push to `main` via
`.github/workflows/deploy-docs.yml`.

## Contents

| Path | Purpose |
|------|---------|
| `src/SUMMARY.md` | Navigation -- update it when adding any page |
| `src/roadmap.md` | Phases, milestones, memory budget (living doc) |
| `src/system_requirements.md` | IEEE 29148-style SRS (framework + feather board) |
| `src/risk_register.md` | Canonical risk list (R1..) |
| `src/architecture/decisions.md` | Canonical ADR log (ADR-001..) |
| `src/development/testing.md` | Five-layer test pyramid detail |
| `src/projects/` | Per-project docs (ARS toolhead sensor, ...) |

## Local Rules

- Prose follows
  [`markdown_style.md`](../.agents/rules/markdown_style.md).
- `decisions.md` and `risk_register.md` are the single sources of
  truth for ADRs and risks; other documents link to them instead
  of copying content.  New ADRs append sequentially (next free
  number) using the template at the end of `decisions.md`.
- After editing, verify the build: `mise run docs:build`.
