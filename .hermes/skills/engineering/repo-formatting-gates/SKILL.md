---
name: repo-formatting-gates
description: "Use when running a repo's formatting gates with CI parity - the CI toolchain and CI scope, never the PATH formatter."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: engineering
category_taxonomy: engineering/repo-formatting-gates
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, formatting, gate-checking, toolchain-pinning, scoping]
        related_skills: [code-quality-improvement, markdown-readme-audit-fix]
status: active
---

# Repo formatting gates

Run and repair a repo's formatting gates the way CI actually enforces them.
The cardinal rule: the gate that counts is the one CI runs, with CI's
TOOLCHAIN and CI's SCOPE - never the formatter that happens to be on PATH
and never a root-wide sweep the CI gate does not make.

## Procedure

1. **Read the CI gate first.** Find the workflow step that runs each
   formatter (Check.yml / push workflows): the exact command, the
   toolchain (a pinned `cargo +nightly-<pin> fmt --check`, or an UNPINNED
   `toolchain: nightly` that resolves to whatever is latest when the run
   starts), and the SCOPE
   (`astral-sh/ruff-action` with `src: plugins/...` - a scoped ruff never
   sees the rest of the tree). The gate is whatever that step runs.
2. **Check the toolchain pin** (`rust-toolchain.toml`) against the CI's
   formatter pin. If the rustfmt.toml has nightly-only options and the
   toolchain file pins STABLE, plain `cargo fmt` is NOT the gate formatter.
3. **Run each gate with CI parity**: `cargo +<ci-pin> fmt --check` for
   rust, the CI-scoped ruff command, `npx prettier --check` on the tracked
   md set. For a local ruff pass, scope to the repo's own tracked files:
   `ruff format $(git ls-files '*.py')` - `git ls-files` never lists
   submodule contents, so vendored trees are excluded automatically.
4. **When a gate is red, decide if the product changed or the TOOL changed.**
   A newly-flagged set with no matching source edits usually means the
   tool version moved (new ruff formats Python inside .md, new rustfmt
   reflows) - compare against what the CI scope actually covers before
   touching files.

## Pitfalls

- **A formatter config full of nightly-only options silently diverges
  under the STABLE toolchain.** When rustfmt.toml sets
  `unstable_features = true` plus nightly-only keys (`imports_granularity`,
  `group_imports`, `format_code_in_doc_comments`, `ignore`,
  `space_after_colon`...), STABLE rustfmt silently ignores them (warnings:
  "unstable features are only available in nightly channel") and emits
  different output (spaces after colons, flat imports). Format with the
  gate-equal formatter `cargo +<ci-pin> fmt`; a STABLE pass can flip a
  green gate red by re-introducing exactly the divergence the nightly
  rules remove.
- **VSCode Alt+F is a third formatter - wire it to the same pin.**
  rust-analyzer uses whatever `rustfmt` it resolves; a repo with a
  nightly-only config needs
  `rust-analyzer.rustfmt.overrideCommand: ["rustup", "run", "<pin>",
"rustfmt"]` in .vscode/settings.json or editor formatting diverges from
  CI. Check for the existing override before touching the formatter.
- **Never reformat vendored submodule content.** A root-wide formatter
  pass walks `vendor/` (submodule checkouts are in the working tree, not
  gitignored). Reformatting them dirties submodules and rewrites upstream
  docs. Scope by `git ls-files` (excludes submodule trees) or the CI
  scope; keep `vendor/` in the format ignore lists.
- **Latest ruff formats Python blocks inside .md files** - a `ruff format
--check .` at the repo root flags doc/note files (and vendored wikis)
  that the CI's scoped ruff never checks. This is a tool-version change,
  not a repo regression: verify the CI scope before "fixing" the flagged
  files, and leave the .md python blocks to prettier (the markdown
  owner).
- **The nightly-vs-stable warning lines are the tell.** Stable rustfmt
  prints "can't set X, unstable features are only available in nightly
  channel" for each ignored option - if you see them, the pass you just
  ran is NOT the gate formatter and its output is not gate-clean.
- **An UNPINNED `toolchain: nightly` in CI resolves at RUN TIME - a
  local dated nightly goes stale.** CI diffs that reformat code you did
  not touch while local `fmt --check` is green mean the CI formatter is
  NEWER than the local one, not that the tree drifted. Typical tell:
  newer rustfmt reflows `&&`-chained let-chain conditions so every
  operand sits on its own line (`&& w == 0 && let Some(..)` becomes
  `&& w == 0` / `&& let Some(..)`), while the older formatter accepts the
  merged shape. Fix: `rustup update nightly` to sync the local formatter
  to the CI-resolved one, apply exactly the shape the CI diff shows
  (manual edit, not a formatter run from the stale toolchain), and
  re-run `cargo fmt --all -- --check` under the fresh formatter. Never
  "repair" the diff list with the stale formatter - a stable/dated pass
  can re-introduce the divergence the newer rules remove.
- **A stable pass reporting HUNDREDS of diffs is a tool divergence, not
  a repair signal.** `cargo fmt --check` under stable against a
  nightly-config repo reports the whole tree as unformatted (e.g. 684
  hunks, 112 in your new tree) while the CI-pinned command reports 0-1.
  Do NOT start "fixing" files on the stable diff list - run the
  CI-pinned command first; the count collapses. Then fix only what the
  pinned command flags (usually one file the sweep got wrong).
- **Re-verify after anything rewrites the tree between check and
  report.** An external auto-committer/sweeper can reformat files (often
  with a DIFFERENT toolchain than the gate - a stable sweep re-introduces
  exactly the divergence nightly removes, e.g. `: &T` spaces where
  `space_after_colon = false` wants `:&T`) between your `fmt --check`
  and your summary, so a check run minutes earlier is stale. If the diff
  list changes across runs, or files you never touched appear in it, or
  `git status` shows edits that vanish on re-read: re-run the CI-pinned
  command after the sweep settles, and grep the diff list for YOUR tree
  before claiming your files are clean or dirty.
- **A pnpm-workspace.yaml ABOVE the repo hijacks `pnpm exec` / `pnpm
install` from inside it.** Repos living under a parent workspace root that
  carries its own pnpm-workspace.yaml resolve upward: `pnpm exec prettier` runs
  the parent project's lifecycle scripts (a foreign postinstall can fail the
  whole run) and `pnpm install` re-resolves the workspace. Run the repo's
  LOCAL binary directly (`./node_modules/.bin/prettier`) or install with
  `pnpm install --ignore-workspace --no-lockfile`; the VS Code prettier
  extension likewise resolves plugins from the ambient workspace - harmless
  when versions match, but verify with the local binary, never the editor
  log.
- **Generated artifacts must be formatter-canonical BY CONSTRUCTION, and
  the formatter runs AFTER generation (user preference).** A hand-rolled
  exporter that emits tab-indented YAML fails prettier's parser outright
  ("Tabs are not allowed as indentation") - fix the emitter to produce
  canonical output, do not paper over the artifact with a `.prettierignore`
  entry. Generation scripts that produce docs/fixtures end with the format
  pass (cargo fmt + prettier --write) and then the full test suite, so a
  refresh leaves the tree gate-clean with zero manual follow-up; a stable
  `prettier --write` on generated files then only pads tables and is
  reproducible output.
