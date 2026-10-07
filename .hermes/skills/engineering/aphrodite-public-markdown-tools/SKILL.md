---
name: aphrodite-public-markdown-tools
description: "Use when running public-markdown link hygiene (inventory/rewrite/branch-flip), .vscode/settings.json parity utilities (ruff port, squash resolve, nightly-rustfmt override), or the crates/aphrodite dep-bump to Current's pins in PlayForm/Aphrodite."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: engineering
category_taxonomy: engineering/aphrodite-public-markdown-tools
date: 2026-09-29
metadata:
    hermes:
        tags: [engineering, markdown, links, vscode-settings, rustfmt, dep-bump, aphrodite]
        related_skills:
            - github-readme-generation
            - markdown-readme-audit-fix
            - aphrodite-cargo-upgrade
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
        - Current
owns:
    - The public-markdown link hygiene runbook (inventory -> rewrite -> branch-flip)
    - The .vscode/settings.json parity utilities (ruff key port, squash conflict resolve, nightly-rustfmt override)
    - The crates/aphrodite dep-bump pointer (literal-replace bumps to Current's crates.io pins)
depends_on:
    - aphrodite-orientation (preflight gate before any write)
supersedes: []
verification:
    source_of_truth:
        - .hermes/governance/SKILL-MANIFEST.md
        - git refs Development/Current (link targets verified against ls-tree)
        - .vscode/settings.json (editor parity ground truth)
mutation_level: local
---

# Aphrodite Public Markdown & Settings Tools

Seven distilled Python utilities for the Aphrodite monorepo: public-markdown
link hygiene across the `Development`/`Current` refs, `.vscode/settings.json`
parity between the two branches, and the direct-dependency bump to Current's
crates.io pins. All scripts live in `scripts/`, are abstracted (repo root is
derived from the script's own location, `<PROJECT>` is a one-line constant),
and follow a shared **assert-once / refuse-on-drift** discipline: every literal
replace must match exactly once, otherwise the script prints `REFUSE`, exits 1,
and leaves the target file untouched.

## When to Use

- Development's public markdown (`README.md`, `CHANGELOG.md`, `docs/**/*.md`)
  needs link hygiene before a merge or release.
- `.vscode/settings.json` must match Current's editor behavior (ruff keys,
  `[python]` block, nightly rustfmt override) without losing Development's
  structure.
- `crates/aphrodite/Cargo.toml` direct deps must be bumped to Current's pins
  (= latest crates.io).

## Link hygiene pipeline (run in this order)

1. **Inventory** - `aphrodite-link-inventory.py` (read-only). Reads files from
   the `Development` git ref, extracts `](...)` link targets, classifies them
   (relative / absolute `tree/Current` / absolute other), resolves relative
   targets against the file's directory, and checks whether each resolved
   target exists on the branch. Prints summary counts + a table. Run first to
   size the work and catch missing targets before any write.
2. **Rewrite** - `aphrodite-link-rewrite.py`. Converts relative file links in
   public files to absolute `https://github.com/<PROJECT>/tree/Current/...`
   URLs (Current's link convention). Every rewritten target is verified to
   exist on the `Current` ref; any relative link whose target is missing on
   `Current` is reported and left untouched, and the run exits 1. Absolute
   http(s) links and pure same-file anchors are left alone. Writes the working
   tree.
3. **Branch-flip** - `aphrodite-link-branch-flip.py`. Flips
   `<PROJECT>/tree/Current/` URLs to `<PROJECT>/tree/Development/` in public
   markdown (Development's docs must link to Development). Asserts every
   occurrence sits inside a full Aphrodite URL - any stray `tree/Current`
   outside the URL form refuses the run. Writes the working tree.

`rewrite` and `branch-flip` are inverse conventions. Use `rewrite` when the
docs should mirror Current's absolute `tree/Current` links; use `branch-flip`
when the docs must self-link to Development. Do not run both in one pass.

## VSCode settings utilities (all CRLF-safe)

The working tree's `.vscode/settings.json` is in `core.autocrlf=true`
checkout form (CRLF). The `patch` tool cannot match across `\r\n` - use these
byte-preserving scripts instead.

- `aphrodite-port-settings.py` - ports the 3 ruff keys
  (`ruff.lineLength` 100, `ruff.organizeImports`, `ruff.showSyntaxErrors`)
  from Current into Development's settings, inserting before
  `"files.exclude"`. Anchor must match exactly once; writes back with explicit
  `newline="\r\n"`.
- `aphrodite-resolve-vscode-settings.py` - resolves a squash-conflict state
  (`<<<<<<< HEAD ... ======= ... >>>>>>> Source/Current`) keeping
  Development's (HEAD) structure and grafting Current's unique `[python]`
  behavior (`editor.formatOnPaste: true` + the ruff-explanatory comment).
  Asserts all three conflict markers exactly once and that none remain in the
  merged output; writes CRLF.
- `insert_rustfmt_override.py` - inserts the
  `rust-analyzer.rustfmt.overrideCommand` block (rustup run
  `nightly-2026-05-01` rustfmt `--edition 2024`, the same nightly CI's
  Check.yml pins) right before the `"[rust]"` section, byte-level on CRLF.
  No-op when `overrideCommand` is already present.

## Dep-bump pointer

- `aphrodite-dep-bump.py` - applies 12 literal `(old, new)` version bumps to
  `crates/aphrodite/Cargo.toml` direct deps (Development -> Current's pins,
  verified 2026-09-18). Each pair must match exactly once or the whole file is
  left untouched. Leaves the crate version (1.4.6) and file newlines alone.

## Running

- Any of the 7 scripts: `python3 scripts/<name>.py` from a clone of
  `<PROJECT>` (paths are derived from the script location - no machine-local
  paths). No network needed: link checks use `git ls-tree` on local refs.
- The repo must be on `Development` with both `Development` and `Current`
  refs present locally (`ls-tree` fails otherwise).
- The link scripts touch only public files: `README.md`, `CHANGELOG.md`,
  `docs/**/*.md` (inventory also reads `aphrodite.toml.example` and the two
  crate READMEs).
- Run the orientation gate (`aphrodite-orientation`) before any write; leave
  all changes unstaged - the repo's auto-committer sweeps the working tree.

## Common Pitfalls

1. **`patch` on CRLF settings files.** `patch` cannot match across `\r\n`;
   use the byte-level scripts (`insert_rustfmt_override.py`, ports) instead.
2. **Running rewrite and branch-flip back-to-back.** They implement opposite
   conventions; pick the one matching the current goal, never both.
3. **Ignoring refuse-on-drift semantics.** A `REFUSE` (exit 1) means the file
   was left untouched - do not force the edit; investigate the anchor drift
   first (a prior run may already have applied part of it).
4. **Missing refs.** `Development`/`Current` must exist as local refs for the
   `ls-tree`-based checks; a fresh shallow clone breaks them.
5. **Editing non-public files.** The link scripts deliberately exclude
   everything outside the public file list; keep it that way.
6. **Committing manually.** Repo skills and their scripts are source changes;
   staging belongs to the Integrate phase (auto-committer).

## Verification Checklist

- [ ] All 7 scripts pass `python3 -m py_compile`
- [ ] `aphrodite-dep-bump.py` prints `OK: 12 dep bumps applied` and the
      Cargo.toml diff shows only version changes
- [ ] `aphrodite-link-rewrite.py` exits 0 with "all rewritten targets
      verified to exist on Current"
- [ ] `aphrodite-link-branch-flip.py` exits 0 and `TOTAL` matches the
      inventory's absolute `tree/Current` count
- [ ] Settings scripts preserve CRLF (verify with `file` after a write)
- [ ] No personal paths (`/Users/...`, `/Volumes/...`) in SKILL.md, scripts/,
      or references/ - repo skills are public
- [ ] `git status --short` shows only intended, unstaged changes
