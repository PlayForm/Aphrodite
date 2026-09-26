# Gate R7 - Pre-Publish Trigger Audit (worked evidence)

Owner: `aphrodite-release-workflow` (section 2 of the owning SKILL.md). The
ceremony (`aphrodite-release-flow`) runs this audit before ANY tag; the
definitions live here. This file is the verified snapshot for commit
`a81acab6` - it is **evidence, not a substitute for the audit**: inspect the
actual workflow at the exact commit to be tagged, build a trigger table,
never trust remembered or documented behavior. A workflow file that exists
is not a workflow that behaves as documented; the `if:` conditions at the
tag commit are the behavior.

## C-002 resolution (stated explicitly)

One release document claimed `Publish.yml` only publishes crates after a
deliberate `workflow_dispatch`, and that a plain tag push triggers only
Build.yml's GitHub Release artifacts. That claim is **false at the verified
commit** - a plain `Aphrodite/v*` tag push DOES reach `cargo publish` for
`aphrodite` and `aphrodite-hermes` (their publish steps carry
`|| startsWith(github.ref, 'refs/tags/Aphrodite/')`); only
`aphrodite-headroom-core` is truly dispatch-gated. A tag push is not a
build-only event.

## Gate R7 template (the ceremony runs this; definitions owned here)

**Read**

- Workflow files at the intended release commit (`.github/workflows/*.yml`)
- Trigger clauses for tag push, push branch, `workflow_dispatch`, and
  reusable calls (`workflow_call`)

**Record**

- Which workflows trigger from this tag
- Which jobs publish GitHub assets
- Which jobs publish crates/packages
- Required secrets and manual inputs

**Pass**

- The release owner has explicitly accepted every triggered side effect.

**Stop**

- Any unexpected publish job is reachable from the tag.

## Verified trigger table (commit `a81acab6`, 2026-09-20)

| Event                                                | Triggered workflows        | Jobs                                                                                                                             | Publishing side effects                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tag push `Aphrodite/v*`                              | `Build.yml`, `Publish.yml` | Build: Release, Build×4 (matrix), Finalize; Publish: Test, Publish-Headroom-Core (check only), Publish-Aphrodite, Publish-Hermes | GitHub release created + 12 assets attached (Build); **`cargo publish -p aphrodite`** (Publish-Aphrodite) and **`cargo publish -p aphrodite-hermes`** (Publish-Hermes) - no already-published version check, so a re-publish errors red and a never-published version IS published by the tag push alone. `aphrodite-headroom-core` is **not** published on tag push (its publish step is `workflow_dispatch`-only). |
| `workflow_dispatch` (default `publish_crates=false`) | `Build.yml`, `Publish.yml` | Build: Build×4 only (Release/Finalize/upload steps are tag-gated); Publish: Test + headroom version check; publish steps skipped | None - build only.                                                                                                                                                                                                                                                                                                                                                                                                   |
| `workflow_dispatch` with `publish_crates=true`       | `Build.yml`, `Publish.yml` | Publish: Test → Publish-Headroom-Core → Publish-Aphrodite → Publish-Hermes (hard `needs:` chain)                                 | `aphrodite-headroom-core` first (skipped if already live - index check), then `aphrodite`, then `aphrodite-hermes`.                                                                                                                                                                                                                                                                                                  |

Re-audit command at the tag commit:

```bash
grep -A3 'Publish to crates.io' .github/workflows/Publish.yml
```

## Notes from the same source

- Publish jobs run in `environment: Release` - if that environment has
  required reviewers/approval rules, jobs pause there; that is a workflow-
  level human-approval boundary, not a substitute for the pre-tag audit.
- Secrets needed for the publish path: `CARGO_REGISTRY_TOKEN` (crates.io) and
  the default `GITHUB_TOKEN`. Manual input: `publish_crates` (boolean).
  Named, never echoed.
- Build.yml's `Finalize` job fails the release if any of the 12 expected
  assets is missing (all four targets × binary + dylib + `SHA256SUMS`).
- Build.yml release notes are authored per `.hermes/release/RELEASE-TEMPLATE.md`,
  never auto-generated.
- `Publish.yml`'s `Test` job gates the publish chain: `cargo test -p aphrodite
  -p aphrodite-hermes --release` plus a packaging guard asserting all six
  `src/builtin_directives/*.md` are inside the `cargo package` tarball (the
  v1.3.8 regression: a recursive `*.md` exclude stripped them and
  `cargo install` failed to compile).
- The tag push has no already-published version check: a re-publish errors
  red, a never-published version IS published by the tag push alone.