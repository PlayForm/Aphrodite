# Release readiness verification

Prove a release is actually consumable BEFORE reporting success or automating a
release. Every claim in a repo (workflow files, version manifests, badges,
README install instructions) is only a claim - the truth is what the live
probes and the user-facing commands return.

## The ladder (cheapest → decisive)

1. **Registry path** - proves `cargo install <crate> --locked` / `npm i <pkg>`
   will work. Per package:
    ```sh
    curl -s -H "User-Agent: <ua>" https://crates.io/api/v1/crates/<name> \
    	| jq -r '.crate | .max_version + " created=" + .created_at + " downloads=" + (.downloads|tostring)'
    ```
    Empty/missing `crate` object = never published.
2. **GitHub path** - proves a release + tag exist:
    ```sh
    gh api repos/<owner>/<repo>/releases --paginate -q '.[] | .tag_name + " | " + (.published_at // "draft")'
    git ls-remote --tags <remote>     # remote tags, not just local ones
    ```
    Empty output on BOTH = no release and no tag. Do not read success from exit
    code - empty pipelines still exit 0; inspect the output.
3. **User-facing installer** - the decisive test. Run the actual install script
   the README tells users to run (`bash download.sh` / install script). A 404 on
   the asset URL with exit 1 proves the release assets were never uploaded,
   even when the installer resolved its pinned BINARY_VERSION / Cargo.toml
   version without complaint. Its error message even names the missing
   release URL - capture it, it is the fix instruction.
4. **Badges and links** - follow every README badge/URL to its target. A
   `release v0.0.1` badge pointing at an empty releases page is the report
   card: the pipeline is not done.

## Pitfalls

- **CI workflow exists ≠ release exists.** A `Build.yml` triggered
  `on: push: tags: ["Aphrodite/v*"]` runs only after a tag is pushed - nothing
  about the workflow's existence implies any tag was ever pushed.
- **Version-manifest consistency ≠ tag pushed.** Cargo.toml == installer
  BINARY_VERSION == plugin.yaml can all agree on 0.0.1 while the release tag
  was never created. The tag is the missing link, and it is invisible in every
  manifest - only `git ls-remote --tags` / `gh api releases` see it.
- **Independent delivery paths, independent failure modes.** crates.io
  availability and GitHub release assets are separate: a crate can be
  published (cargo install works) while every download.sh and badge is still 404. Verify each path separately and report each path's status distinctly.
- **Tag push may be the gated action.** When the repo contract says "nothing
  pushed without explicit OK", a release tag is exactly that gated action.
  Ask first; if no answer arrives, do not push - hand the user the exact
  commands (`git tag Aphrodite/v<ver> && git push <remote> Aphrodite/v<ver>`)
  and the expected outcome (tag triggers the release workflow, which uploads
  assets + SHA256SUMS).
- **Parse registry/API JSON with `jq` in terminal commands** - `python3 -c`
  one-liners are blocked by the runtime hook; jq is installed and reads the
  same streams without a file write.

## Run-mechanics traps (tag-triggered multi-platform releases)

- **Job `queued` forever = runner image starvation, not a busy queue.**
  `macos-13` (the last Intel-macOS hosted image) is effectively never
  scheduled on free tier - the job sits queued for 30+ minutes while every
  other leg finishes in ~1 minute. Fix the matrix, do not wait: build
  `x86_64-apple-darwin` on a macos-14/macos-latest ARM64 runner. The
  dtolnay/rust-toolchain action installs the requested `targets:` and clang
  cross-compiles macOS x86_64 from arm64 natively (no linker flags, no
  additional toolchain). Keep `fail-fast: false` so one starving leg does not
  cancel the others, and keep a Finalize gate so an incomplete matrix fails
  loudly instead of shipping.
- **Re-triggering after a workflow fix still runs the OLD file.**
  `gh workflow run W -R O/R --ref <tag>` reads the workflow YAML at the tag
  commit, not at the branch tip. Re-point the tag after any workflow change:
    ```sh
    git push O :refs/tags/Aphrodite/v<ver>   # delete remote tag
    git tag -d Aphrodite/v<ver>              # delete local
    git tag -a Aphrodite/v<ver> -m "..."     # re-create at the fixed HEAD
    git push O Aphrodite/v<ver>
    gh workflow run Build.yml -R O/R --ref Aphrodite/v<ver>
    ```
    A stale tag silently re-runs the broken pipeline - same runner, same
    failure - and the new run's logs look identical to the old ones.
- **Dispatch ref must be the tag for tag-gated jobs.** Release/Finalize jobs
  guarded by `if: startsWith(github.ref, 'refs/tags/...')` only execute when
  the dispatch ref IS a tag; dispatching on a branch skips them silently.
  Always `--ref <tag>`.
- **`concurrency: group: <wf>-${{ github.ref }}` + `cancel-in-progress: true`
  auto-cancels the superseded run** when you re-trigger on the same ref -
  use it instead of manual `gh run cancel` for stale runs.
- **`gh` `--json` output is camelCase** (`tagName`, `databaseId`,
  `displayTitle`, `conclusion`) - the REST-docs snake_case names yield empty
  jq output, not an error.

## Separated release pattern (release-created-once + per-leg attach)

The dependable multi-platform layout (mirrors the PlayForm/Aphrodite
Build.yml):

1. **Release job** - tag-gated, runs FIRST, creates the release exactly once
   via softprops/action-gh-release with NO files. Concurrent matrix legs
   attaching in parallel would race to create it.
2. **Build matrix legs** - each `needs: Release`, `if: always() &&
(Release success || skipped)`, `fail-fast: false`. Each leg builds,
   packages its own tarballs, generates its OWN checksum file
   `SHA256SUMS-<target>.txt` (sha256sum/shasum), uploads them as an artifact,
   and attaches ONLY its own files to the release via its own
   softprops/action-gh-release step with `fail_on_unmatched_files: true`.
   No combined root SHA256SUMS, no download-artifact merge (the merge
   collides on a shared filename across legs).
3. **Finalize job** - `needs: [Build]`, tag-gated; lists release assets via
   `gh release view` and `exit 1` with `::error::` per missing target. A
   partial upload fails loudly rather than shipping silently.

**Installer/checksum consistency rule:** any consumer that downloads release
assets must be updated to the per-target `SHA256SUMS-<target>.txt` layout in
the same change as the workflow, then proven with a real end-to-end install
(`BINARY_DIR="$(mktemp -d)" bash download.sh` and the binary's
`--version`) - a checksum filename mismatch is a silent 404 that only the live
run reveals.

## Post-release verification (after the tag is pushed)

1. `gh api repos/O/R/releases --paginate` → tag present, published_at set.
2. `gh api repos/O/R/releases/tags/Aphrodite/v<ver>/assets` (or the release
   view) → all expected per-target assets + SHA256SUMS attached. If the
   workflow has a Finalize/verify job, its green run is corroboration, not the
   proof.
3. Re-run the installer script → now downloads, verifies the checksum, and
   exits 0.
4. Record the release URL in the release notes / verification record as
   verified, and note whether the submodule/pointer updates still need
   committing.
