---
name: debugging-hermes-tui-commands
description: Use when debugging or adding Hermes TUI slash commands, including commands that surface Aphrodite plugin state.
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
category: hermes
category_taxonomy: hermes/debugging-hermes-tui-commands
date: 2026-09-25
metadata:
    hermes:
        tags: [hermes, debugging, adding, registering, autocompleting, dispatching]
        related_skills:
            - hermes-agent
            - hermes-diagnostics
status: active
---

# Debugging Hermes TUI Slash Commands

Hermes slash commands span three layers: the Python command registry, the `tui_gateway` JSON-RPC bridge, and the Ink/TypeScript frontend. When a command misbehaves (missing from autocomplete, works in CLI but not TUI, config persists but UI does not update), the bug is almost always one layer out of sync with another.

## When to Use

- A slash command exists in one part of the codebase but does not work fully
- A command must be added to both backend and frontend
- Command autocomplete is not working for specific commands
- Command behavior is inconsistent between CLI and TUI
- A command persists config but does not apply live in the TUI

## Prerequisites

- Hermes agent source at `~/.hermes/hermes-agent` (the authoritative tree; do not assume a different checkout)
- TUI build available: `npm --prefix ui-tui run build`

## Quick Reference

```
Python backend (hermes_cli/commands.py)     <- canonical COMMAND_REGISTRY
       |
       v
TUI gateway (tui_gateway/server.py)         <- slash.exec / command.dispatch
       |
       v
TUI frontend (ui-tui/src/app/slash/)        <- local handlers + fallthrough
```

The Python `COMMAND_REGISTRY` is the single source of truth. Everything derives from it: CLI dispatch, gateway `GATEWAY_KNOWN_COMMANDS`, gateway help, Telegram BotCommand menu, Slack subcommand map, and autocomplete data shipped to Ink. Adding an alias = one tuple change; every surface updates automatically.

## Procedure

### 1. Investigate

1. Check the TUI frontend: `search_files` for `/<commandname>` with `--file_glob "*.ts"` / `"*.tsx"` under `ui-tui/`; then `read_file ui-tui/src/app/slash/commands/core.ts` (or `search_files --target files` for the command dir)
2. Check the Python backend: `search_files --pattern "CommandDef" --file_glob "*.py" --path hermes_cli/`; then the command name in `hermes_cli/commands.py` with context
3. Examine the gateway: `search_files --pattern "complete.slash|slash.exec" --path tui_gateway/`

### 2. Fix missing autocomplete

1. Add a `CommandDef` entry to `COMMAND_REGISTRY` in `hermes_cli/commands.py`:

```python
CommandDef("commandname", "Description of the command", "Session",
           cli_only=True, aliases=("alias",),
           args_hint="[arg1|arg2|arg3]",
           subcommands=("arg1", "arg2", "arg3")),
```

2. Pick availability carefully: `cli_only=True` - interactive CLI/TUI only; `gateway_only=True` - messaging platforms only; neither - everywhere; `gateway_config_gate="display.foo"` - config-gated gateway availability (the gate is a config dotpath; `GATEWAY_KNOWN_COMMANDS` always includes gated commands so the gateway can dispatch them)
3. Ensure `subcommands` matches the tab-completion options the TUI shows
4. CLI handler: the dispatcher resolves `_handle_<name>_command` on the relevant `cli_*_mixin.py` by naming convention, or add an explicit `_SLASH_DISPATCH` entry in `cli.py` (`"mycommand": ("_handle_mycommand", True)`) when the method name or arg-passing differs. There is no elif ladder - do not add one
5. Gateway handler: add `_handle_<name>_command(self, event)` on the matching `gateway/slash_commands_*.py` mixin and list the command in `_IDLE_COMMANDS` (or `_PLAIN_COMMANDS` if it must work mid-run) in `gateway/run_busy.py`
6. Persistent settings via `save_config_value()` in `cli.py`

### 3. Common issues

1. **Shows in TUI but not in autocomplete** - defined in the TUI codebase but missing from `COMMAND_REGISTRY`; autocomplete data ships from Python
2. **Shows in autocomplete but does not work** - check the handler in `tui_gateway/server.py` and the frontend handler in `ui-tui/src/app/createSlashHandler.ts`. If the command is local-only in Ink, it must be handled in the `app.tsx` built-in branch; otherwise it falls through to `slash.exec` and needs a Python handler
3. **Behavior differs between CLI and TUI** - different implementations; check `cli.py` dispatch and the TUI's local handler. Local TUI handlers take precedence over gateway dispatch
4. **Persists config but does not apply live** - for TUI-local commands, updating `config.set` is not enough. Also patch the relevant nanostore state immediately (usually `patchUiState(...)`) and thread new state through rendering components. Example: `/details collapsed` must update live detail visibility, not just save `details_mode`
5. **Gateway silently ignores the command** - the gateway only dispatches commands it knows about. Confirm `GATEWAY_KNOWN_COMMANDS` (derived from `COMMAND_REGISTRY`) includes the canonical name; if the command is `cli_only` with a `gateway_config_gate`, verify the gated config value is truthy

### 4. Debugging tactics

- **Python side hangs or misbehaves**: break inside the `_SlashWorker.exec` or the command handler (remote debugger at handler entry is the fastest path)
- **Ink side not reacting**: use `node --inspect` with a debugger to break in `app.tsx`'s slash dispatch or the local command branch after `npm run build`
- **Registry mismatch / unclear which side is wrong**: compare the canonical `COMMAND_REGISTRY` entry against the TUI's local command list side by side

### 5. Aphrodite plugin angle

The Aphrodite plugin registers tools, not slash commands, so a misbehaving
command that touches plugin state usually fails in the plugin layer, not
the TUI. After any plugin change:

1. Rebuild both crates: `cargo watch -x 'build -p aphrodite -p
aphrodite-hermes'` (a single-package watch never rebuilds the dylib).
2. Verify the plugin imports: `python3 -c "import aphrodite"` (a missing
   symbol kills the plugin at session start).
3. Check engine health: `aphrodite_stats` / `aphrodite_rebuild` report the
   dylib version and proxy health; a stale process masks edits.
4. Restart the TUI session so the fresh dylib loads, then re-test the
   command.

Full dev-loop: `aphrodite-development`. Raw CCR markers in command output
are compression-safety territory: `aphrodite-compression-safety`.

## Pitfalls

- **Set the command category** in `CommandDef` (Session, Configuration, Tools & Skills, Info, Exit)
- **Register aliases only in the `aliases` tuple** - no other file changes are needed; everything downstream derives from it
- **Keep `subcommands` in sync with the TUI code** or tab completion lies
- **`cli_only=True` commands will not work in gateway/messaging platforms** unless a `gateway_config_gate` is set and truthy
- **After adding live UI state, search every consumer of the old prop/helper** and thread the new state through ALL render paths, not just the active streaming path - TUI detail rendering has at least two: live `StreamingAssistant`/`ToolTrail` and transcript/pending `MessageLine` rows
- **Rebuild the TUI before testing** - `tsx` watch mode may lag on first launch
- **Do not hand-edit `process_command()` with elif branches** - dispatch is table-driven (`_SLASH_DISPATCH` + naming convention); adding an elif ladder violates the codebase shape rules

## Verification

1. Rebuild: `cd ~/.hermes/hermes-agent && npm --prefix ui-tui run build`
2. Run the TUI: `hermes --tui`
3. Type `/` and verify the command appears in autocomplete with the expected description and args hint
4. Execute the command and confirm: expected behavior fires; persisted config updates correctly (`read_file ~/.hermes/config.yaml`); live UI state reflects the change immediately, not after restart
5. If gateway-available, test from at least one messaging platform (or run `scripts/run_tests.sh tests/gateway/`)
