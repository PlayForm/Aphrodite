# Windows Install

Native Windows (PowerShell) is a first-class install path - `download.ps1` is
the direct PowerShell equivalent of the Unix `download.sh` script, so Git
Bash/WSL are no longer required. This page covers the fast path first, then
the fully manual walkthrough for anyone who wants to see (or needs to do)
every step by hand.

## Fast path: Hermes plugin + `download.ps1`

```powershell
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
cd Aphrodite-Hermes
pwsh ./download.ps1                                    # fetch the binary + dylib
mkdir "$env:USERPROFILE\.hermes\plugins" -Force
New-Item -ItemType Junction -Path "$env:USERPROFILE\.hermes\plugins\aphrodite" -Target (Get-Location)
hermes plugins enable aphrodite
hermes
```

`download.ps1` auto-detects the version and your platform, same as
`download.sh` - no arguments needed (`-Version` and `-Target` pin them
explicitly). It works in both PowerShell 5.1 (built into Windows) and
PowerShell 7+ (`pwsh`). It writes the binary and dylib into
`%USERPROFILE%\.hermes\aphrodite\binaries\` - the canonical runtime home the
plugin resolves on launch - validating them against the in-tree
`SHA256SUMS.txt`. The plugin **never downloads**: if you skip the script,
`register()` logs the setup command and the plugin stays disabled until the
binaries are present.

If you're working from a local monorepo clone instead, there is no separate
installer script to run - build the crates with cargo, then let the plugin
handle installation itself. The manual walkthrough below shows each step by
hand.

## Option 2: `cargo install` + `aphrodite setup`

`aphrodite setup` provisions the binary, dylibs, and `aphrodite.toml` under
`%USERPROFILE%\.hermes\aphrodite\`, registers the plugin with Hermes, and
prints the one manual step left: linking the plugin into Hermes.

```powershell
cargo install aphrodite
aphrodite setup --api-key sk-... --api-url https://api.deepseek.com --model deepseek-v4-pro
```

The link step is a manual follow-up: either a directory junction (no admin
rights needed on modern Windows) or a copy of the runtime home:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.hermes\plugins" | Out-Null
New-Item -ItemType Junction -Path "$env:USERPROFILE\.hermes\plugins\aphrodite" -Target "$env:USERPROFILE\.hermes\aphrodite"
```

By default setup ends by starting the proxy; pass `--no-launch` to skip
that. Flags: `--cache-port`/`--token-port` (run multiple concurrent Hermes
Agents, each on its own port pair), `--force` (rewrite an existing
`aphrodite.toml`). See
[macOS/Linux: what `aphrodite setup` does](macos-linux.md#option-2-cargo-install--aphrodite-setup)
for the full step-by-step.

## Manual walkthrough

Use this if the fast path doesn't apply to you - no network access to GitHub
Releases, building from source, or you just want to understand each step.

### Step 1: Get the plugin source

```powershell
cd G:\AI\Hermes                       # any working directory you like
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
```

### Step 2: Link the plugin into Hermes

Hermes discovers plugins from `%USERPROFILE%\.hermes\plugins\<name>`. Prefer
a directory junction (needs no admin rights on modern Windows, unlike a real
symlink) so edits to your clone are picked up without recopying:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.hermes\plugins" | Out-Null
New-Item -ItemType Junction -Path "$env:USERPROFILE\.hermes\plugins\aphrodite" -Target "G:\AI\Hermes\Aphrodite-Hermes"
```

If that refuses (some locked-down environments still restrict junctions),
copy instead - just remember you'll need to re-copy after every plugin
update:

```powershell
Copy-Item -Recurse "G:\AI\Hermes\Aphrodite-Hermes" "$env:USERPROFILE\.hermes\plugins\aphrodite"
```

### Step 3: Get the binary and dylib

The plugin resolves them from `%USERPROFILE%\.hermes\aphrodite\binaries\`
but **never downloads** - `register()` only checks presence and logs the
setup command when they're missing. Fetch them ahead of time with one of:

| Option                          | How                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `download.ps1` (explicit setup) | `pwsh ./download.ps1` from inside your plugin clone (see [Fast path](#fast-path-hermes-plugin--downloadps1)) - writes to `%USERPROFILE%\.hermes\aphrodite\binaries\`, validated against the in-tree `SHA256SUMS.txt`                                                                                                                                                                      |
| Download by hand                | Go to the [releases page](https://github.com/PlayForm/Aphrodite/releases), find the tag matching the version you want (tags look like `Aphrodite/v1.4.6`), download `aphrodite-x86_64-pc-windows-msvc.exe` and `libaphrodite_hermes-x86_64-pc-windows-msvc.dll`, place both in `%USERPROFILE%\.hermes\aphrodite\binaries\`, and rename them to `aphrodite.exe` and `aphrodite_hermes.dll` |
| Build from source               | `git submodule update --init --recursive && cargo build --release -p aphrodite -p aphrodite-hermes`, then copy `target\release\aphrodite.exe` and `target\release\aphrodite_hermes.dll` into `%USERPROFILE%\.hermes\aphrodite\binaries\`                                                                                                                                                  |

If none of these ran and you skip straight to enabling the plugin,
`register()` never downloads: it logs the explicit setup command and the
plugin stays disabled until the binaries are present - see
[Troubleshooting](troubleshooting.md#proxy-doesnt-auto-launch).

### Step 4: Enable the plugin

```powershell
cd G:\AI\Hermes\hermes-agent
venv\Scripts\hermes.exe plugins enable aphrodite
```

Answer `Y` if prompted to allow tool overrides.

### Step 5: Verify without launching a full session (optional but recommended)

Before trusting Hermes to launch the proxy for you, confirm the binary itself
runs. This does **not** require a real upstream API key or a Hermes session -
see [Troubleshooting: verify the proxy without Hermes](troubleshooting.md#verify-the-proxy-without-hermes)
for the placeholder-key pattern and what a healthy response looks like.

### Step 6: Configure

Two different files, two different repos - see
[Troubleshooting: two config files](troubleshooting.md#two-separate-config-files)
if this is confusing. On the Hermes side (`config.yaml`), the plugin needs
to be enabled and, optionally, wired as the context engine:

```yaml
plugins:
    enabled:
        - aphrodite
context:
    engine: aphrodite
    engine_threshold_pct: 55
```

Proxy-side tuning (ports, thresholds, preview style) lives in
`%USERPROFILE%\.hermes\aphrodite\aphrodite.toml`, not `config.yaml` - see
[aphrodite.toml Configuration](../config/aphrodite-toml.md) for the full
schema. If you need Hermes and the proxy to disagree with the compiled-in
defaults (`:9797`/`:9798`), set `cache_port`/`token_port` there or via
`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`.

### Step 7: Launch

```powershell
cd G:\AI\Hermes\hermes-agent
venv\Scripts\hermes.exe gateway run
```

If the plugin's own auto-launch doesn't bring the proxy up, start it
yourself in a separate terminal before Hermes:

```powershell
cd "$env:USERPROFILE\.hermes\aphrodite\binaries"
.\aphrodite.exe --api-key sk-placeholder
```

For unattended startup, launch the proxy first and give it a moment before
starting Hermes:

```powershell
Start-Process "$env:USERPROFILE\.hermes\aphrodite\binaries\aphrodite.exe" -ArgumentList "--api-key sk-placeholder"
Start-Sleep -Seconds 3
Set-Location G:\AI\Hermes\hermes-agent
Start-Process "venv\Scripts\hermes.exe" -ArgumentList "gateway run"
```

## Final directory layout

```
G:\AI\Hermes\
├── Aphrodite-Hermes\          ← plugin clone (pure loader: no binaries, no config)
│   ├── __init__.py
│   ├── plugin.yaml
│   ├── download.ps1
│   └── README.md
├── hermes-agent\
│   ├── venv\Scripts\hermes.exe
│   └── config.yaml
└── %USERPROFILE%\.hermes\
    ├── plugins\aphrodite      ← junction to the plugin clone
    └── aphrodite\             ← canonical runtime home
        ├── aphrodite.toml     ← proxy/engine config
        ├── binaries\
        │   ├── aphrodite.exe          ← proxy binary
        │   └── aphrodite_hermes.dll   ← dylib the Python loader ctypes-loads
        └── ccr.db             ← SQLite CCR database (created on first run)
```

## Uninstall

```powershell
hermes plugins disable aphrodite
Remove-Item "$env:USERPROFILE\.hermes\plugins\aphrodite" -Recurse
Get-Process aphrodite -ErrorAction SilentlyContinue | Stop-Process   # stop any proxy still running
```

Also remove `%USERPROFILE%\.hermes\aphrodite\` to fully clean up the
binaries, config, CCR database, and logs that the download scripts or
`aphrodite setup` wrote.
