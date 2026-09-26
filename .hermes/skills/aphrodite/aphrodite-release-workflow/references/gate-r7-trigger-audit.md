# Gate R7 - Pre-Publish Trigger Audit (worked evidence)

Owner: `aphrodite-release-workflow` (section 2 of the owning SKILL.md). The
ceremony (`aphrodite-release-flow`) runs this audit before ANY tag; the
definitions live here. This file is the verified snapshot for commit
`677a7b3` (2026-09-25; trigger chain restructured from `a81acab6`) - it is **evidence, not a
substitute for the audit**: inspect the
actual workflow at the exact commit to be tagged, build a trigger table,
never trust remembered or documented behavior. A workflow file that exists
is not a workflow that behaves as documented; the `if:` conditions at the
tag commit are the behavior.

## C-002 resolution (stated explicitly)

One release document claimed `Publish.yml` only publishes crates after a
deliberate `workflow_dispatch`, and that a plain tag push triggers only
Build.yml's GitHub Release artifacts. That was **false at `a81acab6`** - a
plain `Aphrodite/v*` tag push reached `cargo publish` for `aphrodite` and
`aphrodite-hermes` directly (publish steps carried
`|| startsWith(github.ref, 'refs/tags/Aphrodite/')`). **At `677a7b3` the
chain was restructured**: the tag push starts Build.yml only (tag-push-only
trigger); Publish.yml follows via `workflow_run` on Build `completed`
(jobs gated `conclusion == 'success' && head_branch startsWith
'Aphrodite/v'`, checkouts pinned to the tag) and `cargo publish` for
`aphrodite` and `aphrodite-hermes` runs there. `aphrodite-headroom-core`
is never CI-published: its step `if` names `workflow_dispatch &&
inputs.publish_crates`, unreachable under the `workflow_run`-only `on:`
(the dispatch input is gone). A tag push is not a build-only event.

## Gate R7 template (the ceremony runs this; definitions owned here)

**Read**

- Workflow files at the intended release commit (`.github/workflows/*.yml`)
- Trigger clauses for tag push, push branch, `workflow_dispatch`,
  `workflow_call`, and `workflow_run` (Build -> Publish chaining)

**Record**

- Which workflows trigger from this tag
- Which jobs publish GitHub assets
- Which jobs publish crates/packages
- Required secrets and manual inputs

**Pass**

- The release owner has explicitly accepted every triggered side effect.

**Stop**

- Any unexpected publish job is reachable from the tag.

## Verified trigger table (commit `677a7b3`, 2026-09-25)

| Event                                                | Triggered workflows        | Jobs                                                                                                                             | Publishing side effects                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tag push `Aphrodite/v*` | `Build.yml`, then `Publish.yml` via `workflow_run` (Build `completed`) | Build: Release, Build×4 (matrix), Finalize (commits in-tree `plugins/aphrodite/SHA256SUMS.txt`, pushes child Current with a fine-grained PAT, tags the plugin `v$(plugin.yaml version)` at the child tip); Publish: Test → Publish-Headroom-Core (check only) → Publish-Aphrodite → Publish-Hermes → Bump-Plugin-Gitlink | GitHub release created + 12 assets attached (Build); **`cargo publish -p aphrodite`** (Publish-Aphrodite) and **`cargo publish -p aphrodite-hermes`** (Publish-Hermes) in the chained run - no already-published version check, so a re-publish errors red and a never-published version IS published by the tag push → Build → Publish chain; parent gitlink floated to the child Current tip (GH006/protected-branch → loud warning + exit 0). `aphrodite-headroom-core` is **never** CI-published (its step `if` is `workflow_dispatch`-only, unreachable under `workflow_run`). |
| `workflow_dispatch` (any) | - | Removed at `677a7b3`: `on:` is `workflow_run` only; the `publish_crates` input no longer exists | None - no dispatch trigger remains. |

Re-audit command at the tag commit:

```bash
grep -nE 'workflow_run|head_branch' .github/workflows/Publish.yml
```

## Notes from the same source

- Publish jobs run in `environment: Release` - if that environment has
  required reviewers/approval rules, jobs pause there; that is a workflow-
  level human-approval boundary, not a substitute for the pre-tag audit.
- Secrets needed for the publish path: `CARGO_REGISTRY_TOKEN` (crates.io) and
  the default `GITHUB_TOKEN`. No manual inputs - the `publish_crates` dispatch
  input is gone (677a7b3). Named, never echoed.
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
  red, a never-published version IS published by the tag push → Build → Publish `workflow_run` chain alone.