# 09 - Release & Publish CI

Two workflows fire on a `Aphrodite/v*` tag push. `Build.yml` creates the GitHub
release once, then a 4-target matrix builds+attaches per-platform assets, and a
`Finalize` job asserts the matrix came out complete. `Publish.yml` runs tests
then a 3-stage crates.io chain that is opt-in (only via `workflow_dispatch` with
`publish_crates=true`).

> Correction vs the tracing brief: the "no-finalize-job gap" is **closed** in
> v1.3.4 - `Build.yml` now has a `Finalize` job (`Build.yml:205`) that fails
> loudly if any of the 12 expected assets are missing. Windows is still the
> long pole; `fail-fast: false` lets every leg finish attaching regardless.

## Build.yml - tag push → release + 4-target matrix

```mermaid
flowchart TD
    T["push tag Aphrodite/v*"] --> R["Release job (environment: Release)"]
    R --> R1["softprops/action-gh-release - create release ONCE (before any attach)"]
    R1 --> M{"Build matrix (needs: Release; fail-fast:false)"}
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
    CK -->|windows| CW["pwsh Get-FileHash (bash findstr mangles /v flag)"]
    CK -->|unix| CU["shasum -a 256"]
    CW --> UP["upload-artifact + action-gh-release attach (fail_on_unmatched_files:true)"]
    CU --> UP

    UP --> F["Finalize job (needs: Build)"]
    F --> FV["gh release view - assert all 4 targets × 3 files = 12 assets"]
    FV -->|missing| FX["::error:: exit 1 - do NOT publish notes for incomplete matrix"]
    FV -->|complete| FOK["All 4 platforms present (Windows included)"]
```

## Publish.yml - test → opt-in crates.io chain

```mermaid
flowchart TD
    TT["push tag Aphrodite/v* OR workflow_dispatch"] --> TEST["Test job: cargo test --workspace"]
    TEST --> PHC["Publish-Headroom-Core (needs Test)"]
    PHC --> C1{"workflow_dispatch && inputs.publish_crates?"}
    C1 -->|no| SKIP1["skip publish (tag-push builds stay green)"]
    C1 -->|yes| CK1["check index.crates.io for aphrodite-headroom-core version"]
    CK1 -->|already published| SKIP2["skip (crates.io versions immutable)"]
    CK1 -->|not published| PUB1["cargo publish -p aphrodite-headroom-core --no-verify"]

    PUB1 --> PA["Publish-Aphrodite (needs Test + Publish-Headroom-Core)"]
    SKIP2 --> PA
    SKIP1 --> PA
    PA --> C2{"opt-in?"}
    C2 -->|yes| PUB2["cargo publish -p aphrodite --no-verify"]
    C2 -->|no| SKIP3["build only"]

    PUB2 --> PHh["Publish-Hermes (needs Publish-Aphrodite)"]
    SKIP3 --> PHh
    PHh --> C3{"opt-in?"}
    C3 -->|yes| PUB3["cargo publish -p aphrodite-hermes --no-verify"]
    C3 -->|no| SKIP4["build only"]
```

Ordering rationale: `aphrodite` path-depends on vendored `headroom-core`
(published under the alias `aphrodite-headroom-core`); `cargo publish` strips the
`path` key, so a matching `aphrodite-headroom-core` version must exist on
crates.io first - hence the strict `Headroom-Core → Aphrodite → Hermes` chain,
each gated behind the same opt-in flag so a normal tag push never turns red on a
publish attempt.

## Key call sites
- release-once + matrix + Finalize - `.github/workflows/Build.yml:52,72,205`
- Windows checksum PowerShell step - `.github/workflows/Build.yml:154`
- test → 3-stage publish chain - `.github/workflows/Publish.yml:55,81,138,177`
- crates.io version-exists guard - `.github/workflows/Publish.yml:118`
