# Worked Instance - 2026-09-18 Maintenance Pass

Distilled from the session that produced the six `aphrodite-*.py` tools and
`insert_rustfmt_override.py` (source scripts previously kept in the Maintain
scratch tree, now canonical in `scripts/`). Reconstructed from the scripts'
own invariants; totals below are the guarantees each run printed.

## Link hygiene (public markdown)

1. `aphrodite-link-inventory.py` first - read-only. Sized the work from the
   `Development` ref: counts of relative links, absolute `tree/Current`
   links, and absolute-other links, plus an existence check for every
   resolved relative target (`OK` / `MISSING`). Any `MISSING` row is a
   broken link to fix before rewriting.
2. `aphrodite-link-rewrite.py` - converted relative file links to absolute
   `https://github.com/<PROJECT>/tree/Current/...` URLs, each rewritten
   target verified against `git ls-tree` on the `Current` ref. Run exited 0
   only when every converted target existed on `Current` ("all rewritten
   targets verified to exist on Current").
3. `aphrodite-link-branch-flip.py` - the inverse pass: flipped the
   `tree/Current` URLs back to `tree/Development` so Development's docs
   self-link. The stray-occurrence guard refused any `tree/Current` text
   outside a full Aphrodite URL.

Order matters: inventory → rewrite → branch-flip, never rewrite + branch-flip
in one pass (they implement opposite conventions).

## VSCode settings parity

- `aphrodite-port-settings.py` inserted the 3 ruff keys
  (`ruff.lineLength` 100, `ruff.organizeImports`, `ruff.showSyntaxErrors`)
  ahead of `"files.exclude"`, CRLF preserved. The anchor refused once when
  the file had already drifted (prior manual edit) - the file was left
  untouched and the anchor was re-checked before the retry.
- `aphrodite-resolve-vscode-settings.py` resolved the `.vscode/settings.json`
  squash conflict from the Development/Current merge: Development's (HEAD)
  structure kept, Current's `[python]` `editor.formatOnPaste: true` +
  explanatory comment grafted in, all three conflict markers asserted
  exactly once and verified absent from the merged output.
- `insert_rustfmt_override.py` inserted the
  `rust-analyzer.rustfmt.overrideCommand` block (rustup run
  `nightly-2026-05-01` rustfmt `--edition 2024`) before `"[rust]"` -
  byte-level CRLF needle; no-op once `overrideCommand` was present.

## Dep bump

- `aphrodite-dep-bump.py` applied 12 literal direct-dep bumps in
  `crates/aphrodite/Cargo.toml` (Development → Current's pins = latest
  crates.io, verified 2026-09-18), crate version 1.4.6 untouched. Each
  pair asserted to match exactly once.

## Lessons

- The `patch` tool cannot match across CRLF - the settings utilities must
  stay byte-level (`newline="\r\n"` / bytes needle).
- `git ls-tree` on `Development`/`Current` refs needs no network; keep the
  refs present locally for the checks.
- All writes land in the working tree only; the repo's auto-committer
  sweeps them - never stage manually.