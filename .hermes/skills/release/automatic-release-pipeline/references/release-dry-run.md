# Release-script dry-run harness + crates.io Publish.yml

## The release scripts

Local release helper scripts (repo-level `Release/NPM.sh` and `Release/Cargo.sh`
conventions) follow the same flow:

1. `Fn "$@"` - version from arg 1 (sources `Fn/Argument/Version.sh`, exports `Version`)
2. source `~/.aliases` + `~/.functions` (real dotfiles; `bump_dependencies` runs
   `cargo upgrade` / `composer update` / `ncu -u` via `find_ignore`)
3. `cargo build` / `cargo build --release` / `cargo install --force --path .`
4. Workflow rewrite - rewrites `.github/workflows/*` (SHA-pinning action refs)
5. `git add .` / `git ecommit -m "$Version"` / `git sync`
6. tag dance: `git tag -d` → `git push --delete Source` → `git tag -s -m -a` → `git push --tags`
7. `gh release delete -y` → `gh release create --generate-notes`
8. `cargo publish` (Cargo.sh)

The tag argument is the FULL tag, e.g. `Aphrodite/v1.5.1` - it flows verbatim into
ecommit, tag, and release calls.

## DryRun harness (parameterized)

```bash
<release-scripts-dir>/DryRun.sh <package-dir> <version-tag> [NPM.sh|Cargo.sh]
```

- Sandbox: `git clone --local --no-hardlinks` into `~/.hermes/tmp/dryrun/sandbox`
  (throwaway; real repo never touched). Source remote URL is derived from the
  real repo's `Source` remote so `push --delete Source` shape is realistic.
- Runs `PATH="$BIN:$PATH" bash <copied release script> "$VERSION"`; every
  mutating command logs `[DRY] ...` to `actions.log` and is echoed.

### Stub taxonomy (the non-obvious parts)

- **git wrapper**: `exec`s real git ONLY for read-only subcommands
  (`status log diff show rev-parse describe config remote branch ls-files
cat-file for-each-ref symbolic-ref --version`); everything else (incl. `add`,
  `ecommit`, `sync`, `push`) logs `[DRY]`. `tag` logs only when `-d/-s/-a/-f`
  appears (mutation), execs real git otherwise.
- **generic stubs**: `dum ncu gh composer cargo npm pnpm` - all log + exit 0.
- **absolute-path helper stubs**: PATH stubs CANNOT intercept absolute
  invocations (e.g. `~/.../node_modules/.bin/Workflow`). A naive harness LEAKS
  because of this - it regenerated real workflows. The harness copies the script
  and python-rewrites absolute helper paths to bare names so the stubs win.
- **`find` stub**: neuters `clean_dependencies`' recursive `rm -rf` and
  `find_ignore ... -execdir` chains (bump_dependencies).
- Per-script Fn helper copies: `NPM.sh` → `Fn/{Argument/Version,Update/NPM,Document/NPM}.sh`;
  `Cargo.sh` → `Fn/Argument/Version.sh` only.

### Reading the result

A script without `set -e` reports only the LAST command's exit code - the `[DRY]`
log is the real result. Count every step, verify `$Version` appears in the
`ecommit`/`tag`/`release` args, and check the workflow-rewrite step was
intercepted (not command-not-found, not real).

## crates.io Publish.yml from GitHub (PlayForm/Aphrodite pattern)

```yaml
on:
    push:
        tags: ["Aphrodite/v*"]
    workflow_dispatch: # publish_crates bool input, default true
```

Jobs: `Test` (`cargo test --release`, needs `Cargo.lock` cache paths) →
`Publish` (needs Test, `environment: Release`): build release → determine plan →
`cargo publish --no-verify --allow-dirty`.

- Version extraction: `grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'`
  (works when the package `version` is the only line starting with `version`).
- Idempotency: `curl -sL https://crates.io/api/v1/crates/<crate>/$VERSION | grep -q '"num"'`
  → skip if already published.
- `--allow-dirty` is needed because `cargo build --release` updates `Cargo.lock`.

### crates.io auth: token vs trusted publishing - NOT interchangeable

Per-crate setting on crates.io decides which publish credential works:

- **No trusted publishing configured** → token path: `CARGO_REGISTRY_TOKEN`
  secret in the `Release` environment (the PlayForm/Aphrodite pattern).
- **Trusted publishing enabled** (crate Settings → Trusted Publishing, GitHub
  registration scoped to repo + workflow filename + optional environment) →
  token-based publish fails with `403 Forbidden: New versions of this crate can
only be published using Trusted Publishing`. Do NOT try to add a token
  fallback - the token is rejected outright.

For trusted publishing use the crates.io-documented GitHub pattern - NOT
cargo's native OIDC (it can print `error: no token found` and skip the exchange
even with `id-token: write` set):

```yaml
permissions:
    contents: write
    id-token: write # required for the OIDC exchange
...
- name: Get crates.io publish token (trusted publishing)
  uses: rust-lang/crates-io-auth-action@v1
  id: auth
- name: Publish to crates.io
  run: cargo publish --no-verify --allow-dirty
  env:
      CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
```

The registration's repository + workflow filename + environment must match the
workflow exactly (job `environment: Release` == registration's Environment).
The 403 message appears even when the registration exists; it only means the
token path is blocked, not that OIDC is misconfigured.

**Test-job gate**: a `cargo test` gate on a codegen'd crate fails on DOCTESTS,
not unit tests - generated `///` examples use `crate::` paths (invalid in
doctests, which are external crates) and bare identifiers (`Fn`, `Option`,
`DashMap`) that collide with std items. When the package has no test targets
(`autotests=false`, no `[[test]]`), scope the gate to skip doctests:
`cargo test --release --tests --bins` - it still compiles the shipped bins in
test mode and runs 0 tests, exit 0.

**New-repo prerequisite**: the `Release` environment (and a `CARGO_REGISTRY_TOKEN`
secret if the crate is NOT trusted-publishing-gated) are per-repo and do NOT
exist by default - `gh api repos/O/R/environments` returns `total_count: 0`.
Create before the first tag push, or the Publish job fails.

## Workflow-rewrite helper

A globally installed workflow-rewriter (`.../node_modules/.bin/Workflow`,
resolving to a published package's bundled rewriter module) is only
resolvable when its parent `node_modules/.bin` is on PATH - off PATH it is
command-not-found (a no-`set -e` script silently continues past that line).
It can resolve an OLDER action version than the tag currently referenced -
inspect the `# vX.Y.Z` annotation on the new SHA and flag downgrades to the
user before committing.
