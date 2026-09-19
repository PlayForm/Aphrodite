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

If you install via `hermes plugins enable aphrodite` and let everything
auto-download, you don't need this table. It matters the moment something
**doesn't** auto-download and you start following a generic "run the binary
manually" instruction - at that point, `aphrodite.exe --api-key sk-...` is a
real, correct command for the proxy binary, but running anything manually is
never required just to make the Hermes plugin work.

There's also a **third, distinct config file** worth naming up front: Hermes
Agent's own `config.yaml` (`providers.*`, `plugins.enabled`,
`context.engine`, ...) is not the same file as Aphrodite's `aphrodite.toml`
(`[[proxies]]`, `[compression]`, `[previews]`, `[prompts]`, ...). Different
processes, no shared keys. See
[Troubleshooting: two config files](troubleshooting.md#two-separate-config-files).

## Three ways to install

Only the plugin auto-download path ships native scripts: `download.sh` for
macOS/Linux and `download.ps1` for native Windows PowerShell. Both live in
the plugin, auto-detect your platform, and fetch the binary + dylib into the
canonical runtime home `~/.hermes/aphrodite/binaries/`. The other paths need
no installer script.

| Path                                             | Best for                                                                                                                                | How                                                                                                                                                  |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Hermes plugin, auto-download**                 | Everyday users on any platform                                                                                                          | Symlink the plugin into `~/.hermes/plugins/aphrodite`, then `hermes plugins enable aphrodite`; first launch fetches the binary + dylib automatically |
| **`cargo install aphrodite && aphrodite setup`** | Users with a Rust toolchain who want one command to bootstrap the binary, dylibs, and config (the plugin symlink is a manual follow-up) | [macOS/Linux](macos-linux.md#option-2-cargo-install--aphrodite-setup), [Windows](windows.md#option-2-cargo-install--aphrodite-setup)                 |
| **From source (monorepo)**                       | Working from a full checkout, building the Rust crates yourself                                                                         | Build with cargo, then let the plugin itself handle installation and symlinking (layout self-heal on launch)                                         |

## Guides

| Guide                                 | Covers                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [Windows install](windows.md)         | Fast path with `download.ps1`, plus the fully manual walkthrough                                       |
| [macOS/Linux install](macos-linux.md) | `download.sh`, `aphrodite setup`, building from source                                                 |
| [Troubleshooting](troubleshooting.md) | Proxy not auto-launching, verifying the proxy without a full Hermes session, the two-config-files trap |
