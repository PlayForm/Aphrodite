> **Draft** - retrospective fork release note for the planned 0.1.3
> publication; verification numbers below are real (run against
> vendor/headroom).

## Headroom Fork: aphrodite-headroom-core 0.1.3 · aphrodite-v0.10.0

### Summary

The PlayForm/Headroom fork's first crates.io publication since July: 0.1.3
carries everything the fork accumulated after the 0.1.2 bump (2026-07-14).
The headline is the upstream sync - headroomlabs-ai/headroom@main merged 396
behind / 31 ahead (43dc9836, 2026-08-07), preceded in range by an 87-commit
sync (b1932981, upstream/main c365c7ff) - which realigns the vendored crate
with upstream proxy policy extraction, Responses output shaping, provider
simulators, and C# CodeAwareCompressor. On top of the sync sit the fork's own
hardening: the `ml` feature defaults OFF (d9f12039), the ml-cluster dep pin
(84c8d117), package-name references fixed to `aphrodite-headroom-core`
(39ae7079), the vendored headroom submodule removed (1f80236c), and the Sep
18-19 nightly-toolchain syntax batch that keeps the fork compiling on the
same nightly/edition-2024 toolchain as the Aphrodite workspace
(5fda221a..02706ea1). Fork tag: `aphrodite-v0.10.0`.

### Changes

- **Upstream sync**: merge headroomlabs-ai/headroom@main (43dc9836, 396
  behind / 31 ahead, 2026-08-07) plus the preceding 87-commit sync
  (b1932981, upstream/main c365c7ff) already in range; fork-side fix
  4385995a stops the content detector misclassifying idiomatic Go source as
  build output.
- **Packaging/deps**: `ml` feature defaulted OFF (d9f12039); ml-cluster dep
  pinned to known-good (84c8d117: hf-hub 0.5, fastembed 5, ort rc.12);
  package-name references corrected from `aphrodite-headroom` to
  `aphrodite-headroom-core` (39ae7079); vendored headroom submodule removed
  (1f80236c); package.json version ranges pinned to exact versions
  (682ebd0d); transformers 5.3.0 -> 5.5.0 (e8ded481).
- **Nightly-toolchain migration**: `no_mangle` exports migrated to
  edition-2024 `unsafe` attribute syntax (2c6c68b3); closure signatures
  restored in kompress/search_compressor (be61a6de); em-dashes and clippy
  pattern bindings normalized (3a3573a1); nightly clippy lints suppressed in
  the vendored fork (02706ea1); rustfmt passes (4bc1d366).
- **Tests**: kompress parity test structure improved (5fda221a); parity
  fixtures recorded for kompress and code-aware compression.

### Verification

- Commit range analyzed: `c6b61470..02706ea1` (107 commits; `git rev-list --count`)
- Diffstat: `git diff --stat c6b61470..02706ea1` = 1021 files changed,
  +136220/-45505, dominated by the upstream sync (the range pulls the full
  upstream tree - Python proxy, docs, workflows - into the range); the
  fork's own commits are the ~20 in Changes above plus its merge history.

### What Ships

- `aphrodite-headroom-core` 0.1.3 published to crates.io (planned, at the
  1.5.1 release)
- Fork tag `aphrodite-v0.10.0` on PlayForm/Headroom at 02706ea1 (planned)

### Links

- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **Fork tag compare**: https://github.com/PlayForm/Headroom/compare/aphrodite-v0.9.4...aphrodite-v0.10.0
- **crates.io**: https://crates.io/crates/aphrodite-headroom-core
