# Windows Install

Native Windows (PowerShell or `cmd.exe`) is now a first-class install path -
`download.ps1` and `install.ps1` are direct PowerShell equivalents of the
Unix `download.sh`/`install.sh` scripts, so Git Bash/WSL are no longer
required. This page covers the fast path first, then the fully manual
walkthrough for anyone who wants to see (or needs to do) every step by hand.

## Fast path

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
`download.sh` - no arguments needed. Works in both PowerShell 5.1 (built into
Windows) and PowerShell 7+ (`pwsh`).

If you're working from a local monorepo clone instead, `Maintain\install.ps1`
does the whole flow in one step - binary copy, plugin junction, skills
junction, all 7 profile junctions, and plugin registration:

```powershell
cargo build --release -p aphrodite -p aphrodite-hermes
pwsh Maintain\install.ps1
```

`Maintain\install.bat` calls the same script for `cmd.exe` users - it just
detects `pwsh`/`powershell` on your `PATH` and delegates to `install.ps1`, so
there's one script to keep working, not two.

## Manual walkthrough

Use this if the fast path doesn't apply to you - no network access to
GitHub Releases, building from source, or you just want to understand each
step.

### Step 1: Get the plugin source

```powershell
cd G:\AI\Hermes                       # any working directory you like
git clone https://github.com/PlayForm/Aphrodite-Hermes.git
```

### Step 2: Link the plugin into Hermes

Hermes discovers plugins from `%USERPROFILE%\.hermes\plugins\<name>`. Prefer a
directory junction (needs no admin rights on modern Windows, unlike a real
symlink) so edits to your clone are picked up without recopying:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.hermes\plugins" | Out-Null
New-Item -ItemType Junction -Path "$env:USERPROFILE\.hermes\plugins\aphrodite" -Target "G:\AI\Hermes\Aphrodite-Hermes"
```

If that refuses (some locked-down environments still restrict junctions),
copy instead - just remember you'll need to re-copy after every plugin update:

```powershell
Copy-Item -Recurse "G:\AI\Hermes\Aphrodite-Hermes" "$env:USERPROFILE\.hermes\plugins\aphrodite"
```

### Step 3: Get the binary and dylib

Pick one:

| Option            | How                                                                                                                                                                                                                                                                                                                                                                        |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Auto-download     | `pwsh ./download.ps1` from inside your plugin clone (see [Fast path](#fast-path))                                                                                                                                                                                                                                                                                          |
| Download by hand  | Go to the [releases page](https://github.com/PlayForm/Aphrodite/releases), find the tag matching the version you want (tags look like `Aphrodite/v1.3.2`), download `aphrodite-x86_64-pc-windows-msvc.exe` and `libaphrodite_hermes-x86_64-pc-windows-msvc.dll`, place both in `Aphrodite-Hermes\binaries\`, and rename them to `aphrodite.exe` and `aphrodite_hermes.dll` |
| Build from source | `git submodule update --init --recursive && cargo build --release -p aphrodite -p aphrodite-hermes`, then copy `target\release\aphrodite.exe` and `target\release\aphrodite_hermes.dll` into `Aphrodite-Hermes\binaries\`                                                                                                                                                  |

If none of these ran and you skip straight to enabling the plugin, Hermes
will try to auto-download for you on first launch - if that doesn't work,
see [Troubleshooting](troubleshooting.md#proxy-doesnt-auto-launch).

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
if this is confusing. On the Hermes side (`config.yaml`), the plugin needs to
be enabled and, optionally, wired as the context engine:

```yaml
plugins:
    enabled:
        - aphrodite
context:
    engine: aphrodite
    engine_threshold_pct: 55
```

Proxy-side tuning (ports, thresholds, preview style) lives in `aphrodite.toml`,
not `config.yaml` - see [aphrodite.toml Configuration](../config/aphrodite-toml.md)
for the full schema. If you need Hermes and the proxy to disagree with the
compiled-in defaults (`:9797`/`:9798`), set `cache_port`/`token_port` there or
via `APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`.

### Step 7: Launch

```powershell
cd G:\AI\Hermes\hermes-agent
venv\Scripts\hermes.exe gateway run
```

If the plugin's own auto-launch doesn't bring the proxy up, start it yourself
in a separate terminal before Hermes:

```powershell
cd G:\AI\Hermes\Aphrodite-Hermes\binaries
.\aphrodite.exe --api-key sk-placeholder
```

For unattended startup, launch the proxy first and give it a moment before
starting Hermes:

```powershell
Start-Process "G:\AI\Hermes\Aphrodite-Hermes\binaries\aphrodite.exe" -ArgumentList "--api-key sk-placeholder"
Start-Sleep -Seconds 3
Set-Location G:\AI\Hermes\hermes-agent
Start-Process "venv\Scripts\hermes.exe" -ArgumentList "gateway run"
```

## Final directory layout

```
G:\AI\Hermes\
├── Aphrodite-Hermes\
│   ├── binaries\
│   │   ├── aphrodite.exe          ← proxy binary
│   │   └── aphrodite_hermes.dll   ← dylib the Python loader ctypes-loads
│   ├── __init__.py
│   ├── plugin.yaml
│   ├── download.ps1
│   └── README.md
├── hermes-agent\
│   ├── venv\Scripts\hermes.exe
│   ├── config.yaml
│   └── .hermes\plugins\aphrodite  ← junction to Aphrodite-Hermes
└── .hermes\
    └── aphrodite\
        └── ccr.db                 ← SQLite CCR database (created on first run)
```
