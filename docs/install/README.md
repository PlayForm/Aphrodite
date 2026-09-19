# Installing Aphrodite

"How do I install this" has three correct answers depending on your platform
and setup. This page picks the right one for you before you touch a terminal.

## Which artifact do you need?

Aphrodite ships two separate build artifacts from two separate crates. It's
easy to conflate them once you're past the happy path:

| Artifact                             | What it is                                                                                                                                                                                            | Needs an API key?                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aphrodite` (`.exe` on Windows)      | A standalone binary. Runs as a subprocess, listens on `:9797`/`:9798` as an HTTP proxy.                                                                                                               | Only when invoked **without** an `aphrodite.toml` next to it (falls back to CLI parsing, where `--api-key`/`APHRODITE_API_KEY` is required). With a config file present, the same [resolution chain](https://github.com/PlayForm/Aphrodite/tree/Current/docs/config/aphrodite-toml.md#api-key-resolution-chain) applies as a runtime check, not a required flag. |
| `libaphrodite_hermes.{dylib,so,dll}` | A dylib, **loaded in-process** by the Python plugin shim - not launched as a subprocess, has no CLI, takes no `--api-key`. The Hermes session that loads it already has its own model/API-key config. | No - it isn't a process, it can't take CLI args at all.                                                                                                                                                                                                                                                                                                              |

If you install via `hermes plugins enable aphrodite` and let everything
auto-download, you don't need this table. It matters the moment something
**doesn't** auto-download and you start following a generic "run the binary
manually" instruction - at that point, `aphrodite.exe --api-key sk-...` is a
real, correct command for the proxy binary, but running anything manually is
never required just to make the Hermes plugin work.

There's also a **third, distinct config file** worth naming up front: Hermes
Agent's own `config.yaml` (`providers.*`, `plugins.enabled`, `context.engine`,
...) is not the same file as Aphrodite's `aphrodite.toml`
(`[compression]`, `[previews]`, `[prompts]`, ...). Different processes, no
shared keys. See
[Troubleshooting: two config files](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/troubleshooting.md#two-separate-config-files).

## Three ways to install

Only the plugin auto-download path ships native scripts: a `.sh` for
macOS/Linux (and Windows via Git Bash/WSL) and a `.ps1` for native Windows
PowerShell - both auto-detect your platform, so you never need to pick a
target triple by hand. The other paths need no installer script.

| Path                                             | Best for                                                                                                                                | How                                                                                                                              |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Hermes plugin, auto-download**                 | Everyday users on any platform                                                                                                          | `download.sh` / `download.ps1` inside your plugin clone, then `hermes plugins enable aphrodite`                                  |
| **`cargo install aphrodite && aphrodite setup`** | Users with a Rust toolchain who want one command to bootstrap the binary, dylibs, and config (the plugin symlink is a manual follow-up) | [macOS/Linux](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/macos-linux.md#cargo-install--aphrodite-setup) |
| **From source (monorepo)**                       | Working from a full checkout, building the Rust crates yourself                                                                         | Build with cargo, then let the plugin itself handle installation and symlinking (layout self-heal on launch)                     |

## Guides

| Guide                                                                                                     | Covers                                                                                                 |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| [Windows install](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/windows.md)         | Fast path with `download.ps1`, plus the fully manual walkthrough                                       |
| [macOS/Linux install](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/macos-linux.md) | `download.sh`, `aphrodite setup`, building from source                                                 |
| [Troubleshooting](https://github.com/PlayForm/Aphrodite/tree/Current/docs/install/troubleshooting.md) | Proxy not auto-launching, verifying the proxy without a full Hermes session, the two-config-files trap |
