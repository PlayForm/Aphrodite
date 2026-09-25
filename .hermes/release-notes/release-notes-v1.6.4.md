**[Compare Aphrodite/v1.6.3...Aphrodite/1.6.4](https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.3...Aphrodite/1.6.4)**

## Aphrodite 1.6.4 💋 Plugin v2.2.4

### Summary

1.6.4 is a release-infra and plugin-hygiene release over 1.6.3. The shim's
runtime-home decision export moves from import time to `register()` - a
module import now has zero environment side effects, which fixes the
Development CI failures (CI runs every test in one process, and the
import-time export made `layout_check` heal against the real runner home).
The publish pipeline stops racing the build: `Publish.yml` now starts only
after the same tag's `Build.yml` run completed successfully (workflow_run
chaining), and `Build.yml`'s Finalize can finally push the in-tree checksums
to the child repo via the `PLAYFORM_RELEASE_PAT` secret (the 1.6.3 Finalize
403). Binary `1.6.3 → 1.6.4`, plugin `2.2.3 → 2.2.4`.

### Changes

- **Fix (plugin, import hygiene)**: the `APHRODITE_HOME` /
  `APHRODITE_DIRECTIVES_DIR` exports move from import time into a new
  `_export_runtime_home_env()` called at the top of `register()` - the dylib,
  directives materialize, and the proxy child still inherit the same
  runtime-home decision (issue 40 F1), but a mere `import` of the shim no
  longer mutates the process environment. Regression test
  `test_import_never_leaks_env`; the CI-shaped repro (shim import + full
  layout suite in one process) passes 15/15.
- **Chore (CI, publish chaining)**: `Publish.yml` switches from the tag-push
  trigger to `workflow_run` on Build.yml completion - publishing can never
  race or precede the build, and the gitlink bump can never run before
  Finalize pushed the child SHA256SUMS commit. Every Publish job is gated on
  `conclusion == 'success'` + the triggering tag; every checkout is pinned to
  that tag (the published code is exactly the code Build tested); the
  concurrency group is keyed on the tag so a faster later release can never
  cancel an in-flight publish chain.
- **Chore (CI, cross-repo child push)**: Build.yml's Finalize pushes the
  in-tree `SHA256SUMS.txt` to `PlayForm/Aphrodite-Hermes` with the
  `PLAYFORM_RELEASE_PAT` Release-environment secret (`GITHUB_TOKEN` is scoped
  to the parent repo and cannot push to the child - the 1.6.3 Finalize 403).
  The step drops the parent token's extraheader and fails loudly with an
  `::error::` when the secret is missing; the manual child-push fallback is
  no longer needed at release time.

### Infrastructure

- Python: `tests/test_runtime_home.py` ✅ 9/9 (48 asserts); `tests/
test_layout_check.py` ✅ 15/15 (50 asserts), standalone AND after a shim
  import in the same process (the CI poisoning repro)
- Lint/format: `ruff check plugins/aphrodite/` ✅; `ruff format --check` ✅;
  `npx prettier --check` on all changed docs/workflows ✅;
  `cargo +nightly-2026-05-01 fmt --all -- --check` ✅ (the pinned canonical
  toolchain)
- Rust suites: unchanged since 1.6.3's verification (434 passed / 0 failed
  aphrodite; 56 passed aphrodite-hermes) - the 1.6.4 delta is shim + workflow
  only
- Drift guard: `plugins/aphrodite/__init__.py` == `crates/aphrodite/
templates/__init__.py` == installed loader ✅
- Version track: crates + pin + `package.json` 1.6.4; `plugin.yaml` 2.2.4;
  `BINARY_VERSION` 1.6.4; README badges 1.6.4 / v2.2.4

### What Ships

Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact                                                             | Platform                                 |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target                        |
| `SHA256SUMS-<target>.txt`                                            | checksums for that target's two binaries |
| Plugin v2.2.4                                                        | Hermes (standalone repo)                 |

`aphrodite-headroom-core` stays at 0.1.3 (published with 1.5.1; the fork
holds 2 dependency-update commits post-0.1.3 - fastembed→5, crate version
refresh - carried in the fork's RELEASE-CYCLE ledger, no version bump in
this release).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v1.6.3...Aphrodite/1.6.4
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
- **crates.io**: https://crates.io/crates/aphrodite
