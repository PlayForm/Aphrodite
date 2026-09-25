# TUI path completion (`complete.path`) - resolution chain & verification

## `_completion_cwd()` fallback chain (`tui_gateway/session_workdir.py`)

Root for relative (`./`) completions, in priority order - the FIRST hit wins:

1. `params["cwd"]` - the TUI does NOT send it (only `{word}`), so normally skipped
2. `_sessions[session_id]["cwd"]` - skipped too: `complete.path` carries no `session_id`
3. `_profile_configured_cwd(profile)` - only for a named secondary profile
4. `_launch_configured_cwd()` - `terminal.cwd` from config; `.`/`auto`/`cwd` are
   PLACEHOLDERS (`_CWD_PLACEHOLDERS`) and are skipped
5. `TERMINAL_CWD` env - bridged into the PTY child only; the TUI's own gateway child
   and the dashboard in-memory gateway do NOT get it
6. `os.getcwd()` - the gateway child, spawned with `cwd = HERMES_CWD` (the directory
   `hermes` was launched from) - the usual winner

`os.path.abspath(os.path.expanduser(raw))` is applied to the result and it must be an
existing dir, else the whole chain falls back to `os.getcwd()`.

The session row cwd (status bar, agent tools) is a DIFFERENT value:
`_sessions[sid]["cwd"]`, persisted in `~/.hermes/state.db`
`sessions(id, source, started_at, cwd)`. The status bar reads it; completion never does.

## Gateway-side flow (`tui_gateway/methods_complete.py`)

- `_normalize_completion_path()` applies `os.path.expanduser()` to the word - `~/`
  becomes home unconditionally, regardless of the completion root.
- `_dir_listing_items(root, word, ...)`: `search_dir` = dirname of the expanded word
  (or `.`), joined to `root` when relative; lists max 30 prefix-matching entries,
  sorted; `@folder:`/`@file:` filter by kind.
- Display style follows the user's input: `~/...` results render relative to home,
  `./...` relative to root, absolute words stay absolute.

## Forensic confirmation: prove the launch directory

When the user says "I launched from <project>" yet `./` completes home files,
the session record may disagree. Four independent sources must agree before
you call the launch dir the cause - check all of them:

1. `sqlite3 ~/.hermes/state.db "SELECT id, source, cwd, started_at, datetime(started_at,'unixepoch','localtime') FROM sessions WHERE started_at > <window> ORDER BY started_at"` - the session's `cwd` column
2. `HERMES_CWD` in the session env (TUI child env)
3. `TERMINAL_CWD` in the session env
4. the gateway child's `os.getcwd()` (the terminal tool's PWD inside the session)

All four pointing at one dir = the process really launched there; the "session
is in X" belief is about the shell tab, not the process. Map WHICH launch
created the session by grepping the shell history for `hermes` around the
session's `started_at` (`grep -a hermes ~/.zsh_history`). zsh history does
not record cwd, but a window with several launches whose session rows carry
different cwds (e.g. `tui` sessions from `~` next to `cli` sessions from the
project) disambiguates: match the `source` column to the command flags
(`--tui`/`HERMES_TUI=1` → `tui`; plain `hermes --yolo` under
`display.interface: cli` → `cli`).

Caveat before blaming the launch dir: `hermes_cli/main_tui_launch.py`
`_apply_tui_python_env` keeps `HERMES_CWD` when it is already set in the
environment and names an existing dir - only then does it fall back to
`os.getcwd()`. A stray `export HERMES_CWD=...` in the private environment
file, `.zshrc`, or `.bashrc` pins EVERY launch to that dir regardless of
shell cwd; grep the rc files for it. Also `which -a hermes` + read the shim
to confirm which install/launcher execs (a `~/.local/bin/hermes` shim may
point at a git checkout's `.hermes/bin/hermes`, which execs the managed
python - the launch dir logic is the same for all of them).

## Fixes, proven in the field

- **Guaranteed override regardless of launch dir:** set `terminal.cwd` to an absolute path
  (`hermes config set terminal.cwd /abs/path`) - chain position #4 (`_launch_configured_cwd()`)
  beats `TERMINAL_CWD` and the gateway child's `os.getcwd()`. Global (applies to every session
  of the profile); `hermes config unset terminal.cwd` (or set back to `.`) restores launch-dir
  behavior. Placeholder values (`.`, `auto`, `cwd`) never become the root. Aphrodite dev loop:
  `hermes config set terminal.cwd <workspace>/PlayForm/Aphrodite --profile dev-aphrodite`
  pins every dev-profile session to the repo root regardless of where the TUI launched.
- **Stale `TERMINAL_CWD` exported in the launching shell** wins over the launch dir at position
  #5 - the launcher's `build_subprocess_env` inherits it into the gateway child. If completion
  ignores a `cd`'d launch dir, check `echo $TERMINAL_CWD` in the shell.
- `./` results render relative to the root; `~/...` relative to home - a user seeing "home
  files" after typing `~/` is correct behavior, not a bug. The first complaint filed from a
  `hermes --profile dev-aphrodite` session turned out to be this.

## Reproduce against real code (throwaway gateway)

`scripts/probe_complete_path.py` spawns `python -m tui_gateway.entry` with the same
env the TUI uses (`PYTHONPATH` = hermes source root, `HERMES_CWD` = launch dir, child
cwd = launch dir) and asks `complete.path` for several words. Deterministic and
independent of the live gateway - the same probe run from the PlayForm/Aphrodite
repo root returns repo files, while `~`/`~/x` return home entries. Sessions under
`hermes --profile dev-aphrodite` follow the same launch-dir rule; the profile
changes `HERMES_HOME`, not the completion root.

Notes:

- The gateway child's stdio is BINARY pipes - encode the JSON-RPC frame
  (`frame.encode()`), or `stdin.write` raises `TypeError`.
- Boot takes ~4s; match responses by request `id` (the child also emits event frames).
- If the probe prints nothing, allow more boot time and confirm no leftover
  `tui_gateway.entry` from a previous probe run.
