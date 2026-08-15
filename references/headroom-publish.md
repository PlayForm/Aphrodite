# Publishing & renaming `vendor/headroom` (`aphrodite-headroom`)

## Facts
- `vendor/headroom` is a git submodule → `PlayForm/Headroom.git`, branch `Current`.
- Publishable crate: `crates/headroom-core/Cargo.toml`, published under package
  name `aphrodite-headroom` (its `name` field). The live crate on crates.io is the
  OLD name `aphrodite-headroom-core`; the renamed `aphrodite-headroom` is a
  brand-new (immutable) name the first time it publishes.
- Parent `Publish.yml` job `Publish-Headroom-Core` runs `working-directory: vendor/headroom`
  and publishes BEFORE `Publish-Aphrodite` (hard `needs:`).

## Rename procedure (Cargo.toml only — no version changes)
1. `crates/headroom-core/Cargo.toml`: `name = "aphrodite-headroom"` ↔ `aphrodite-headroom-core`.
2. In each sibling crate, flip the `package = "..."` reference:
   - `crates/headroom-ffi/Cargo.toml`
   - `crates/headroom-parity/Cargo.toml`
   - `crates/headroom-proxy/Cargo.toml`
   - `crates/headroom-py/Cargo.toml`
   (pattern: `headroom-core = { package = "aphrodite-headroom", path = "../headroom-core" }`)
3. Parent `crates/aphrodite/Cargo.toml`: flip the `package = "..."` in its
   `headroom-core = { package = "...", path = "../../vendor/headroom/crates/headroom-core", version = "0.1.2" }`.
4. `Cargo.lock` regenerates on next `cargo build` (it tracks the package name);
   no manual edit needed.
5. Commit in the submodule, push to `Source/Current`. Then update the parent
   gitlink (below) so CI publishes the renamed tree.

## Gitlink trap (critical)
CI checks out the submodule at the **parent's recorded gitlink**, not the local
submodule HEAD. To publish a changed headroom tree:
```
cd <Aphrodite>
git add vendor/headroom
git commit -m "build(submodule): point vendor/headroom at <new headroom commit>"
git push Source Current
```
If you skip this, `Publish.yml` publishes the previously-recorded commit's tree
(e.g. the pre-rename crate name).

## Pre-publish verification (read-only)
- crates.io index: `https://index.crates.io/ap/hr/aphrodite-headroom`
  (`404` = not published → CI will attempt; `200` + `"vers":"X.Y.Z"` = live → CI skips).
- `cargo publish` needs `CARGO_REGISTRY_TOKEN` (CI secret). Never run locally
  without it.

## Trigger
```
gh workflow run Publish -f publish_crates=true
```
The headroom publish step is gated on `publish_crates == true`; without it,
`Publish-Aphrodite` fails (missing `aphrodite-headroom` dependency on crates.io).
