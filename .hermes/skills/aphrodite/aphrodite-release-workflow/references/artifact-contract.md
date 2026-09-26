# Artifact Contract Matrix - consumer paths vs published assets

Owner: `aphrodite-release-workflow` (section 4 of the owning SKILL.md).
Consumer paths are the ground truth for what a release must ship. The rule:
**consumer download names must match published release assets exactly**, and
the **installer must not fail if an optional artifact is absent** (degrade
with a warning - `aphrodite-boundaries` failure-policy "degrade", not
fail-closed). Build.yml stages, per matrix target: `aphrodite-<target>[.exe]`,
`libaphrodite_hermes-<target>.{so,dylib,dll}`, `SHA256SUMS-<target>.txt`
(4 targets × 3 = 12 assets; `Finalize` enforces all 12). Download base URL:
`https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v{version}`.

| Consumer path                   | Source (real paths)                                                                                                                                                  | Required asset                                                                                            | Optional asset                                                 | Missing-required result                                                   | Missing-optional result                                                                                         |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| macOS setup (`aphrodite setup`) | `crates/aphrodite/src/setup/macos.rs` (Gatekeeper-safe install), `setup/dylib.rs` (resolver: `("libaphrodite.dylib", false)`, `("libaphrodite_hermes.dylib", true)`) | `aphrodite-aarch64-apple-darwin` / `aphrodite-x86_64-apple-darwin` + `libaphrodite_hermes-<target>.dylib` | `libaphrodite.dylib` (core cdylib, external-embedder use only) | Clear setup failure with build-from-source / manual-download instructions | `WARNING: optional dylib ... continuing without it`                                                             |
| Plugin loader                   | `plugins/aphrodite/__init__.py` (ctypes load, `_ensure_binaries` auto-fetch) + `download.sh`                                                                         | `aphrodite-<target>` + `libaphrodite_hermes-<target>.{dylib,so}`                                          | `SHA256SUMS-<target>.txt`                                      | Clear load/install failure (`download.sh` exits 1)                        | Loud `WARNING: SHA256SUMS-<target>.txt not found - skipping checksum verification`; older tags stay installable |
| Windows setup                   | `plugins/aphrodite/download.ps1`                                                                                                                                     | `aphrodite-x86_64-pc-windows-msvc.exe` + `libaphrodite_hermes-x86_64-pc-windows-msvc.dll`                 | `SHA256SUMS-x86_64-pc-windows-msvc.txt`                        | Clear setup failure (throw / exit 1)                                      | Warning and continue                                                                                            |
| Linux setup                     | `plugins/aphrodite/download.sh`                                                                                                                                      | `aphrodite-x86_64-unknown-linux-gnu` + `libaphrodite_hermes-x86_64-unknown-linux-gnu.so`                  | `SHA256SUMS-x86_64-unknown-linux-gnu.txt`                      | Clear setup failure                                                       | Warning and continue                                                                                            |

`download.sh`/`download.ps1` also validate downloaded files (non-empty,
native magic bytes ELF/Mach-O/PE, exact-match SHA-256 when a sums file is
present) and restore any prior copy on failure - a checksum mismatch is a
hard error.

## Verification before tag publication

Run a **clean-install simulation or an artifact-name verification against
staged output** - a successful build must not become a failed first-run
setup:

```bash
# Option A - clean-install simulation (macOS/Linux):
BINARY_DIR="$(mktemp -d)" bash plugins/aphrodite/download.sh <version> <target>
"$HOME/.hermes/aphrodite/binaries/aphrodite" --version   # or the mktemp dir binary

# Option B - artifact-name verification against the live release:
gh release view "Aphrodite/v<version>" --repo PlayForm/Aphrodite --json assets \
  -q '.assets[].name' | grep -E 'aphrodite-|libaphrodite_hermes-|SHA256SUMS-'
```

Every consumer-required name in the matrix must appear in the asset list.
`BINARY_VERSION` is NOT bumped until this passes (ledger rule + release
boundary in `aphrodite-boundaries`).