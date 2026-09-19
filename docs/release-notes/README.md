# Release Notes

Release notes for every Aphrodite binary and Hermes plugin release. Each note
summarizes what shipped in that version: headline features, fixes, breaking
changes, and the platforms covered by the release artifacts.

## Versioning

The binary and the Hermes plugin version independently. The binary - the CCR
compression proxy and dylib - is versioned `1.4.x`; the plugin
(`plugins/aphrodite`, a loader/registration shim) is versioned `2.1.x`. A
release ships both, and the plugin pins the binary version it expects via
`BINARY_VERSION`.

## Releases

| Version             | Plugin | Summary                                                                                                           |
| ------------------- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| [v1.4.0](v1.4.0.md) | 2.1.0  | Packaging and runtime-cache lifecycle fixes: portable directives, bounded hot-reload storage, manifest alignment. |
| [v1.4.1](v1.4.1.md) | 2.1.1  | Corrective release: CI build/lint and release-tooling fixes, no engine changes.                                   |
| [v1.4.2](v1.4.2.md) | 2.1.2  | Corrective release: experimental navigation crates removed from the build and audit surface.                      |
| [v1.4.3](v1.4.3.md) | 2.1.3  | Directives-path override, Windows multi-home dylib reuse, install auto-download, benchmark tooling.               |
| [v1.4.4](v1.4.4.md) | 2.1.3  | Setup hotfix: `cargo install` completes without the core dylib, which is downloaded at runtime.                   |
| [v1.4.5](v1.4.5.md) | 2.1.3  | Shipped config templates refreshed: provider-specific defaults dropped.                                           |
| [v1.4.6](v1.4.6.md) | 2.1.4  | Chain-split CCR engine, honest previews, generated FFI bindings, layout self-heal, installer removal.             |

## Platforms

Every release ships the same four binary targets plus the Hermes plugin:

| Artifact                             | Platform       |
| ------------------------------------ | -------------- |
| `aphrodite-aarch64-apple-darwin`     | macOS ARM64    |
| `aphrodite-x86_64-apple-darwin`      | macOS Intel    |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64   |
| `aphrodite-x86_64-pc-windows-msvc`   | Windows x86_64 |
| Plugin                               | Hermes         |
