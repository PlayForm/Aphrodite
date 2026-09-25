# Windows-Specific Quirks

Use when diagnosing Hermes-on-Windows behavior - the Hermes host the Aphrodite
plugin runs inside is cross-platform, and users on Windows hit these quirks.
Aphrodite development itself is macOS-only (`platforms: [macos]`); this
reference documents the Windows host for user reports and for Windows-side
test guards, not for the Aphrodite dev loop.

Hermes runs natively on Windows (PowerShell, cmd, Windows Terminal, git-bash
mintty, VS Code integrated terminal). Most of it just works, but a handful of
differences between Win32 and POSIX have bitten us - document new ones here as
you hit them so the next person (or the next session) doesn't rediscover them
from scratch.

## Input / Keybindings

**Alt+Enter doesn't insert a newline** - Windows Terminal (and mintty) grab it
for fullscreen before prompt_toolkit sees it. Use **Ctrl+Enter** instead (the
CLI binds it to newline on Windows; raw Ctrl+J does the same, harmlessly).
To inspect how your terminal reports a keystroke, run
`python scripts/keystroke_diagnostic.py` from the repo root.

## Config / Files

**HTTP 400 "No models provided" on first run** - `config.yaml` was saved with
a UTF-8 BOM (Notepad does this). Re-save as UTF-8 without BOM;
`hermes config edit` writes correctly.

## execute_code / Sandbox

**WinError 10106** from the sandbox child process - it can't create an
`AF_INET` socket. Root cause is usually Hermes's env scrubber dropping
`SYSTEMROOT`/`WINDIR`/`COMSPEC` (Python's `socket` needs `SYSTEMROOT` to find
`mswsock.dll`), not a broken Winsock LSP. The `_WINDOWS_ESSENTIAL_ENV_VARS`
allowlist in `tools/code_execution_env.py` covers it; if you still hit it,
echo `os.environ` inside an `execute_code` block to confirm `SYSTEMROOT` is
set.

## Testing on Windows

`scripts/run_tests.sh` is POSIX-only (expects `.venv/bin/activate`); the
Hermes-installed `venv/Scripts/` has no pip/pytest (stripped for size).
Install pytest into a system Python and run directly (the canonical runner
does per-file subprocess isolation, which the POSIX-only wrapper handles):

```bash
"<python-install-dir>/python" -m pip install --user pytest pyyaml
export PYTHONPATH="$(pwd)"
"<python-install-dir>/python" -m pytest tests/foo/test_bar.py -v --tb=short
```

(POSIX-only tests need skip guards - see the cross-platform guard list in
`references/contributor-guide.md`.)

## Path / Filesystem

**Line endings.** Git may warn `LF will be replaced by CRLF`. Cosmetic - the
repo's `.gitattributes` normalizes. Don't let editors auto-convert committed
POSIX-newline files to CRLF.

**Forward slashes work almost everywhere.** `C:/...` style paths are accepted
by every Hermes tool and most Windows APIs. Prefer forward slashes in code
and logs - avoids shell-escaping backslashes in bash.

**Stop if** you are debugging one of these quirks inside a macOS Aphrodite
dev session - they do not apply there; the plugin targets macOS.

**Recovery** - permitted: reproduce the report on an actual Windows host or
in CI. Prohibited: "fixing" macOS behavior to mimic Windows quirks.

## Verification

| Claim                         | Test                                        | Pass condition                      |
| ----------------------------- | ------------------------------------------- | ----------------------------------- |
| BOM-free config               | `file config.yaml` on a Windows-saved file  | `UTF-8` (no BOM)                    |
| Env scrubber keeps essentials | `execute_code` printing `SYSTEMROOT` on Win | Var present                         |
| Quirk list is Windows-only    | Any entry applied on macOS                  | Not reproducible - skip, don't port |
