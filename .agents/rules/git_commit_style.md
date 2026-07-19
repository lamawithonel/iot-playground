---
# Applies to every commit, not to specific files.
---

# Git Commit Style Guide

- One logical change per commit
- Use Conventional Commits format: `type(scope): subject`
- Use Markdown formatting for body if needed (e.g., lists, code blocks)
- Present tense: "add" not "added"
- Imperative mood: "fix bug" not "fixes bug"
- Reference issues: `Closes: #123`, `Refs: #456`
- Keep headline concise, under 50 characters if possible, no more than 72
- The body should focus on the "why" and "how" more than the "what" (which
  should be in the headline)
- Wrap text at 72 characters except for URLs
- Use Git trailers for metadata and references: `Co-authored-by:`, `See-also:`,
  `Tested-on:`, etc.
