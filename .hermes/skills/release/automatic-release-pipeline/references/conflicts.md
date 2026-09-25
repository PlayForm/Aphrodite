# Conflict taxonomy for scheduled release + Dependabot auto-merge

## The 9 conflicts

C1 Two writers to the version manifests / release branch (Release vs Dependabot auto-merge) - stale checkout
overwrites dep bump OR push rejected.
C2 Concurrent Release runs (cron + manual dispatch) → double bump/tag/publish (rejected).
C3 `cancel-in-progress: true` kills job mid-commit/tag → orphaned version commit.
C4 Failed push + rebase moves version commit; local tag points at pre-rebase SHA.
C5 CHANGELOG.md / docs copy edit conflict (human vs bot).
C6 Publish.yml double-trigger (keep the tag-push trigger AND call via `workflow_call`).
C7 Lockfile drift (no committed lockfile → non-reproducible publish).
C8 Auto.yml overlap (no-op daily empty-commit + new Release push).
C9 Dependabot line-level clash on the version manifest.

## Mitigations

- C1/C2: SAME `concurrency.group` (e.g. `aphrodite-release-write`) on Release.yml AND
  Dependabot `Merge`; `cancel-in-progress: false` → queue, never overlap.
- C3: `cancel-in-progress: false` (queue) + bounded retry (no irreversible partial publish;
  only `cargo publish`/`npm publish` is non-idempotent → guard it).
- C4: `git pull --rebase` right before bumping; create `git tag` AFTER rebase, immediately
  before push; bounded 3-retry re-absorbs + `git tag -f`.
- C5: CHANGELOG.md is source of truth; sync docs copy in same commit.
- C6: make Publish.yml `workflow_call`-only; publish idempotent (crates.io version-exists
  guard: `curl -sL https://crates.io/api/v1/crates/<crate>/<version> | grep -q '"num"'`).
- C7: generate + commit a lockfile in the Release job (match pnpm if repo uses pnpm).
- C8: delete no-op Auto.yml when adding Release.yml.
- C9: different lines (version field vs deps) → usually clean via rebase.

## GITHUB_TOKEN cross-workflow gotcha

A GitHub Release created with the default `GITHUB_TOKEN` does NOT fire another workflow's
`on: release: created`. Fix: call the publisher via `workflow_call`, or inline the publish step.

## Net result

Race-safe (shared group + retry), overlap-safe (serialized + idempotent), cancellation-safe
(queue + recoverable), reproducible (lockfile). Residual ~1-2s race covered by bounded retry.
