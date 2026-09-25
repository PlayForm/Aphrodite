# Session Transcript Export (state.db)

Recipe for dumping a session's FULL record to scratch when the user wants to
see the actual conversation, what was injected into context, and what the
Aphrodite plugin did - without the TUI renderer.

## ID mapping (the trap)

The TUI banner's `Session: cdac3b69` is the TUI-side `ui_session` id, NOT the
store key. The agent-side id comes from `agent.log`:

```
tui prompt accepted: ui_session=cdac3b69 session_key=20260917_021559_004f56 agent_session_id=20260917_021559_004f56
```

`agent_session_id` is the key used everywhere in state.db and in log line
prefixes (`[20260917_021559_004f56]`). Grep `agent.log` for the ui_session to
discover it.

## Store layout (~/.hermes/state.db, SQLite)

- `sessions` - metadata only: `started_at` (UNIX epoch), `message_count`,
  `tool_call_count`, token counts, `system_prompt_hash`, `title`, `git_branch`.
  NO message payloads here.
- `messages` - the transcript: `session_id`, `role` (user/assistant/tool),
  `content`, `tool_calls` (JSON), `timestamp`, `display_order`. Order by
  `display_order`.
- `system_prompts` - hash-keyed; the assembled system prompt at session start
  (the banner's "System Prompt - N chars" is this, via
  `sessions.system_prompt_hash`). Column is `prompt`, not `content`.

## Commands

```bash
# metadata + system prompt
sqlite3 ~/.hermes/state.db "SELECT * FROM sessions WHERE id='<agent_session_id>';"
sqlite3 ~/.hermes/state.db "SELECT prompt FROM system_prompts WHERE hash='<hash>';"

# transcript
sqlite3 ~/.hermes/state.db "SELECT '=== '||role||' ==='||char(10)||COALESCE(content,'')||char(10)||CASE WHEN tool_calls IS NOT NULL THEN 'TOOL_CALLS: '||tool_calls ELSE '' END||char(10)||'---' FROM messages WHERE session_id='<agent_session_id>' ORDER BY display_order;"
```

## Inspecting what a session attempted (by ID)

When the user hands you a full session ID (`YYYYMMDD_HHMMSS_hex`) and asks
what it did, resolve it straight from state.db - do NOT `find` the
filesystem for the ID (broad filesystem finds time out, and the ID appears
nowhere as a filename).

- `~/.hermes/sessions/sessions.json` is a LEGACY gateway-routing mirror (its
  `_README` says the primary lives in state.db's `gateway_routing` table) -
  it holds none of the recent sessions. `~/.hermes/cron/output/<job-id>/`
  files use a different naming scheme (`2026-09-19_05-47-52.md`) - a cron
  run is not a session.
- Metadata: `SELECT id, source, title, model, started_at, ended_at,
end_reason, message_count, tool_call_count, api_call_count, cwd FROM
sessions WHERE id='<sid>';` - `started_at`/`ended_at` are UNIX epochs,
  convert with `date -r <epoch>`.
- Narrative replay (the "what did it attempt" view): project
  `role, tool_name, substr(content,1,200), timestamp ... ORDER BY id` -
  role + tool_name + content-preview reads like an activity log. Tool
  results in `content` are often `<<<CCR:hash|type|size>>>` markers; resolve
  them via `aphrodite_retrieve(hash)` to see what a tool actually returned.
  Chunk long sessions with LIMIT/OFFSET.
- `end_reason` tells you HOW it ended (`tui_shutdown`, etc.) - the "why it
  stopped" answer.
- Cross-check its claims against the working tree afterwards
  (`git status --short`, `ls` of claimed artifacts): a session's own report
  is not evidence of what landed.

## What is NOT stored (inject the gap yourself)

- **Reasoning/thinking blocks** are not persisted - the visible flow is all
  you get.
- **Turn-0 injections** (`session_inject` / the `[APHRODITE]` orientation
  block injected by `pre_llm_call` on turn 0) are NOT in the stored system
  prompt or the transcript. Extract the code default (`SHIPPED_SESSION_INJECT`
  in `flow.rs`) or the config value and append it to the export, or the
  "what got injected" picture is incomplete.
- A session that ran WITHOUT a config file uses code defaults - the stats it
  reports (thresholds) reveal which defaults were in effect; note this in the
  export header.

## Side evidence (user-side + plugin activity)

- Session-scoped log: `grep -E "<agent_session_id>|ui_session=<short>" ~/.hermes/logs/agent.log`
    - every turn, API call (in/out/cache tokens), tool completion, error.
- User-side shell actions: zsh history timestamps are `: <epoch>:<dur>;cmd` -
  `awk -F: '$2 >= <start> && $2 <= <end>' ~/.zsh_history` for the window
  (start = session `started_at`).
- Export layout to scratch (`.hermes/tmp/`, never the repo tree):
  transcript.txt, system-prompt.txt, turn-0-inject.txt, session-metadata.txt,
  agent-log-session.txt, user-side-shell.log.
