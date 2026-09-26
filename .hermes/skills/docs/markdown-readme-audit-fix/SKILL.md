---
name: markdown-readme-audit-fix
description: "Use when fixing README markdown formatting and links. Audit the PlayForm/Aphrodite README + docs: lychee links, dash/quote normalization, emoji placement, backticking."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: docs
category_taxonomy: docs/markdown-readme-audit-fix
scope: README.md and docs/ markdown audit and manual fix in the Aphrodite repo; no scripted file edits
owns: the formatting/fix state of README.md, docs/, and .hermes/ authored docs
depends_on:
    - github-readme-generation
    - scripts/dash-normalizer.py
    - scripts/emoji-scanner.py
    - scripts/emoji-placement-check.py
supersedes: []
verification:
    source_of_truth: repo source tree (git ls-files, ls) and lychee output
mutation_level: manual, one location at a time (patch/write_file); scans are read-only
date: 2026-09-25
metadata:
    hermes:
        tags: [docs, auditing, fixing, normalizing, backticking, lycheeing]
        related_skills: [github-readme-generation]
status: active
---

# Markdown README Audit & Fix

Audit the Aphrodite README and docs against the source tree, then fix broken
links, unicode dashes/curly quotes, emoji placement, and backtick formatting.
Every edit is manual, one location at a time.

## When to Use

- The README or `docs/` files have broken local/relative links, stale
  directory trees, or paths that do not exist in the repo.
- Dash/quote normalization is needed. Smart Unicode characters are invisible
  in most editors, break diffing and search, and reveal AI-generated content
  (CLAIM: this skill gives no probe for those effects). The normalize-dashes
  hook is user-configured in `$HOME/.hermes/agent-hooks/` and wired via
  config.yaml `hooks:` for `write_file|patch|execute_code`; it is not
  guaranteed active in every session (CLAIM: statement about live config, no
  test here). This manual workflow is the reliable enforcement point.
- Emoji spacing/placement needs fixing, or technical terms need backticks.
- Lychee link verification before/after content changes.

## Hard constraint: NO script-based file edits

The user has prohibited script-based file editing (`sed`, `awk`,
bulk-patch scripts, mass-replace tools). Every file edit must be manual, one
location at a time, using the `patch` tool or the `write_file` tool.

- Never write a Python/SH script that opens, reads, and rewrites a file,
  because a scripted rewrite skips the per-location context check.
- Never run `sed`, `awk`, `perl -pi`, or similar over a set of files, because
  each location must be read with its surrounding context before it is
  patched.
- Never batch-apply patches from a generated list, because each location must
  be confirmed in context first.
- Scanning/analysis scripts (`grep`, the skill's Python linters) are allowed,
  because they read but never write. All scans in this skill are read-only by
  design.

## 1. Scope the file set

List only repo-authored `.md` files. `git ls-files` never lists submodule
contents, so vendored trees are excluded automatically:

```bash
git ls-files '*.md' ':!:vendor/**' ':!:plugins/aphrodite/**'
```

For web doc links, run lychee with `--offline` first to find broken local
paths before touching content.

## 2. Verify project structure trees against source

Never trust a README tree without checking, because the most common error in
repo READMEs is a stale or fictional directory tree. Build the ASCII tree from
the `ls` output, not from memory. The walk probe is in
`references/audit-maps-and-probes.md`.

- Rust crates: `mod.rs` in subdirs, or flat `*.rs` at root.
- Each dir line needs a `# comment` describing its role.

## 3. Bulk technical term backticking

Backtick technical terms in prose. The ordered `TECH_TERMS` list and the
`re.sub` probe are in `references/audit-maps-and-probes.md`. Order matters:
longer/more-specific terms FIRST, short terms LAST, because short terms match
inside longer words.

**CRITICAL - post-pass substring corruption check.** The auto-formatter
corrupts words containing short terms: `` `Wind`ows `` must be fixed back to
`Windows`; `` `Rest`art `` back to `Restart`. Always run the corruption sweep
in §9 after backticking.

## 4. lychee verification

After content changes:

```bash
lychee --no-progress --offline {files...}
```

Expected: `0 Errors`. Ignore errors from:

- Vendored upstream code (`vendor/headroom`, `vendor/rtk` submodules),
  because that content is not repo-authored.
- `target/` build output, because cargo generates it.
- The `plugins/aphrodite` submodule tree, because it is a separate repository.

## 5. Emoji placement & compound emoji (run BEFORE editing)

Run `scripts/emoji-scanner.py` before making any changes. All four types
(A/B/C/D) must read back as zero before the pass is complete. The type matrix,
the before/after sub-patterns, and the NOT-errors leave-as-is list are in
`references/emoji-placement.md`.

Universal rule: every emoji is preceded by an em-quad (`&#x2001;`) and appears
at the end of a heading, sentence, or labeled item. Never place an emoji
before the prose it decorates, because decoration belongs at the right end of
the text segment. The manual fix is always the same two-step rewrite:

1. Pick up the emoji and any preceding em-quad from wherever it sits before
   the prose.
2. Place it at the right end of the text, preceded by a single em-quad.

In mermaid subgraph titles and node labels, emoji must still appear on the
right, preceded by an em-quad; the only difference from prose is that the
label is quoted. Never omit the em-quad inside mermaid quotes, because it
separates the label text from the emoji decoration. Examples are in
`references/emoji-placement.md`.

Check the whole file before and after: run `scripts/emoji-scanner.py`, confirm
the count dropped by the number of locations edited, and confirm zero false
removals from diagrams/tables. `scripts/emoji-placement-check.py` verifies
emoji-left-of-text in prose is clean.

## 6. Character normalization - dashes & curly quotes

Normalize typographically "smart" Unicode characters to plain ASCII, because
they are invisible in most editors, break diffing and search, and reveal
AI-generated content (CLAIM: this skill gives no probe for those effects).
The normalize-dashes shell hook (user-configured in `$HOME/.hermes/agent-hooks/`,
wired via config.yaml `hooks:` for `write_file|patch|execute_code`) may
already normalize the dash set on files the agent writes; this scan-and-patch
workflow is the enforcement point for files written outside the agent (manual
edits, imported content) and for the full 24-char set the hook does not cover.

### 6a. Minimal character map

The 6-row map (U+2014, U+2013 → `-`; U+2018, U+2019 → `'`; U+201C, U+201D →
`"`, with the U+2014 context rule) is in `references/audit-maps-and-probes.md`.
Use it for every manual dash/quote edit.

### 6b. Full regex set (24 characters)

The former normalize-dashes hook applied this broader set to every file
written by the agent. The live hook covers only the common dash codepoints,
the U+2010-U+2015 range (CLAIM: statement about live config; no test here).
Use this full set as the scan set for documentation audits. The
24-code-point list is in `references/audit-maps-and-probes.md`. Run
`python3 scripts/dash-normalizer.py --full` to scan with the complete set.

### 6c. Workflow

1. **Scan first** - run `scripts/dash-normalizer.py` in check mode to list
   all instances. `--check` = minimal 6-char set, exit 1 if any found;
   `--full` = 24-char set; `--check --strict mode=repo` = exclusion presets.
   The script has no write mode - read-only by design, consistent with the
   manual-edit constraint.
2. **Exclude generated/vendor content** (see 6d).
3. **Edit manually, one file at a time** - read each line's surrounding
   context, then `patch`.
4. **Re-run check mode** to verify zero remaining instances in authored files.

### 6d. Where to patch vs where to skip

**Patch:** `README.md`, `docs/` (install/config/api/proxy/architecture/plugin/
guides/tool-relay), `.hermes/` authored docs (release notes, notes, skills).

**Skip:**

- `vendor/` - vendored submodule checkouts (`headroom`, `rtk`), because that
  content is not repo-authored.
- `target/` - cargo build output, because cargo regenerates it.
- `plugins/aphrodite/` - the plugin submodule, because it is a separate
  repository (Aphrodite-Hermes).
- `node_modules/` - third-party packages, because they are not repo-authored.

**Mermaid subgraph heading exception:** in mermaid subgraph label strings, the
em dash `-` is a visual grouping separator between the element name and its
subtitle. Replace it with a single space ` `. Do not replace it with a hyphen.
`subgraph "Proxy - Cache"` → `subgraph "Proxy Cache"`.

**Workflow rule:** each file is edited individually - read the surrounding
context for every location, then patch. Do NOT batch-apply fixes without
reading each file to confirm the context first, because a fix applied without
context can land in the wrong location.

## 7. Standard README structure (Aphrodite conventions)

The repo README follows the restyled convention (see `github-readme-generation`
for authoring; this skill enforces it during fixes). The canonical shape block
is in `references/readme-structure-template.md`:

- **Title is a link.** A bracketed title `# [Name]` resolves via a
  link-reference definition at the bottom (`[Name]: <repo URL>`).
- **Emoji headers use an EM QUAD (U+2001), not a space:** `## Text emoji`.
  Verify with a byte check (`\xe2\x80\x81`). A normal space is not
  sufficient.
- **Badges use `/static/v1`.** Never use `/badge/`, because `/badge/` breaks
  when the badge message contains a `/`. Each badge is clickable (`<a href>`
  wrapper); the license badge shows the actual license.
- **GitHub markdown alerts:** `> [!TYPE]` on its own line, then a blank `>`
  line, then the body. Never merge the tag with body text on one line,
  because the alert renders only in that three-part shape.
- **Code fences are labeled** with a bold source/file callout before the
  block.
- **Hero images are centered** with `<p align="center">…</p>`, sized for wide
  captures, descriptive `alt`.
- **Contributing/License are short link-style headers** backed by real files
  (`CONTRIBUTING.md`, `LICENSE`), never full prose in the README.

## 8. Content style rules

- Prose paragraphs over bullet lists for Key Features.
- Technical terms backticked in prose: `Aphrodite`, `CCR`, `dylib`,
  `ctypes`, `Hermes`; file names and paths backticked: `aphrodite.toml`,
  `plugins/aphrodite/`, `crates/aphrodite/src/`; table cells with code use
  backticks.
- **Em quad `&#x2001;` in section headers** - every heading with an emoji has
  an em-quad immediately before the emoji. Three variants exist in the wild:
  actual `\u2001` (U+2001) in `README.md`, `&#x2001;` in docs, and (in error)
  `&nbsp;` in some older headers; standardize on the file-native form.
- **No compound emoji splitting** - Fitzpatrick skin-tone modifiers (🏻-🏿)
  must sit immediately adjacent to their base emoji. Never allow any spacing
  character (em-quad, `&nbsp;`, or regular space) between a base like `💪`,
  `🙏`, `🪱`, `👌`, `✊`, `👉` and its modifier, because the pair must be a
  single code-point cluster with zero width between them.
- **Emoji on the RIGHT of all text segments** - emoji belong at the end of a
  heading, sentence, or labeled item, separated from the preceding text by an
  em-quad. Never place them before the prose they decorate, because
  decoration belongs at the end of the segment.
- `https://` absolute URLs preferred for links.
- Repo links: `github.com/PlayForm/Aphrodite/tree/Current/...` for the
  parent. `github.com/PlayForm/Aphrodite-Hermes` is the plugin's own
  repository. It is not a tree inside the parent.

## 9. Post-pass corruption scan (ALWAYS after backticking)

The bulk formatter injects backticks indiscriminately. Run this sweep after
backticking, because the pass corrupts words containing short terms and URLs.
The patterns dict, the real-corruption fix table, and the "do not flag" case
are in `references/audit-maps-and-probes.md`. Do NOT flag ``[`url`][ref]``,
because that is intentional markdown link-text formatting.

## Related: authoring vs fixing

When authoring new README content (badges, hero images, install sections,
release-note structure), use the `github-readme-generation` skill; this skill
is the fix/enforcement pass on top of the same conventions.

## Stop if

- Stop if a target file is under `vendor/`, `target/`, `plugins/aphrodite/`,
  or `node_modules/`. Do not patch it, because that content is vendored,
  generated, or owned by a separate repository.
- Stop if the next action is a scripted or bulk edit instead of one `patch`
  per location. The only legal next act is a manual single-location edit.
- Stop if a README tree entry has no match in the source tree. Rebuild the
  tree from the `ls` output before continuing.
- Stop if `python3 scripts/dash-normalizer.py --check` still finds instances
  after editing. The pass is not complete until check mode exits 0.

## Recovery

- Re-read the surrounding context with `read_file`, then apply one `patch`
  per exact `old_string`. If the file changed on disk since the read (the
  external auto-committer commits continuously), re-read and retry once;
  do not re-apply from memory.
- Re-run `scripts/emoji-scanner.py` and `scripts/emoji-placement-check.py`.
  Zero emoji-left-of-text and a count that dropped by exactly the number of
  edited locations is the exit condition.

## Claim-to-test table

| Claim                                    | Test                                                                                                                     |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| The file set is repo-authored only       | `git ls-files '*.md' ':!:vendor/**' ':!:plugins/aphrodite/**'` lists no vendored paths                                   |
| No broken local links remain             | `lychee --no-progress --offline {files...}` reports `0 Errors`                                                           |
| No smart dashes/quotes in authored files | `python3 scripts/dash-normalizer.py --check` exits 0; `python3 scripts/dash-normalizer.py --full` reports zero instances |
| No emoji-left-of-text in prose           | `scripts/emoji-placement-check.py` reports zero                                                                          |
| Emoji count matches the edits            | `scripts/emoji-scanner.py` count dropped by exactly the number of edited locations                                       |
| Emoji headers use em-quad, not space     | byte check finds `\xe2\x80\x81` before each emoji header                                                                 |
| No backtick corruption after backticking | run the §9 patterns over all touched files; zero matches                                                                 |
| Base emoji and modifier form one cluster | `od -c` on the line shows no spacing character between them                                                              |
