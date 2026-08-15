# Publishing `vendor/headroom` (`aphrodite-headroom-core`)

## Facts
- `vendor/headroom` is a git submodule → `PlayForm/Headroom.git`, branch `Current`.
- Publishable crate: `crates/headroom-core/Cargo.toml`, published under package
  name `aphrodite-headroom-core` (its `name` field). This is the live crate on
  crates.io (`https://index.crates.io/ap/hr/aphrodite-headroom-core`).
- Parent `Publish.yml` job `Publish-Headroom-Core` runs `working-directory: vendor/headroom`
  and publishes BEFORE `Publish-Aphrodite` (hard `needs:`).

## Publish procedure (no rename — name is `aphrodite-headroom-core`)
1. Bump `version` in `crates/headroom-core/Cargo.toml` (and the parent pin in
   `crates/aphrodite/Cargo.toml`) when the fork changes. The package name stays
   `aphrodite-headroom-core` — do not rename it.
2. Sibling crates reference it via
   `headroom-core = { package = "aphrodite-headroom-core", path = "../headroom-core" }`
   (headroom-ffi, headroom-parity, headroom-proxy, headroom-py).
3. Parent `crates/aphrodite/Cargo.toml` depends on it via
   `headroom-core = { package = "aphrodite-headroom-core", path = "../../vendor/headroom/crates/headroom-core", version = "0.1.2", default-features = false }`.
4. `Cargo.lock` regenerates on next `cargo build` (it tracks the package name);
   no manual edit needed.
5. Commit in the submodule, push to `Source/Current`. Then update the parent
   gitlink (below) so CI publishes the correct tree.

## Gitlink trap (critical)
CI checks out the submodule at the **parent's recorded gitlink**, not the local
submodule HEAD. To publish a changed headroom tree:
```
cd <Aphrodite>
git add vendor/headroom
git commit -m "build(submodule): point vendor/headroom at <new headroom commit>"
git push Source Current
```
If you skip this, `Publish.yml` publishes the previously-recorded commit's tree.

## Pre-publish verification (read-only)
- crates.io index: `https://index.crates.io/ap/hr/aphrodite-headroom-core`
  (`404` = not published → CI will attempt; `200` + `"vers":"X.Y.Z"` = live → CI skips).
- `cargo publish` needs `CARGO_REGISTRY_TOKEN` (CI secret). Never run locally
  without it.

## Trigger
```
gh workflow run Publish -f publish_crates=true
```
The headroom publish step is gated on `publish_crates == true`; without it,
`Publish-Aphrodite` fails (missing `aphrodite-headroom-core` dependency on crates.io).
