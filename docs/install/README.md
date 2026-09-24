# Installing Aphrodite

"How do I install this" has three correct answers depending on your platform
and setup. This page picks the right one for you before you touch a terminal.

## Which artifact do you need?

Aphrodite ships two separate build artifacts from two separate crates. It's
easy to conflate them once you're past the happy path:

| Artifact                             | What it is                                                                                                                                                                                            | Needs an API key?                                                                                                                                                                                                                                                 |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aphrodite` (`.exe` on Windows)      | A standalone proxy binary. Runs as a subprocess and listens on `:9797` (cache) and `:9798` (token). Works with or without Hermes.                                                                     | Only when invoked **without** an `aphrodite.toml`: CLI mode requires `--api-key` or `APHRODITE_API_KEY`. With a config file present, the same [resolution chain](../config/aphrodite-toml.md#api-key-resolution) applies as a runtime check, not a required flag. |
| `libaphrodite_hermes.{dylib,so,dll}` | A dylib, **loaded in-process** by the Python plugin shim - not launched as a subprocess, has no CLI, takes no `--api-key`. The Hermes session that loads it already has its own model/API-key config. | No - it isn't a process, it can't take CLI args at all.                                                                                                                                                                                                           |

If you install via the Hermes plugin, the explicit setup step (`bash
download.sh` on macOS/Linux, `pwsh ./download.ps1` on Windows - or
`aphrodite setup`) fetches the binary + dylib for you, so you don't need
this table. It matters the moment you skip that step and start following a
generic "run the binary manually" instruction - at that point,
`aphrodite.exe --api-key sk-...` is a real, correct command for the proxy
binary, but running anything manually is never required just to make the
Hermes plugin work.

There's also a **third, distinct config file** worth naming up front: Hermes
Agent's own `config.yaml` (`providers.*`, `plugins.enabled`,
`context.engine`, ...) is not the same file as Aphrodite's `aphrodite.toml`
(`[[proxies]]`, `[compression]`, `[previews]`, `[prompts]`, ...). Different
processes, no shared keys. See
[Troubleshooting: two config files](troubleshooting.md#two-separate-config-files).

## Three ways to install

Only the plugin path ships native setup scripts: `download.sh` for
macOS/Linux and `download.ps1` for native Windows PowerShell. Both live in
the plugin, auto-detect the version and your platform, and fetch the binary

- dylib into the canonical runtime home `~/.hermes/aphrodite/binaries/`,
  validated against the in-tree `SHA256SUMS.txt`. This is an **explicit setup
  step** - `register()` never downloads. The other paths need no installer
  script.

| Path                                             | Best for                                                                                                                                | How                                                                                                                                                                                                                                        |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Hermes plugin + explicit setup**               | Everyday users on any platform                                                                                                          | Symlink the plugin into `~/.hermes/plugins/aphrodite`, run `bash download.sh` (or `pwsh ./download.ps1`), then `hermes plugins enable aphrodite` - `register()` never downloads                                                            |
| **`cargo install aphrodite && aphrodite setup`** | Users with a Rust toolchain who want one command to bootstrap the binary, dylibs, and config (the plugin symlink is a manual follow-up) | [macOS/Linux](macos-linux.md#option-2-cargo-install--aphrodite-setup), [Windows](windows.md#option-2-cargo-install--aphrodite-setup)                                                                                                       |
| **From source (monorepo)**                       | Working from a full checkout, building the Rust crates yourself                                                                         | Build with cargo, then point `APHRODITE_BINARY_PATH`/`APHRODITE_HERMES_DYLIB_PATH` at `target/{debug,release}/` or copy the build output into the runtime home's `binaries/` - the plugin never downloads and never self-heals the install |

## Guides

| Guide                                 | Covers                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [Windows install](windows.md)         | Fast path with `download.ps1`, plus the fully manual walkthrough                                       |
| [macOS/Linux install](macos-linux.md) | `download.sh`, `aphrodite setup`, building from source                                                 |
| [Troubleshooting](troubleshooting.md) | Proxy not auto-launching, verifying the proxy without a full Hermes session, the two-config-files trap |
