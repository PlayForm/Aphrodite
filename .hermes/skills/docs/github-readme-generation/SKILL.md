---
name: github-readme-generation
description: "Use when generating, editing, or restyling PlayForm/Aphrodite READMEs. Claim verification against the release ledger, restyle checks (em-quads, GFM alerts, badges), real-output examples."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos, linux]
category: docs
category_taxonomy: docs/github-readme-generation
date: 2026-09-25
metadata:
    hermes:
        tags: [docs, generating, restyling, verifying, byte-checking, prettiering]
        related_skills: [markdown-readme-audit-fix]
status: active
---

# GitHub README Generation (Aphrodite)

The Aphrodite README is hand-authored in-repo - there is no generator
pipeline. Source of truth: `README.md` at the repo root plus the release
ledger (`plugin.yaml`, `BINARY_VERSION`, crate manifests, README badge row)
for version claims. When a claim drifts, fix the README and the ledger
together; never treat a badge as authoritative.

## When to Use

- Restyling the Aphrodite README to match a reference README's conventions:
  em-quad (U+2001) emoji headers, GFM alerts, `/static/v1` badges wrapped in
  `<a href>`, labeled code fences, centered hero images, short
  Contributing/License link sections.
- Editing README version claims - verify against the release ledger first
  (see the verification flow below).
- Regenerating a real-output example - run the actual binary and embed its
  output verbatim in a GFM `diff` fence.

## Restyling hand-authored READMEs (verification flow)

Applies to any README restyle that mimics a reference README's style and adds
"examples of what it exactly accomplishes" - not just pipeline-generated ones.

- **Verify every claim against the current source before restyling.** Counts
  and names drift: an old README's "62 built-in extensions" was 53 in the
  source, "installed binaries psummary/Summary" were actually Summary/PSummary,
  "clap derive" was the builder API, and two declared dependencies were unused.
  Grep the source for each number/name/flag the README asserts, and run
  `--help`/`--version` on the real binary rather than trusting the old block.
- **For real-output examples, run the tool live and embed its output verbatim**
  in GFM `diff` fences. Save the run to scratch, then python-compare the README
  fences byte-for-byte against the captured lines before finishing - a fence
  that is close but not exact is a misquote.
- **`write_file` strips trailing whitespace**: a diff line that is exactly `- `
  or `+ ` (a removed/added blank line) comes back as `-`/`+`, no longer
  byte-faithful. Restore the trailing space with a `patch` whose old/new
  strings carry it explicitly, then re-verify.
- **Final sweep**: em-quad byte count (`\xe2\x80\x81`) equals the emoji-heading
  count; every GFM alert tag sits alone on a `>` line followed by a blank `>`
  then the body; badge URLs return HTTP 200 (`curl -s -o /dev/null -w
"%{http_code}"`); the bracketed title link-reference resolves; every named
  asset exists relative to the repo root.

## Restyle depth: prettier, em-quads, wide tables

Restyle rules that survive contact with the toolchain. The content-verification
workflow is the section above; `scripts/readme-verify.py` runs the mechanical
checks and `scripts/emquad_fix.py` repairs em-quad separators.

- **Scope the fleet before touching anything.** When the target is a submodule
  README inside a monorepo, confirm it is the project's OWN repo first:
  `git -C <sub> remote -v` - an `origin` pointing at a non-org upstream marks
  a FORK (skip it; forks keep their own conventions); only the org's `Source`
  remote means fair game.
- **Drift-prone facts and their authoritative sources:** version / hook-count /
  tool-count / classifier size → the MANIFEST (`plugin.yaml`, `Cargo.toml`),
  never the README badge; config example values → the config schema AND the
  LIVE runtime config (schema examples differ from shipped/live values);
  loader/script line counts → `wc -l`; API/health response field names → the
  consumer code that parses them. Count pattern lists (binary extensions,
  omit regexes) from the binary's own `--help` defaults, never a hand-rolled
  regex over source - backslash-escaped metacharacters and multi-part
  extensions undercount.
- **Completeness is the bar, not a representative subset.** "Refill/adapt the
  README" means the FULL catalog: every output type with its real preview
  shape, the architecture tree expanded to every real module, a pipeline
  diagram covering all variants - a diagram showing 7 of 21 types misses the
  bar.
- **Prettier ignoring your target `.md`? Fix `.prettierignore` with gitignore
  negation ordering, not a bare `!file`.** A single `!foo.md` inside an
  excluded directory is a NO-OP (the directory exclusion wins; prettier never
  descends). Per directory level the working shape is: un-exclude the dir
  (`!plugins/`), un-exclude the subdir, re-exclude everything, then
  re-include the target LAST.
- **Verify an ignore fix with prettier's own oracle, not `--check`.**
  `--check` drops ignored files SILENTLY (a "checked" verdict can mean
  "skipped"); the authoritative probe is
  `getFileInfo(path, {ignorePath: "<repo>/.prettierignore"})` expecting
  `{ignored: false, inferredParser: "markdown"}`. `--no-ignore` is not a real
  prettier flag.
- **Wide tables break GitHub rendering - expand them into per-row blocks,
  never bullets.** Prettier pads every column to the widest cell, so one long
  cell inflates the whole row into a horizontally-scrolling monster. The
  accepted shape for any cell over ~140 chars: one result after another,
  separated by `---` BETWEEN rows (never after the last); each row a block of
  `**Label**: value` fields; code/output/preview cells in FULL untruncated
  fences; before/after pairs as two labeled stacked code blocks. Bullets are
  NOT an acceptable substitute.
- **Pipes in table cells must be escaped `\|` even inside backticks.** GFM
  treats a pipe inside a code span as a cell delimiter - backticks never
  protect `|`. The canonical form is `` `[build:1E 1W 142L \| error[...]]` ``.
  `<code>` tags are NOT the fix: prettier's table pass reflows/pads the cell
  and splits the display at the pipe, rendering a giant gap on GitHub.
- **Escaped `\|` is the formatter-stability test.** The pinned prettier
  provably leaves `\|` untouched (a `--write` on a `\|` table reports
  `(unchanged)`), so canonical tables are formatter no-ops. A table whose
  pipes get unescaped and re-padded after a formatter run was mangled by a
  NON-repo formatter (IDE markdown formatters do exactly this: unescape `\|`,
  re-pad cells) - restore `\|` everywhere and the file stops breaking.
- **Mangled-table signatures to scan for**: a raw pipe inside a code span
  padded with spaces (`[build:1E 1W 142L<+spaces>| error`), or a whole table
  collapsed onto ONE line with literal `\n` sequences between rows
  (formatter/export flattening newlines to `\n` text).
- **Repair a literal-`\n`-collapsed table with an assert-first literal-replace
  script**: split the mangled line on the `\n` separator, strip leading
  whitespace before each `|`, re-emit real lines - but assert the expected
  shape BEFORE writing (exactly one mangled line; every segment starts with
  `|`; expected row count). A worktree file can be regenerated between your
  read and the script run (open editor saving, an auto-committer committing a
  refresh): a failed assertion means re-inspect the NEW state, never force
  the write. Verify `prettier --check` after.
- **A regeneration that "fixes formatting" can silently drop content
  coverage** - after any refresh/regeneration, compare the regenerated row or
  file set against the actual tree (count classified rows vs files on disk).
  Prettier-clean AND committed is not completeness; a thin table is a content
  regression.
- **Consecutive `**Label**: value` lines are ONE markdown paragraph.** No
  blank line between them joins them onto the same rendered line;
  blank-line-separate every labeled field. Detect with the `**Label:**`
  pattern (substring `:** `), not `**: `.
- **Backtick balance is mandatory when content moves out of tables** - every
  code fence verified closed (``` per block even); stray single backticks
  break rendering once unwrapped. Check after EVERY edit.
- **Never author markdown as comment-style `#`-prefixed prose lines.**
  Prettier treats `#` as ATX heading syntax and mangles them into stray
  headings; author real markdown (plain paragraphs, real `##`/`###` headings,
  standard `-` bullets) so prettier is idempotent - a second `--write` reports
  every file `(unchanged)`.
- **The patch/write_file transport may silently remap U+2001 (EM QUAD) to
  U+2003 (EM SPACE) - and it is INCONSISTENT** (the same U+2001 input can
  survive one write and be remapped on the next). Never trust the write;
  byte-verify the U+2001 count after EVERY write/patch that touches headings
  and re-run `scripts/emquad_fix.py` (line-based, regex-free; handles BOTH a
  trailing emoji run `## Install ⚡` and a mid-line run
  `# Aphrodite 💋 Hermes Plugin`; emoji detection must include VS16 variants
  like \u2699\ufe0f).

## Related skills

- `markdown-readme-audit-fix` - manual formatting/emoji/backtick fixes for
  hand-authored READMEs (lychee links, dash/quote normalization)
