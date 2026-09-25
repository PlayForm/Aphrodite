# Publishing Aphrodite Cargo crates from GitHub

Publishing from GitHub is NOT uniform across the fleet - always check
`.github/workflows/` in the target repo AND the crate's crates.io Trusted
Publishing setting before assuming a publish path exists.

## PlayForm/Aphrodite - the known-good pattern

`Publish.yml`:

- **Triggers**: `push: tags: ["Aphrodite/v*"]`, or `workflow_dispatch` with a
  boolean input `publish_crates` (default true). At the verified commit the
  tag push reaches `cargo publish` for `aphrodite` and `aphrodite-hermes`
  (their publish steps carry `|| startsWith(github.ref,
'refs/tags/Aphrodite/')`); `aphrodite-headroom-core` is
  `workflow_dispatch`-gated only. Re-verify the trigger table at the tag
  commit (Gate R7, `aphrodite-release-workflow`).
- **Jobs**: `Test` (`cargo test --release`) then the publish chain
  (`needs:`), running in GitHub `environment: Release` (holds the registry
  token).
- **Idempotency**: before publishing,
  `curl -sL https://crates.io/api/v1/crates/<crate>/<version>` and skip if the
  response already contains a `"num"` (version exists) - re-runs are safe.
- **Publish step**: `cargo publish --no-verify --allow-dirty` - `--no-verify`
  skips the pre-publish build (already built by the job), `--allow-dirty` is
  required because the earlier `cargo build --release` updates `Cargo.lock`,
  dirtying the tree.
- **Token**: `env: CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`
    - requires the secret configured on the `Release` environment.

## Trusted publishing (OIDC) - for crates.io-gated crates

Some PlayForm crates have Trusted Publishing ENABLED on crates.io, so
API-token publishing is rejected with `403 Forbidden: New versions of this
crate can only be published using Trusted Publishing`. Their publish
workflows use the OIDC pattern below instead of `CARGO_REGISTRY_TOKEN`.

Symptom: `cargo publish` fails with the 403 "... can only be published using
Trusted Publishing" (token sent) or "no token found" (no token and cargo's
native OIDC did not engage). The crates.io-documented pattern is an explicit
token exchange, not cargo's automatic flow:

```yaml
permissions:
    contents: write
    id-token: write # required for the OIDC exchange
jobs:
    Publish:
        environment: Release # must match the crates.io registration
        steps:
            - uses: rust-lang/crates-io-auth-action@v1
              id: auth
            - run: cargo publish --no-verify --allow-dirty
              env:
                  CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
```

- The crates.io registration (Settings -> Trusted Publishing) pins repo +
  workflow filename + environment; all three must match the job exactly.
- Cargo's native OIDC exchange does NOT reliably engage - it can print
  "no token found" even with `id-token: write` present. Always use
  `crates-io-auth-action` explicitly; do not ship a publish step with neither
  a token nor the auth action.
- A workflow fix after a failed run requires MOVING the tag: the workflow
  file that executes is the one at the tag's commit. `git tag -f` +
  `git push --force Source <tag>` re-triggers at the fixed commit.
- The Test job gates Publish; if the crate's `///` doc examples do not
  compile as doctests, scope the gate to `cargo test --release --tests --bins`.
- Verify the publish EXTERNALLY: crates.io API drops UA-less requests
  (non-JSON body), so send a User-Agent and read `max_version`:
  `curl -s -H "User-Agent: <contact>" https://crates.io/api/v1/crates/<name>`.

## Converting an existing token-based Publish.yml to trusted publishing

For a repo whose workflow already publishes with `CARGO_REGISTRY_TOKEN` and is
being moved to OIDC (crates.io 403s the API token once trusted publishing is
enabled on the crate):

1. Add `id-token: write` to the workflow `permissions:` block.
2. In EACH publish job, add `- uses: rust-lang/crates-io-auth-action@v1` with
   `id: auth` before the publish step (a multi-crate ordered chain keeps one
   auth step per job - steps do not share ids across jobs).
3. Replace the publish step's env with `CARGO_REGISTRY_TOKEN:
${{ steps.auth.outputs.token }}`.
4. Keep the existing opt-in semantics and the version-exists skip - they are
   orthogonal to the auth mechanism.
   Validate the conversion by parsing the workflow with `yaml.safe_load` and
   asserting every `cargo publish` step's env token comes from
   `steps.auth.outputs.token` and `id-token: write` is present. The crates.io
   Trusted Publishing registration (repo + workflow filename + environment) is
   the USER's crates.io-side step - say when the workflow side is ready, do not
   create the registration yourself.

- GitHub release notes: `gh release create <tag> --title ... --notes-file <f>`
  after the crates.io publish confirms; verify with `gh release view`.

## Wiring a publish workflow for a new crate

Mirror the Aphrodite pattern with the tag pattern changed to the crate's own
`<Crate>/v*` and the version-exists check pointed at the real crate name.
Confirm the crates.io crate name exists (`crates.io/api/v1/crates/<name>`)
before wiring CI - binary names can differ from crate names (e.g.
`aphrodite` vs the fork package `aphrodite-headroom-core`).
