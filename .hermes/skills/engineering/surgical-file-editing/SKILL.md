---
name: surgical-file-editing
description: "Use when patch mangles whitespace on multi-line edits in the Aphrodite monorepo (crates, plugin Python, .hermes docs)."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: engineering
category_taxonomy: engineering/surgical-file-editing
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, scripting, byte-checking, resolving, crlf-editing]
        related_skills: [git-operations, submodule-fleet-management]
status: active
---

# Surgical File Editing

Make exact, verifiable file edits when the interactive patch tool's fuzzy
matcher fights you, and resolve multi-region edits (merge conflicts,
CRLF/tab files) without silent corruption. The patch tool is the default for
single clean edits; this skill is the fallback ladder and the discipline that
keeps surgical edits honest.

## When to use

- The `patch` tool re-indents or re-spaces a block instead of applying your
  exact whitespace (observed on CRLF + tab-indented Python in
  `plugins/aphrodite/`: it ADDS indentation when asked to remove it).
- Multi-region edits in one file (merge-conflict resolution, mechanical
  rewrites across `crates/`, `plugins/`, `.hermes/skills/`).
- Files with CRLF line endings, or files whose line endings got mixed by a
  git merge.
- JSONC files: .json files containing `//` comments that patch/write_file
  refuse with a strict-JSON syntax-validation error before any edit lands -
  the guard fires on the comments (a pre-existing condition), not on your
  edit, and the scripted literal-replace fallback (Section 3) is the ONLY
  path, not a second choice.
- `.hermes/**/*.md` edits where the repo's `npx prettier --check` gate must
  stay green.

## Procedure

### 1. Detect line endings first

`file <path>` - CRLF / LF / mixed. Git merges can produce mixed endings
(e.g. every line CRLF except the last, appended line LF). A byte-exact
script written for CRLF fails with a zero-count assertion on the LF line;
re-read the exact bytes before writing the pattern.

### 2. Try the patch tool; bail after two mangles

Small unique single-line edits patch cleanly. When a multi-line region gets
re-indented instead of corrected (the matcher re-spaces from surrounding
context), stop after two attempts - the third attempt repeats the first. Do
not loop.

### 3. Script the edit with exact bytes and count assertions

Write a real Python script (scratch dir, never inline `python -c`):

```python
from pathlib import Path
CR = "\r\n"
text = p.read_bytes().decode("utf-8")
old = old.replace("\n", CR)   # preserve CRLF
new = new.replace("\n", CR)
assert text.count(old) == 1, f"expected 1 occurrence, found {text.count(old)}"
text = text.replace(old, new)
p.write_bytes(text.encode("utf-8"))
```

- `assert count == 1` per replacement: a stale/wrong pattern fails loudly
  instead of silently skipping the edit.
- One coherent pass per script; fix defects with follow-up scripts. Never
  re-run the original after partial success - its anchors (marker blocks)
  are gone.
- For merge-conflict regions, replace from the `<<<<<<<` line through the
  `>>>>>>>` line INCLUSIVE of its terminator: `end = text.index(">>>>>>>",
start); end_line = text.index(CR, end) + len(CR)`.

### 4. Trailing-newline discipline (the glue bug)

If the text FOLLOWING a replaced block is code - not a blank line - the
replacement must end with a trailing newline. A missing one glues the next
line onto the last replaced line, and when that last line is a `#` comment
the glued code is silently swallowed into the comment: **py_compile still
passes**. Regions followed by a blank line are safe without it.

### 5. Verify beyond the compiler

After any multi-region edit:

1. `grep -n` for glued patterns (comment text immediately followed by code
   tokens).
2. `py_compile` (syntax only - misses comment glues).
3. Run the repo's targeted gates: `cargo fmt --check` (Rust - also
   normalizes matcher re-indentation), `cargo test -p <crate>` for touched
   crates, `ruff check plugins/aphrodite/` (plugin Python), `npx prettier
--check .hermes/**/*.md` (docs).
4. After conflict resolution: `grep -n "^<<<<<<<\|^>>>>>>>"` must return
   nothing.

## Merge-conflict resolution rules

- **Parallel-feature conflicts**: when both sides implemented the same
  feature, compare the two sides per region, take the SUPERSET implementation
  (more tests, more complete behavior), keep the more accurate documentation
  from the other side, and merge the two rather than preferring one side
  wholesale.
- **Dedupe doubled additions**: both sides adding the same thing produces
  doubled lines that do NOT conflict (guard blocks, tests, config entries,
  env-var pops in setUp). After resolution, grep the file for each doubled
  line and remove the extra.
- **Mixed-endings duplication**: a merge can leave the duplicate as the only
  LF line; the dedupe pattern must match its exact terminator.
- **Bookkeeping**: leave the resolved files in the working tree - the repo's
  auto-committer/sweeper completes the merge commit; never commit from an
  agent workflow (Aphrodite repo convention). Verify the branch after:
  `git symbolic-ref -q HEAD`.

## Pitfalls

- Mixed tab/space Python files: the fuzzy matcher can re-indent an inserted
  block into the surrounding style (or splice tabs into a spaces file and
  vice versa) and the returned diff still looks right - the module then
  fails with IndentationError at import, not at patch time. After any
  multi-line patch on a mixed-indentation file, import the module (or run
  its tests) before declaring done. For Rust, `cargo fmt --check`
  deterministically normalizes matcher re-indentation - run it after
  multi-line patches.
- Never fight the fuzzy matcher more than twice - script it; the matcher's
  whitespace normalization is not a stable transform for multi-line blocks.
- Anchor lines you mean to KEEP must be re-emitted in new_string: an
  old_string that ends with a line you only included to locate the region
  gets consumed (and deleted) when new_string omits it - observed twice as a
  silently truncated docstring opening. Keep old_string minimal: only the
  exact text being replaced; when you need context, put it on the new_string
  side or verify the region with read_file after every patch (a patch that
  reports a suspiciously small diff - e.g. +1/-1 on a 30-line intended
  change - is a clipped edit; re-read before continuing).
- The glue bug is invisible to compilers - always grep for glue and run
  tests.
- CRLF is not a cosmetic detail: a byte-exact script that forgets `\r\n`
  fails the count assertion; a patch tool edit can silently rewrite a file's
  endings.
- Backslash-heavy shell content: the patch tool writes `\` verbatim from
  new_string (and the fuzzy matcher can double it), so a `\echo`/`\sed`-
  prefixed bash line can land as `\\echo` - bash then treats it as a command
  literally named `\echo` and the script fails "command not found" at
  runtime, after the diff looked right. For files that lean on `\command`
  prefixes (or any dense-backslash content), rewrite the whole file with
  write_file instead of patching; if a script silently misbehaves after a
  patch, verify the critical lines with `od -c` before debugging the logic.
- A resolver that replaces whole marker blocks must know what follows each
  block: code-followed blocks need the trailing newline, blank-followed
  blocks do not.
- Shell scripts must stay executable: a rewrite via write_file preserves the
  mode, but verify with `ls -l` after any rename/recreate (repo convention:
  `scripts/` are runnable).
