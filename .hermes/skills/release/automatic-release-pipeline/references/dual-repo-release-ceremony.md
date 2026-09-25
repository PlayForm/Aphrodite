# Dual-repo release ceremony (parent + plugin submodule)

Layout: parent repo PlayForm/Aphrodite (Rust workspace: `crates/aphrodite` core
crate, `crates/aphrodite-hermes` plugin-binary crate, `plugins/aphrodite` git
submodule = the separate PlayForm/Aphrodite-Hermes child repo). Parent and
child versions move in lockstep; parent release tags follow `Aphrodite/v<semver>`
(annotated, PGP-signed when `tag.gpgsign` is set, message = the tag name).

## Order of operations

1. **Child fix first.** Land and commit the fix in the child repo; verify the
   child is on branch `Current` (never detached) and push its tip.
2. **Enumerate surfaces.** `git show <prev-release-commit> --stat` in BOTH
   repos lists the exact file set the ceremony bumps:
    - child: `plugin.yaml`, `BINARY_VERSION`, `fixtures/*version*.json`,
      README badge
    - parent: `CHANGELOG.md`, README badges (release + plugin tracks),
      both `Cargo.toml`s incl. the path-dep pin, `Cargo.lock`, `package.json`,
      `Publish.yml` version comments, the submodule gitlink
3. **Bump the child** (one line per file), run the child's version-compat AND
   real-binary tests, commit `release: Bump plugin version to <v>`, push
   child `Current`.
4. **Bump the parent**, confirm `Cargo.lock` consistency (`cargo check`),
   `git add` the child gitlink, commit
   `release: Prepare v<v> (version bump, changelog)`, push `Development`.
5. **Fork publish first (only when the headroom fork's crate version moved):
   dispatch `Publish.yml` with `publish_crates=true` so the fork publishes
   before the parent tag-push publish resolves it from the registry.**
6. **Tag**: `git tag -a Aphrodite/v<v> -m "Aphrodite/v<v>"` - never bare
   (signing opens an editor); push the tag. Do NOT push anything else to
   `Development` until the tag-triggered workflows finish.
7. **Verify**: `gh run list` shows Build + Publish on the tag; release has
   all 12 assets (4 targets × binary + dylib + `SHA256SUMS-<target>.txt`);
   crates.io index carries `<v>` for both crates; the child's in-tree
   `SHA256SUMS.txt` header reads `BINARY_VERSION: <v>`; parent gitlink =
   child tip; `git submodule status` shows no `+`.

## Failure modes (validated)

- **Bump-Plugin-Gitlink non-fast-forward.** The job commits on detached HEAD
  at the tag commit and pushes `HEAD:Current`; any commit pushed to `Current`
  after the tag push (an unrelated fix is enough) makes the push non-FF and
  the bump dies. Land the same bump manually (add the gitlink, commit, push)
    - the workflow's detached commit is dangling and harmless.
- **Concurrency cancellation is harmless here.** The `workflow_dispatch`
  publish run shares the tag ref's concurrency group (`cancel-in-progress:
true`), so it cancels the tag-push Publish run - but the dispatch run
  re-executes the tag-gated jobs (gitlink bump included), so nothing is lost.
- **Real-binary tests fail after the bump** = the locally installed binary is
  the old version. Rebuild BOTH binaries from the workspace root
  (`cargo install --path crates/<crate> --force`), then re-run - the failures
  are the expected transient, not a regression.
- **Child pushes during Build.yml Finalize.** Finalize checks out the child's
  `origin/Current` fresh and pushes its SHA256SUMS commit on top with a plain
  push - child commits pushed before Finalize starts stack safely; only its
  own short commit→push window races.
- **Dependabot config in `.github/workflows/`** is parsed as a workflow and
  reds every push - it lives at `.github/dependabot.yml`.
