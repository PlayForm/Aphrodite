# macOS / Linux: Install

Everything below also works on Windows if you have Git Bash, WSL, or MSYS -
these are all POSIX shell scripts. If you're on native PowerShell/`cmd.exe`,
use [Windows install](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/windows.md) instead - `download.ps1` is the direct
PowerShell equivalent of `download.sh`.

## Option 1: Hermes plugin, auto-download (recommended for most users)

```bash
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/plugins/aphrodite
hermes plugins enable aphrodite
hermes
```

On first launch, the plugin looks for a binary/dylib under `binaries/`; if
missing, run `download.sh` yourself first:

```bash
cd ~/.hermes/plugins/aphrodite   # or your Aphrodite-Hermes clone
bash download.sh                 # auto-detects version + platform
```

`download.sh` resolves the version to fetch automatically (a bundled version
file, the monorepo's own version, or the latest published release) and
detects your platform automatically too. No Rust toolchain needed. If the
proxy never comes up, see [Troubleshooting](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/troubleshooting.md#proxy-doesnt-auto-launch).

## Option 2: `cargo install` + `aphrodite setup`

If you have a Rust toolchain, `aphrodite setup` provisions the binary, dylibs,
and `aphrodite.toml` under `~/.hermes/aphrodite/`. It never links the plugin
into Hermes - the Hermes plugin is a git folder (Option 1) by design. The two
options are alternatives: git clone OR cargo install - you don't need both.

```bash
cargo install aphrodite aphrodite-hermes
aphrodite setup --api-key sk-... --api-url https://api.deepseek.com --model deepseek-v4-pro
```

To also use the Hermes plugin, follow Option 1's link step (clone the plugin
repo and `ln -s` it into `~/.hermes/plugins/aphrodite`); `aphrodite setup`
prints the exact command.

What `aphrodite setup` does, in order:

| Step | Action                                                                                                                        |
| ---- | ----------------------------------------------------------------------------------------------------------------------------- |
| 1    | Verifies `hermes` is on `PATH` - fails fast with a clear error if not                                                         |
| 2    | Refuses to run twice unless you pass `--force`                                                                                |
| 3    | Copies itself into `~/.hermes/aphrodite/binaries/` with tightened permissions                                                 |
| 4    | Finds and copies both dylibs from nearby build/install locations - errors out naming the missing one if none are found        |
| 5    | Writes `~/.hermes/aphrodite/aphrodite.toml` from a template (ports from `--cache-port`/`--token-port`, default `9797`/`9798`) |
| 6    | Writes `plugin.yaml` and a thin plugin shim                                                                                   |
| 7    | Runs `hermes plugins enable aphrodite`                                                                                        |

Useful flags: `--cache-port`/`--token-port` (run multiple concurrent Hermes
Agents on one machine, each pointed at its own port pair), `--no-launch`
(skip auto-starting the proxy after setup), `--force` (re-run setup over an
existing install). The plugin link itself is not part of setup - run the
`ln -s` from the command block above (or its junction/copy equivalent on
Windows) after setup completes.

### macOS Gatekeeper handling

On macOS, every artifact copy in `aphrodite setup` (binary, dylibs, and the
`target/release` dev-build fallback) goes through one Gatekeeper-safe path
(v1.3.2): `ditto` (metadata-preserving copy), falling back to `fs::copy` +
`xattr -c` (clear quarantine), then `install_name_tool -id @rpath/<name>`
and an explicit ad-hoc `codesign -f -s -` re-sign for dylibs. Every step
that fails prints a specific warning instead of being silently swallowed -
a machine missing Xcode Command Line Tools previously got a
Gatekeeper-SIGKILLed binary with zero diagnostic, and a failed `ditto` used
to be treated as success.

## Option 3: From source (monorepo, full dev setup)

```bash
git clone https://github.com/PlayForm/Aphrodite.git
cd Aphrodite
git submodule update --init --recursive  # required - vendored deps live in submodules
cargo build --release -p aphrodite -p aphrodite-hermes
# Binary: target/release/aphrodite
# Dylibs: target/release/libaphrodite.dylib, target/release/libaphrodite_hermes.dylib (or .so on Linux)
```

Then either:

| Approach                      | What it does                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Let the plugin install itself | Build the crates, then enable the plugin - installation and symlinking are handled by the plugin itself (it self-heals its `~/.hermes/` layout on launch; there is no separate installer script). Point `APHRODITE_BINARY_PATH`/`APHRODITE_HERMES_DYLIB_PATH` at your `target/{debug,release}/` build output, or copy it into the plugin's `binaries/` |
| Wire things up manually       | Symlink the plugin directory yourself, then point `APHRODITE_BINARY_PATH`/`APHRODITE_HERMES_DYLIB_PATH` at your `target/{debug,release}/` build output instead of copying files around                                                                                                                                                                 |

## What changes after any of these

```
~/.hermes/
├── plugins/
│   └── aphrodite/          ← manual symlink (or junction/copy on Windows) to the plugin source
├── aphrodite/
│   ├── aphrodite            ← binary (auto-downloaded, hand-placed, or built)
│   ├── aphrodite.toml        ← proxy/engine config (written by `aphrodite setup`)
│   └── ccr.db                ← SQLite CCR store (created on first run)
```

Two proxy processes come up on `:9797` (cache) and `:9798` (token) once
Hermes launches the plugin (or once you launch `aphrodite` yourself - see
[Troubleshooting](https://github.com/PlayForm/Aphrodite/tree/Development/docs/install/troubleshooting.md)).

## Uninstall

```bash
hermes plugins disable aphrodite
rm ~/.hermes/plugins/aphrodite
pkill -f "aphrodite" 2>/dev/null || true   # stop any proxy still running
```

If you used `aphrodite setup`, also remove `~/.hermes/aphrodite/` to fully
clean up the binaries/config it wrote.
