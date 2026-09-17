# 09 - Release & Publish CI

Two workflows fire on an `Aphrodite/v*` tag push. `Build.yml` creates the
GitHub release once, then a 4-target matrix builds+attaches per-platform
assets, and a `Finalize` job asserts the matrix came out complete (12 assets:
4 targets × binary + dylib + SHA256SUMS). `Publish.yml` runs tests + a
packaging guard, then a 3-stage crates.io chain: on a **plain tag push** the
`aphrodite` and `aphrodite-hermes` publish steps DO fire, while
`aphrodite-headroom-core` (the vendored dependency published under our own
namespace) publishes only via explicit `workflow_dispatch` with
`publish_crates=true`.

> Correction vs the tracing brief: the "no-finalize-job gap" is **closed** -
> `Build.yml` now has a `Finalize` job (`Build.yml:205`) that fails loudly if
> any of the 12 expected assets are missing. Windows is still the long pole;
> `fail-fast: false` lets every leg finish attaching regardless. (No
> `.githooks`, installer scripts, or `profiles/` tree exist anymore - removed
> from Development in the 1.4.6 cycle; `Maintain/` no longer ships
> `install.sh`/`.ps1`/`.bat`.)

## Build.yml - tag push → release + 4-target matrix

```mermaid
flowchart TD
    T["push tag Aphrodite/v* OR workflow_dispatch"] --> R["Release job (Build.yml:53, environment: Release)"]
    R --> R1["softprops/action-gh-release - create release ONCE (before any attach)"]
    R1 --> M{"Build matrix (Build.yml:73, needs: Release; fail-fast:false)"}
    M --> B1["x86_64-unknown-linux-gnu (ubuntu)"]
    M --> B2["aarch64-apple-darwin (macos)"]
    M --> B3["x86_64-apple-darwin (macos, cross from arm64)"]
    M --> B4["x86_64-pc-windows-msvc (windows - LONG POLE)"]

    B1 --> S["cargo build --release -p aphrodite -p aphrodite-hermes --target T"]
    B2 --> S
    B3 --> S
    B4 --> S
    S --> ST["stage artifacts: aphrodite-T(.exe) + libaphrodite_hermes-T(.dylib/.so/.dll)"]
    ST --> CK{"checksums"}
    CK -->|windows| CW["pwsh Get-FileHash (Build.yml:154; bash findstr mangles /v flag)"]
    CK -->|unix| CU["shasum -a 256 (Build.yml:165)"]
    CW --> UP["upload-artifact + action-gh-release attach (fail_on_unmatched_files:true)"]
    CU --> UP

    UP --> F["Finalize job (Build.yml:205, needs: Build, tags only)"]
    F --> FV["gh release view - assert all 4 targets × 3 files = 12 assets"]
    FV -->|missing| FX["::error:: exit 1 - do NOT publish notes for incomplete matrix"]
    FV -->|complete| FOK["All 4 platforms present (Windows included)"]
```

## Publish.yml - test + packaging guard → crates.io chain

```mermaid
flowchart TD
    TT["push tag Aphrodite/v* OR workflow_dispatch"] --> TEST["Test job (Publish.yml:56)"]
    TEST --> T1["cargo test -p aphrodite -p aphrodite-hermes --release (Publish.yml:85)"]
    T1 --> T2["packaging guard: cargo package --list must contain<br/>all 6 builtin_directives/*.md (Publish.yml:94)<br/>- recursive *.md exclude once stripped them (v1.3.8)"]

    TEST --> PHC["Publish-Headroom-Core (Publish.yml:107, needs Test)"]
    PHC --> C0{"version already on crates.io? (check runs on tag AND dispatch, :143)"}
    C0 -->|published| SKIP1["skip - crates.io versions immutable"]
    C0 -->|not published| C1{"workflow_dispatch && publish_crates?"}
    C1 -->|no| SKIP2["skip - headroom-core is DISPATCH-ONLY (:157)"]
    C1 -->|yes| PUB1["cargo publish -p aphrodite-headroom-core --no-verify"]

    PUB1 --> PA["Publish-Aphrodite (Publish.yml:164, needs Test + Headroom-Core)"]
    SKIP2 --> PA
    SKIP1 --> PA
    PA --> C2{"tag push OR (dispatch && publish_crates)? (:197)"}
    C2 -->|yes| PUB2["cargo publish -p aphrodite --no-verify"]
    C2 -->|no| SKIP3["build only"]

    PUB2 --> PHh["Publish-Hermes (Publish.yml:203, needs Publish-Aphrodite)"]
    SKIP3 --> PHh
    PHh --> C3{"tag push OR (dispatch && publish_crates)? (:235)"}
    C3 -->|yes| PUB3["cargo publish -p aphrodite-hermes --no-verify"]
    C3 -->|no| SKIP4["build only"]
```

Ordering rationale: `aphrodite` path-depends on vendored `headroom-core`
(published under the alias `aphrodite-headroom-core`); `cargo publish` strips
the `path` key, so a matching `aphrodite-headroom-core` version must exist on
crates.io first - hence the strict `Headroom-Core → Aphrodite → Hermes` chain.
The tag-push publish condition on `aphrodite`/`aphrodite-hermes` means a
release tag genuinely publishes the two main crates; only the vendored
`aphrodite-headroom-core` publish step is truly dispatch-only (its
version-check step still runs on tag push so the chain is never blocked).

## Key call sites

- release-once + matrix + Finalize - `.github/workflows/Build.yml:53,73,205`
- Windows checksum PowerShell step - `.github/workflows/Build.yml:154`
- test → packaging guard → 3-stage publish chain - `.github/workflows/Publish.yml:56,94,107,164,203`
- headroom-core version-exists guard (tag AND dispatch) - `.github/workflows/Publish.yml:143`
- tag-push publish conditions (aphrodite + aphrodite-hermes) - `.github/workflows/Publish.yml:197,235`
