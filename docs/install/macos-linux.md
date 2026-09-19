# macOS / Linux: Install

Everything below also works on Windows if you have Git Bash, WSL, or MSYS -
these are all POSIX shell scripts. If you're on native PowerShell, use
[Windows install](windows.md) instead - `download.ps1` is the direct
PowerShell equivalent of `download.sh`.

## Option 1: Hermes plugin, auto-download (recommended for most users)

The plugin is a pure loader: it does **not** bundle the binary or the dylib.
`download.sh` lives inside the plugin and fetches them from GitHub Releases
into the canonical runtime home `~/.hermes/aphrodite/binaries/` - never into
the plugin directory.

```bash
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/plugins/aphrodite
hermes plugins enable aphrodite
hermes
```

On first launch the plugin checks `~/.hermes/aphrodite/binaries/`; if the
binary or the dylib is missing, it runs `download.sh` itself (SHA-256
verified, magic-byte checked) before starting the proxy. Run the script by
hand only when you want to prefetch or when the automatic fetch failed:

```bash
cd ~/.hermes/plugins/aphrodite   # your plugin clone
bash download.sh                 # auto-detects version + platform
```

`download.sh` resolves the version automatically (a bundled version file,
the monorepo's own Cargo.toml, or the latest published release) and detects
your platform too - no Rust toolchain needed. You can pin both explicitly:
`bash download.sh 1.4.6 x86_64-unknown-linux-gnu`. If the proxy never comes
up, see [Troubleshooting](troubleshooting.md#proxy-doesnt-auto-launch).

## Option 2: `cargo install` + `aphrodite setup`

If you have a Rust toolchain, `aphrodite setup` provisions the binary,
dylibs, and `aphrodite.toml` under `~/.hermes/aphrodite/`, registers the
plugin with Hermes, and prints the one manual step left: linking the plugin
into Hermes. The two options are alternatives - git clone OR cargo install,
you don't need both.

```bash
cargo install aphrodite
aphrodite setup --api-key sk-... --api-url https://api.deepseek.com --model deepseek-v4-pro
```

To also use the Hermes plugin, follow Option 1's link step (`ln -s
~/.hermes/aphrodite ~/.hermes/plugins/aphrodite`); `aphrodite setup` prints
the exact command. By default setup ends by starting the proxy; pass
`--no-launch` to skip that.

What `aphrodite setup` does, in order:

| Step | Action                                                                                                                                                                                                                                                |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Verifies `hermes` is on `PATH` (`hermes --version`) - fails fast with a clear error if not                                                                                                                                                            |
| 2    | Creates `~/.hermes/aphrodite/` and `~/.hermes/aphrodite/binaries/`                                                                                                                                                                                    |
| 3    | Copies the binary into `binaries/` with locked-down permissions (macOS copies go through a Gatekeeper-safe path, see below)                                                                                                                           |
| 4    | Locates `libaphrodite_hermes` (and the optional core `libaphrodite` dylib): local build/install dirs first, then the matching GitHub release with SHA-256 verification. A missing `libaphrodite_hermes` aborts setup; a missing core dylib only warns |
| 5    | Writes `~/.hermes/aphrodite/aphrodite.toml` from an embedded template (`api_url`, `model`, `--cache-port`/`--token-port`, defaults `9797`/`9798`) - only if none exists, unless you pass `--force`                                                    |
| 6    | Writes `plugin.yaml` and the `__init__.py` loader shim into the runtime home (always refreshed on re-run)                                                                                                                                             |
| 7    | Runs `hermes plugins enable aphrodite` - a failed registration prints a warning, it does not abort                                                                                                                                                    |
| 8    | Prints the manual link command (`ln -s ~/.hermes/aphrodite ~/.hermes/plugins/aphrodite`)                                                                                                                                                              |

Useful flags: `--cache-port`/`--token-port` (run multiple concurrent Hermes
Agents on one machine, each pointed at its own port pair), `--no-launch`
(skip auto-starting the proxy after setup), `--force` (rewrite an existing
`aphrodite.toml`; the binary, dylibs, and manifest are always refreshed on
re-run regardless). The plugin link itself is not part of setup - run the
`ln -s` from step 8 after setup completes.

### macOS Gatekeeper handling

On macOS, every artifact copy in `aphrodite setup` (binary, dylibs, and the
`target/release` dev-build fallback) goes through one Gatekeeper-safe path:
`ditto` (metadata-preserving copy), falling back to `fs::copy` + `xattr -c`
(clear quarantine), then `install_name_tool -id @rpath/<name>` and an
explicit ad-hoc `codesign -f -s -` re-sign for dylibs. Every step that fails
prints a specific warning instead of being silently swallowed - a machine
missing Xcode Command Line Tools previously got a Gatekeeper-SIGKILLed binary
with zero diagnostic, and a failed `ditto` used to be treated as success.

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

| Approach                      | What it does                                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Let the plugin install itself | Build the crates, then enable the plugin - installation and symlinking are handled by the plugin itself (it self-heals its `~/.hermes/` layout on launch; there is no separate installer script). Point `APHRODITE_BINARY_PATH`/`APHRODITE_HERMES_DYLIB_PATH` at your `target/{debug,release}/` build output, or copy it into the runtime home's `binaries/` |
| Wire things up manually       | Symlink the plugin directory yourself, then point `APHRODITE_BINARY_PATH`/`APHRODITE_HERMES_DYLIB_PATH` at your `target/{debug,release}/` build output instead of copying files around                                                                                                                                                                       |

## What changes after any of these

```
~/.hermes/
├── plugins/
│   └── aphrodite/          ← symlink (or junction/copy on Windows) to the plugin source - a pure loader
└── aphrodite/              ← canonical runtime home
    ├── aphrodite.toml      ← proxy/engine config (written by `aphrodite setup`)
    ├── binaries/
    │   ├── aphrodite       ← proxy binary (auto-downloaded, hand-placed, or built)
    │   └── libaphrodite_hermes.dylib   ← dylib the plugin loads (libaphrodite_hermes.so on Linux)
    ├── ccr.db              ← SQLite CCR store (created on first run)
    ├── directives/         ← active directive files
    ├── logs/               ← proxy/engine logs
    └── hotreload/          ← hot-reload dylib copies
```

On plugin startup the layout self-heals toward this schema: misplaced
config, binaries, or the CCR database are moved out of the plugin directory
into the runtime home, and a missing plugin symlink is recreated - the plugin
directory stays a pure loader. Two proxy processes come up on `:9797` (cache)
and `:9798` (token) once Hermes launches the plugin (or once you launch
`aphrodite` yourself - see
[Troubleshooting](troubleshooting.md#verify-the-proxy-without-hermes)).

## Uninstall

```bash
hermes plugins disable aphrodite
rm ~/.hermes/plugins/aphrodite
pkill -f "aphrodite" 2>/dev/null || true   # stop any proxy still running
```

Also remove `~/.hermes/aphrodite/` to fully clean up the binaries, config,
CCR database, and logs that the download scripts or `aphrodite setup` wrote.
