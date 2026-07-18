---
paths:
  - "**/*.sh"
  - ".mise/tasks/**"
  - "test/scripts/**"
---

# Shell Script Style Guide

- Wrap comments at the first word extended beyond 72 characters, and do
  not exceed 80 characters.  Wrap before 72 characters if the last word
  would extend beyond 80 characters.  Exceptions: URLs, long paths, and
  code examples.
- Prefer POSIX syntax over Bash- or Zsh-specific syntax, except in the following
  cases:
    - Where shell-specific features are faster to execute, e.g.,
       `[[ "abc123" =~ c1 ]]` instead of `echo abc123 | grep -q c1`.
    - Where shell-specific features are significantly more readable, e.g.,
       `<<-` heredocs with indented content and `source` instead of `.`.
- Use shell-specific features that add safety, e.g., `set -o pipefail`, `local`,
  `readonly`, `typeset`, etc.
- Quote strings with 'hard quotes' unless variable expansion is needed.
- Enclose all variables in curly braces when they are part of a larger string,
  e.g., `"this ${string}"`, but not when they are a standalone, e.g.,
  `"$solitary_variable"`.
- Use `UPPER_SNAKE_CASE` for variables intended to be used across multiple
  functions, e.g., configuration variables.
- Use `_underscore_lead_lower_snake_case` for file-local variables and
  functions, regardless of whether they are declared with `local` or not.
- Use `lower_snake_case` for functions intended to be used across multiple
  files, e.g., utility functions.
- Unset unneeded functions and variables not needed after use, e.g., temporary
  variables and helper functions.
- Batch `export` and `unset` calls as a micro-optimization-- speed matters!
- Use XDG Base Directory Specification wherever possible.
- Use tabs for indentation.  Spaces may be used after tabs for `<<-` heredoc
  content, e.g., to indent the output file with its native indentation, but
  the initial indentation must be tabs.
- Always use `set -o <option>` for shell options, one per line.
  - `errexit` must be the first option if used.
  - POSIX-compatible options come second (e.g. `nounset`, `noexec`).
  - Common non-POSIX options come third (e.g. `pipefail`).
  - Bash-specific options come last.
