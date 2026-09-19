# Release & Publish CI

Two GitHub Actions workflows fire on an `Aphrodite/v*` tag push. `Build.yml` creates the GitHub release once, runs a 4-target build matrix that attaches per-platform assets, and a `Finalize` job that fails loudly if the matrix came out incomplete (12 assets: 4 targets x binary + dylib + SHA256SUMS). `Publish.yml` runs tests plus a packaging guard, then a 3-stage crates.io chain: on a plain tag push the `aphrodite` and `aphrodite-hermes` publish steps do fire, while `aphrodite-headroom-core` (the vendored dependency published under our own namespace) publishes only via explicit `workflow_dispatch` with `publish_crates=true`.

## Build.yml - tag push → release + 4-target matrix

```mermaid
flowchart TD
	T["push tag Aphrodite/v* OR workflow_dispatch"] --> R["Release job (environment: Release)"]
	R --> R1["softprops/action-gh-release - create release ONCE (before any attach)"]
	R1 --> M{"Build matrix (needs: Release; fail-fast: false)"}
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
	CW --> UP["upload-artifact + action-gh-release attach (fail_on_unmatched_files: true)"]
	CU --> UP

	UP --> F["Finalize job (needs: Build, tags only)"]
	F --> FV["gh release view - assert all 4 targets × 3 files = 12 assets"]
	FV -->|missing| FX["::error:: exit 1 - do NOT publish notes for incomplete matrix"]
	FV -->|complete| FOK["All 4 platforms present (Windows included)"]
```

## Publish.yml - test + packaging guard → crates.io chain

```mermaid
flowchart TD
	TT["push tag Aphrodite/v* OR workflow_dispatch"] --> TEST["Test job"]
	TEST --> T1["cargo test -p aphrodite -p aphrodite-hermes --release"]
	T1 --> T2["packaging guard: cargo package --list must contain<br/>all 6 builtin_directives/*.md<br/>- a recursive *.md exclude once stripped them"]

	TEST --> PHC["Publish-Headroom-Core (needs: Test)"]
	PHC --> C0{"version already on crates.io? (check runs on tag AND dispatch)"}
	C0 -->|published| SKIP1["skip - crates.io versions immutable"]
	C0 -->|not published| C1{"workflow_dispatch && publish_crates?"}
	C1 -->|no| SKIP2["skip - headroom-core is DISPATCH-ONLY"]
	C1 -->|yes| PUB1["cargo publish -p aphrodite-headroom-core --no-verify"]

	PUB1 --> PA["Publish-Aphrodite (needs: Test + Headroom-Core)"]
	SKIP2 --> PA
	SKIP1 --> PA
	PA --> C2{"tag push OR (dispatch && publish_crates)?"}
	C2 -->|yes| PUB2["cargo publish -p aphrodite --no-verify"]
	C2 -->|no| SKIP3["build only"]

	PUB2 --> PHh["Publish-Hermes (needs: Publish-Aphrodite)"]
	SKIP3 --> PHh
	PHh --> C3{"tag push OR (dispatch && publish_crates)?"}
	C3 -->|yes| PUB3["cargo publish -p aphrodite-hermes --no-verify"]
	C3 -->|no| SKIP4["build only"]
```

Ordering rationale: `aphrodite` path-depends on the vendored `headroom-core` (published under the alias `aphrodite-headroom-core`); `cargo publish` strips the `path` key, so a matching `aphrodite-headroom-core` version must already exist on crates.io - hence the strict Headroom-Core → Aphrodite → Hermes chain. The tag-push publish condition on `aphrodite`/`aphrodite-hermes` means a release tag genuinely publishes the two main crates; only the vendored `aphrodite-headroom-core` publish step is truly dispatch-only, and its version-check step still runs on tag push so the chain is never blocked.
