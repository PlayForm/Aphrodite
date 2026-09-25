---
name: markdown-readme-audit-fix
description: "Use when fixing README markdown formatting and links. Audit the PlayForm/Aphrodite README + docs: lychee links, dash/quote normalization, emoji placement, backticking."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos, linux]
category: docs
category_taxonomy: docs/markdown-readme-audit-fix
date: 2026-09-25
metadata:
    hermes:
        tags: [docs, auditing, fixing, normalizing, backticking, lycheeing]
        related_skills: [github-readme-generation]
status: active
---

# Markdown README Audit & Fix

Audit the PlayForm/Aphrodite README and docs against the source tree and fix
broken links, unicode dashes/curly quotes, emoji placement, and backtick
formatting - manually, one location at a time.

## When to Use

- The README or `docs/` files have broken local/relative links, stale
  directory trees, or paths that don't exist in the repo.
- Dash/quote normalization is needed (smart Unicode characters break diffing,
  search, and reveal AI-generated content) - the normalize-dashes hook is
  user-configured (`~/.hermes/agent-hooks/`, wired in config.yaml) and NOT
  guaranteed active in every session, so this manual workflow is the reliable
  enforcement point.
- Emoji spacing/placement needs fixing, or technical terms need backticks.
- Lychee link verification before/after content changes.

## ⚠️ Hard constraint: NO script-based file edits

**The user has explicitly prohibited script-based file editing (sed, awk,
bulk-patch scripts, mass-replace tools). Every file edit must be manual, one
location at a time, using the `patch` tool or `write_file` tool.**

- ❌ Never write a Python/SH script that opens, reads, and rewrites a file.
- ❌ Never run `sed`, `awk`, `perl -pi`, or similar over a set of files.
- ❌ Never batch-apply patches from a generated list.
- ✅ Read surrounding context for each location (read_file), then one `patch`
  per exact old_string.
- ✅ Scanning/analysis scripts (grep, the skill's Python linters) are fine -
  they read but never write. All scans in this skill are read-only by design.

## 1. Scope the file set

List only repo-authored `.md` files. `git ls-files` never lists submodule
contents, so vendored trees are excluded automatically:

```bash
git ls-files '*.md' ':!:vendor/**' ':!:plugins/aphrodite/**'
```

For web doc links, run lychee with `--offline` first to find broken local
paths before touching content.

## 2. Verify project structure trees against source

**Never trust a README tree without checking** - the #1 error in repo READMEs
is a stale or fictional directory tree.

```python
import os
src = "crates/aphrodite/src/"
for root, dirs, files in os.walk(src):
    dirs.sort()  # deterministic order
```

Build the ASCII tree from the `ls` output, not from memory:

- Rust crates: `mod.rs` in subdirs, or flat `*.rs` at root.
- Each dir line needs a `# comment` describing its role.

## 3. Bulk technical term backticking

```python
# Order matters: longer/more-specific terms FIRST, short terms LAST
TECH_TERMS = [
    "aphrodite", "Aphrodite", "BLAKE3", "Hermes", "ctypes", "dylib",
    "SQLite", "Tokio", "WebAssembly",
    "Rust", "JSON", "clippy", "ruff",  # short terms LAST (won't corrupt substrings)
]

for line in prose_lines:
    for term in TECH_TERMS:
        if f'`{term}`' in line:
            continue  # already formatted
        line = re.sub(r'(?<!\w)' + re.escape(term) + r'(?!\w)', f'`{term}`', line)
```

**CRITICAL - post-pass substring corruption check.** The auto-formatter
corrupts words containing short terms: `` `Wind`ows `` → fix back to `Windows`;
`` `Rest`art `` → `Restart`. Always run the corruption sweep in §9 after
backticking.

## 4. lychee verification

After content changes:

```bash
lychee --no-progress --offline {files...}
```

Expected: `0 Errors`. Ignore errors from:

- Vendored upstream code (`vendor/headroom`, `vendor/rtk` submodules).
- `target/` build output.
- The `plugins/aphrodite` submodule tree.

## 5. Emoji placement & compound emoji (run BEFORE editing)

Run `scripts/emoji-scanner.py` before making any changes. All four types must
read back as zero before the pass is complete.

| Type   | Pattern                                                                   | Fix                                                                     |
| ------ | ------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| TYPE A | Regular ASCII space before emoji in `# heading`                           | Replace ` ` with file-native em-quad (`\u2001` or `&#x2001;`)           |
| TYPE B | Double em-quad (two in a row: `&#x2001;&#x2001;`)                         | Collapse to single `&#x2001;`                                           |
| TYPE C | Em-quad between base emoji and Fitzpatrick modifier (e.g. `🙏&#x2001;🏿`) | Remove the quad: `🙏🏿`                                                   |
| TYPE D | Emoji appears on the LEFT of prose text                                   | Move emoji to the RIGHT end of the text segment, preceded by an em-quad |

### TYPE D - emoji-left-of-text (most widespread)

Universal rule: every emoji is preceded by an em-quad and appears **at the
end** of a heading, sentence, or labeled item - never before the prose it
decorates.

| Sub-pattern         | Before (wrong)                                 | After (correct)                                |
| ------------------- | ---------------------------------------------- | ---------------------------------------------- |
| Link/markup line    | `&#x2001;📖 **[CCR Protocol Docs](...)**`      | `**[CCR Protocol Docs](...)**&#x2001;📖`       |
| Bold-intro sentence | `Welcome to **Aphrodite**💋, the lightweight…` | `Welcome to **Aphrodite**, the lightweight…💋` |
| Heading title       | `## \`CCR\`💋 in \`Aphrodite\`🌐 Ecosystem`    | `## \`CCR\` in \`Aphrodite\`🌐 Ecosystem💋`    |

Manual TYPE-D fix - always the same two-step rewrite:

1. Pick up the emoji and any preceding em-quad from wherever it sits before the
   prose.
2. Place it at the right end of the text, preceded by a single em-quad.

```
Wrong:  <text> <emoji> <more_text>
Right: <text> <more_text> <em-quad> <emoji>
```

**NOT errors - leave as-is:**

- Table cell status rows: `| Status | 📝 Specified …` (status icon inside cell).
- Ecosystem heading combos: `&#x2001;🍃 + &#x2001;🌐` - the _second_ emoji is
  furthest-right; reorganize so all decorative emoji are after the text.
- Lines where the emoji is the entire content: `<h3>&#x2001;💋</h3>` or
  standalone `&#x2001;🌐`.
- Some files use the actual U+2001 character on _both_ sides of the emoji -
  deliberate; confirm with `od -c` before changing anything.

### Mermaid subgraph/node labels

In mermaid subgraph titles and node labels, emoji must still appear on the
**right**, preceded by an em-quad. The only difference from prose is the label
is quoted.

| Sub-pattern    | Before (wrong)                 | After (correct)               |
| -------------- | ------------------------------ | ----------------------------- |
| Subgraph title | `"&#x2001;⛰️ Proxy - Backend"` | `"Proxy - Backend&#x2001;⛰️"` |
| Node label     | `"&#x2001;🚀 Bootstrap/…"`     | `"Bootstrap/&#x2001;🚀"`      |

The em-quad is still meaningful inside mermaid quotes - it separates label text
from the emoji decoration. Do not omit it. The only forbidden placement is
emoji on the LEFT of the text it decorates.

Check the whole file before and after: run `scripts/emoji-scanner.py`, confirm
the count dropped by the number edited, and confirm zero false removals from
diagrams/tables. `scripts/emoji-placement-check.py` verifies emoji-left-of-text
in prose is clean.

## 6. Character normalization - dashes & curly quotes

Normalize typographically "smart" Unicode characters to plain ASCII. These
characters are invisible in most editors but break diffing and search, and
reveal AI-generated content. The normalize-dashes shell hook (user-configured
in `~/.hermes/agent-hooks/`, wired via config.yaml `hooks:` for
`write_file|patch|execute_code`) may already normalize the dash set on files
the agent writes; this scan-and-patch workflow is the enforcement point for
files written outside the agent (manual edits, imported content) and for the
full 24-char set the hook does not cover.

### 6a. Minimal character map

| Unicode    | Name                                     | Replace with                                                                                                                                                        |
| ---------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| U+2014 `-` | EM DASH                                  | Context-dependent: `-` → `.  `, `,  `, or `: ` as prose punctuation; when both sides are equally weighted, use `-` (en dash also acceptable; plain `-` is simplest) |
| U+2013 `-` | EN DASH                                  | `-`                                                                                                                                                                 |
| U+2018 `‘` | LEFT SINGLE QUOTATION MARK               | `'`                                                                                                                                                                 |
| U+2019 `’` | RIGHT SINGLE QUOTATION MARK / APOSTROPHE | `'`                                                                                                                                                                 |
| U+201C `“` | LEFT DOUBLE QUOTATION MARK               | `"`                                                                                                                                                                 |
| U+201D `”` | RIGHT DOUBLE QUOTATION MARK              | `"`                                                                                                                                                                 |

### 6b. Full regex set (24 characters)

The former normalize-dashes hook applied this broader set to every file written
by the agent. The live hook covers only the common dash codepoints (U+2010-U+2015
range); use this full set as the scan set for documentation audits:

```
U+058A  Armenian hyphen         U+2E17  Double oblique hyphen
U+05BE  Hebrew maqaf             U+2E1A  Hyphenation point
U+1400  Canadian syllabics hyp   U+2E3A  Two-em dash
U+1806  Mongolian soft hyp       U+2E3B  Three-em dash
U+2010  Hyphen                   U+2E40  Double hyphen
U+2011  Non-breaking hyphen      U+2E5D  Oblique hyphen
U+2012  Figure dash              U+301C  Japanese wave dash
U+2013  En dash                  U+3030  Double vertical line
U+2014  Em dash                  U+30A0  Kana double hyp
U+2015  Horizontal bar           U+FE31  Vert. em dash (pres)
U+FE32  Vert. en dash (pres)     U+FE58  Small em dash
U+FE63  Small hyphen-minus       U+FF0D  Fullwidth hyphen-minus
```

Run `python3 scripts/dash-normalizer.py --full` to scan with the complete set.

### 6c. Workflow

1. **Scan first** - run `scripts/dash-normalizer.py` in check mode to list all
   instances (`--check` = minimal 6-char set, exit 1 if any found; `--full` =
   24-char set; `--check --strict mode=repo` = exclusion presets). The script
   has no write mode - read-only by design, consistent with the manual-edit
   constraint.
2. **Exclude generated/vendor content** (see §6d).
3. **Edit manually, one file at a time** - read each line's surrounding
   context, then `patch`.
4. **Re-run check mode** to verify zero remaining instances in authored files.

### 6d. Where to patch vs where to skip

**Patch:** `README.md`, `docs/` (install/config/api/proxy/architecture/plugin/
guides/tool-relay), `.hermes/` authored docs (release notes, notes, skills).

**Skip:**

- `vendor/` - vendored submodule checkouts (`headroom`, `rtk`).
- `target/` - cargo build output.
- `plugins/aphrodite/` - the plugin submodule (separate repo, Aphrodite-Hermes).
- `node_modules/` - third-party packages.

**Mermaid subgraph heading exception:** in mermaid subgraph label strings, the
em dash `-` is a visual grouping separator between the element name and its
subtitle - replace with a single space ` `, not a hyphen.
`subgraph "Proxy - Cache"` → `subgraph "Proxy Cache"`.

**Workflow rule:** each file is edited individually - read the surrounding
context for every location, then patch. Do NOT batch-apply fixes without
reading each file to confirm the context first.

## 7. Standard README structure (Aphrodite conventions)

The repo README follows the restyled convention (see `github-readme-generation`
for authoring; this skill enforces it during fixes):

- **Title is a link.** A bracketed title `# [Name]` resolves via a
  link-reference definition at the bottom (`[Name]: <repo URL>`).
- **Emoji headers use an EM QUAD (U+2001), not a space:** `## Text emoji`.
  Verify with a byte check (`\xe2\x80\x81`) - a normal space is insufficient.
- **Badges use `/static/v1`** (never `/badge/`, which breaks when the message
  contains a `/`); each badge is clickable (`<a href>` wrapper); license badge
  shows the actual license.
- **GitHub markdown alerts:** `> [!TYPE]` on its own line, then a blank `>`
  line, then the body - never merge the tag with body text on one line.
- **Code fences are labeled** with a bold source/file callout before the block.
- **Hero images are centered** with `<p align="center">…</p>`, sized for wide
  captures, descriptive `alt`.
- **Contributing/License are short link-style headers** backed by real files
  (`CONTRIBUTING.md`, `LICENSE`), never full prose in the README.

```
# **Aphrodite**&#x2001;💋
[![badges...]](...)

> [!NOTE]
>
> Body text here.

## Install&#x2001;
## Usage&#x2001;
## Contributing&#x2001;   ← link to CONTRIBUTING.md
## License&#x2001;        ← link to LICENSE
```

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
  `🙏`, `🪱`, `👌`, `✊`, `👉` and its modifier. The pair must be a single
  code-point cluster with zero width between them.
- **Emoji on the RIGHT of all text segments** - emoji belong at the end of a
  heading, sentence, or labeled item, separated from the preceding text by an
  em-quad. Never before the prose they decorate.
- `https://` absolute URLs preferred for links.
- Repo links: `github.com/PlayForm/Aphrodite/tree/Current/...` for the parent,
  `github.com/PlayForm/Aphrodite-Hermes` for the plugin (it is a separate
  repo, never a tree inside the parent).

## 9. Post-pass corruption scan (ALWAYS after backticking)

The bulk formatter injects backticks indiscriminately. Run this sweep after:

```python
import re

patterns = {
    # Backtick inside HTML attribute URLs
    r'(src|href)="[^"]*`[^"]*"': "HTML attribute URL backtick",
    # Backtick inside markdown link URLs
    r'\]\(https?://[^)]*`[^)]*\)': "Markdown link URL backtick",
    # NGI0 Commons Fund corruption (Common → `Common`)
    r'NGI0 `Common`s Fund': "NGI0 Commons corruption",
    # Windows corruption (Wind → `Wind`)
    r'`Wind`ows': "Windows corruption",
    # Mid-URL backtick style
    r'https://[a-zA-Z.]+`[a-zA-Z]': "Mid-URL backtick",
}

for filepath in all_touched_files:
    with open(filepath) as f:
        content = f.read()
    for pattern, label in patterns.items():
        matches = list(re.finditer(pattern, content))
        if matches:
            print(f"\n--- {label} in {filepath} ---")
        for m in matches:
            line_no = content[:m.start()].count('\n') + 1
            print(f"  L{line_no}: {m.group()[:100]}")
```

**Do NOT flag:** ``[`url`][ref]`` - intentional markdown link text
formatting.

**Real corruptions to fix:**

| Pattern                                                   | Fix                          |
| --------------------------------------------------------- | ---------------------------- |
| `src="https://...` backtick                               | remove backtick, restore URL |
| `NGI0 `Common`s Fund`                                     | `NGI0 Commons Fund`          |
| `` `Wind`ows ``                                           | `Windows`                    |
| Any backtick between `:` and domain continuation in a URL | remove the rogue backtick    |

## Related: authoring vs fixing

When authoring new README content (badges, hero images, install sections,
release-note structure), use the `github-readme-generation` skill; this skill
is the fix/enforcement pass on top of the same conventions.
