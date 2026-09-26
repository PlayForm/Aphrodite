---
name: hermes-agent
description: "Use when operating, configuring, theming, extending, or orchestrating Hermes Agent inside the Aphrodite monorepo. Hub skill: routing table to references, docs index, and hard invariants."
version: 3.4.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: hermes
category_taxonomy: hermes/hermes-agent
date: 2026-09-25
metadata:
    hermes:
        tags: [hermes, operating, configuring, theming, extending, orchestrating, spawning, routing]
        related_skills:
            - hermes-config-maintenance
            - hermes-cron-scheduling
            - hermes-diagnostics
            - hermes-shell-hooks
            - hermes-session-recovery
            - debugging-hermes-tui-commands
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
owns:
    - The routing table (which reference answers which task)
    - Hard invariants (prompt caching, role alternation, secrets placement, CCR marker discipline)
depends_on:
    - aphrodite-orientation
    - aphrodite-operations
verification:
    source_of_truth:
        - https://hermes-agent.nousresearch.com/docs/llms.txt
        - .hermes/AGENTS.md (repo facts)
mutation_level: local
---

# Hermes Agent (Aphrodite)

Hermes Agent is the host runtime the Aphrodite plugin runs inside. The CCR
compression proxy (`crates/aphrodite`) and the Hermes bridge
(`crates/aphrodite-hermes`, built as `libaphrodite_hermes.dylib` and
hot-reloaded from `~/.hermes/aphrodite/hotreload/`) plug into a Hermes session
through the plugin submodule at `plugins/aphrodite` (parent repo
`PlayForm/Aphrodite`, plugin repo `PlayForm/Aphrodite-Hermes`). Hermes runs in
the terminal, a native desktop app, messaging platforms, and IDEs, supports any
LLM provider, and is extensible via plugins, MCP servers, skins, TUI widgets,
and cron.

What makes Hermes different:

- **Self-improving through skills** - reusable procedures load into future
  sessions (the repo's dev skills live in `.hermes/skills/`, Development branch
  only, never shipped with the plugin).
- **Persistent memory across sessions** - remembers user identity,
  preferences, environment details, and lessons learned. Pluggable memory
  backends.
- **Multi-platform gateway** - the same agent runs on Telegram, Discord,
  Slack, WhatsApp, iMessage, Signal, Matrix, Teams, Email, and more, with full
  tool access, not just chat.
- **Many surfaces** - the same agent core drives the CLI, the Ink TUI, a
  native desktop app, a web dashboard, and an ACP server for IDEs.
- **Provider-agnostic + profiles** - swap models and providers mid-workflow;
  `hermes --profile dev-aphrodite` runs the repo's isolated dev instance with
  the Aphrodite plugin installed via the loader (see Key Paths).

The capability list above is CLAIM: probe with `hermes --help` or the docs index at `llms.txt` (see Scope & Verification).

**This skill is a hub.** The body covers identity, quick start, CCR-aware
operation, a spawning pointer, and hard invariants. Everything else lives in
reference files - **load the matching reference (below) before answering**; do
not answer detail questions from the body alone.

**Docs:** https://hermes-agent.nousresearch.com/docs/

## When to Use

Use this skill when the task involves Hermes Agent itself, as it hosts the
Aphrodite plugin:

- Operating it: install, setup, model/provider selection, health checks, surfaces.
- Configuring it: settings, providers, MCP, webhooks, cron, themes.
- Extending it: plugins, desktop UI plugins, TUI widgets, skins, pet mascots.
- Orchestrating it: spawning additional instances, session resume, multi-agent coordination.
- Answering "can Hermes do X?" capability questions.

Pick the matching reference from the Routing Table; for anything not listed,
fetch `llms.txt` (see Scope & Verification). For hooks, background workers, or
config-file surgery, load the dedicated skills: `hermes-shell-hooks`,
`hermes-background-workers`, `hermes-config-maintenance`.

## Scope & Verification

This skill is a concise operating guide, not the complete source of truth for
every Hermes feature. If a Hermes feature, command, or setting is not mentioned
here or in a reference, do not treat that absence as evidence that it does not
exist. Check the live repository and official docs before giving a negative
answer.

Good verification targets, cheapest first:

- **Every shipped feature, one line each: https://hermes-agent.nousresearch.com/docs/llms.txt.** Start here for any "can Hermes do X?" or "how do I do X?" - it indexes the entire documentation set with a link to the page that answers. Fetch it with `web_extract`, or `curl -s https://hermes-agent.nousresearch.com/docs/llms.txt` when web tools are off.
- CLI commands: `hermes --help`, `hermes <command> --help`.
- Source tree: https://github.com/NousResearch/hermes-agent

Never answer "Hermes can't do that" from memory. Hermes ships far more than
this skill body describes, and the `llms.txt` index makes a negative answer
checkable - fetch it with `web_extract`, or
`curl -s https://hermes-agent.nousresearch.com/docs/llms.txt` when web tools
are off.

## Quick Start

```bash
# Dev profile - plugin loader installed by 'aphrodite setup' (hooks-only layout)
hermes --profile dev-aphrodite

# Install/repair the plugin loader + runtime home (writes loader into
# ~/.hermes/plugins/aphrodite, BINARY_VERSION into the runtime home, enables)
aphrodite setup

# Single query
hermes chat -q "Run aphrodite_test and report the compress/retrieve round trip"

# Dev loop - watch BOTH crates; `-p aphrodite` alone never rebuilds the dylib
cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'

# Setup wizard  /  pick model+provider  /  health check
hermes setup
hermes model
hermes doctor
```

## Key Paths

```
~/.hermes/config.yaml - Main configuration (settings - never secrets)
~/.hermes/.env - API keys and secrets ONLY (under $HERMES_HOME if set)
~/.hermes/plugins/aphrodite/ - Plugin dir: ONLY plugin.yaml + __init__.py loader (5 hooks + 13 CCR tools)
~/.hermes/aphrodite/ - Runtime home: binaries/, aphrodite.toml, BINARY_VERSION, ccr.db, directives/, logs/, hotreload/
~/.hermes/profiles/dev-aphrodite/ - Dev profile (hermes --profile dev-aphrodite)
$HERMES_HOME/skills/ - Installed skills
~/.hermes/skins/ - Custom themes (see references/themes.md)
~/.hermes/desktop-plugins/ - Desktop app UI plugins (see references/desktop-plugins.md)
~/.hermes/tui-widgets/ - TUI widget apps (see references/tui-widgets.md)
~/.hermes/pets/ - Installed pet mascots (see references/petdex.md)
~/.hermes/state.db - Canonical session store (SQLite + FTS5)
~/.hermes/sessions/ - Gateway routing index, request dumps, *.jsonl transcripts
~/.hermes/logs/ - Gateway and error logs
~/.hermes/auth.json - OAuth tokens and credential pools
```

Profiles use `~/.hermes/profiles/<name>/` with the same layout. When a profile
is active, resolve the real home from `$HERMES_HOME` - never hardcode
`~/.hermes`.

## Routing Table - load the reference for the task

| User wants...                                                             | Load                                                    |
| ------------------------------------------------------------------------- | ------------------------------------------------------- |
| **Anything not listed below - "can Hermes do X?", "how do I set up X?"**  | **https://hermes-agent.nousresearch.com/docs/llms.txt** |
| Bots that chat, run routines, or message each other; the Bots tab         | docs: `/user-guide/bot-mode`                            |
| CLI commands, subcommands, flags, "how do I run X"                        | `references/cli-reference.md`                           |
| In-session slash commands                                                 | `references/slash-commands.md`                          |
| Provider setup, API keys, OAuth                                           | `references/providers-and-models.md`                    |
| config.yaml sections, toolsets, voice/STT/TTS                             | `references/configuration.md`                           |
| AGENTS.md / .hermes.md / CLAUDE.md project rules                          | `references/project-context-files.md`                   |
| Secret redaction, PII, approval modes, "reset permissions"                | `references/security-privacy.md`                        |
| Delegation, cron, curator, kanban                                         | `references/background-systems.md`                      |
| MCP servers (add, catalog, `hermes mcp`)                                  | `references/native-mcp.md`                              |
| Webhook routes and event-driven runs                                      | `references/webhooks.md`                                |
| A custom theme/skin ("synthwave theme", "change the gold ●")              | `references/themes.md` + `templates/skin.yaml`          |
| A desktop app UI element (pane, widget, ⌘K command, page)                 | `references/desktop-plugins.md` + `templates/plugin.js` |
| A live TUI panel or modal widget (ticker, clock, dashboard)               | `references/tui-widgets.md` + `templates/clock.mjs`     |
| Pet mascots - install, select, scale, diagnose                            | `references/petdex.md`                                  |
| Windows-specific issues (keybinds, WinError 10106, BOM)                   | `references/windows-quirks.md`                          |
| Debugging: voice, tools missing, gateway, aux models                      | `references/troubleshooting.md`                         |
| Contributing code: adding tools, slash commands, tests                    | `references/contributor-guide.md`                       |
| delegate_task "capped at N" reports                                       | `references/delegate-task-concurrency-diagnosis.md`     |
| "Can app X use my Nous Portal subscription/OAuth?"                        | `references/portal-auth-for-third-party-apps.md`        |
| Spawning extra Hermes instances (tmux/PTY, one-shot, resume, multi-agent) | `references/spawning-instances.md`                      |
| Surface orientation (desktop, dashboard, TUI, proxy)                      | `references/surfaces.md`                                |
| Connecting a messaging platform (Telegram, Discord, Slack, WhatsApp, …)   | docs: `/user-guide/messaging`                           |

The reference list above is not the feature list - it is the set of topics that
need more than their docs page. For everything else Hermes ships, fetch
`llms.txt` and it maps the question to the page that answers it.

Two theming rules that hold even without loading the reference: **you apply
skins yourself** (`hermes config set display.skin <name>` - every surface
repaints live within ~a second; don't tell the user to run `/skin`), and **to
tweak one color, edit the ACTIVE skin** (`hermes skin set <key> <hex>`) - never
fork `default`, which drops the palette and resets the background.

## CCR-Aware Operation (compressed sessions)

The Aphrodite proxy compresses large tool output, so a read may come back as a
`<<<CCR:hash|type|size>>>` marker. That marker **is** the content:

- Call `aphrodite_retrieve(hash)` to expand it before acting; never re-read the
  source file behind a marker you hold.
- If `aphrodite_retrieve` fails for an unknown hash, fall back to `read_file`
  for that specific item.
- CCR session tools: `aphrodite_retrieve`, `aphrodite_compress`,
  `aphrodite_stats`, `aphrodite_rebuild`, `aphrodite_search`,
  `aphrodite_catalog`, `aphrodite_diff`, `aphrodite_directive`,
  `aphrodite_files`, `aphrodite_prefetch`, `aphrodite_prefetch_status`,
  `aphrodite_reclassify`, `aphrodite_test`.
- Compression hooks: `on_session_start`, `transform_tool_result`,
  `pre_llm_call`, `transform_terminal_output`, `post_llm_call`.

**Stop if** a marker is treated as opaque text, or a file is re-read while a
live marker for it exists - resolve it with `aphrodite_retrieve(hash)`.

**Recovery** - retrieve the marker (`aphrodite_retrieve(hash)`), then proceed
with the expanded content.

## Spawning Additional Hermes Instances

Run additional Hermes processes as fully independent subprocesses - separate
sessions, tools, and environments. The worked sequences live in
`references/spawning-instances.md`: the `delegate_task` comparison, one-shot
mode, tmux interactive sessions, multi-agent coordination, and session resume.
Load that reference before spawning.

## Surfaces (quick orientation)

Surface descriptions and per-surface commands live in `references/surfaces.md` - load it before describing a surface.

## Hard Invariants (never violate, regardless of what you loaded)

- **Never break prompt caching** - don't change past context, toolsets, or the system prompt mid-conversation. The only exception is context compression.
- **Message role alternation** - never two assistant or two user messages in a row; only `tool` results can repeat.
- **Secrets in `.env`, settings in `config.yaml`** - never tell a user to put a non-credential setting in `.env`.
- **Profile-safe paths** - `get_hermes_home()` in code, `$HERMES_HOME` when resolving paths in a session.
- **`config.yaml` is the settings file; `hermes config set KEY VAL` is the only supported way to alter it** - a stray indent can corrupt the file and break the live gateway.
- **CCR markers are content** - retrieve before acting; never re-read a file you hold a live marker for.
- **Repo dev skills live in `.hermes/skills/`** - edit them directly with `write_file`/`patch`, never via `skill_manage`.
- **Scratch belongs in `.hermes/tmp/`** (gitignored, tracked `.gitkeep`) - never `/tmp`.
- **Repo skill content is scan-gated** - skills are content-hash-scanned on load; a flagged template (eval/exec patterns, encoded payloads) quarantines the skill until a re-scan passes. Keep skill templates minimal, innocuous plain content.

## Local claim-to-test matrix

| Claim                                | Evidence source      | Test                               | Pass condition                            | Failure response                     |
| ------------------------------------ | -------------------- | ---------------------------------- | ----------------------------------------- | ------------------------------------ |
| Routing table maps task to reference | This skill           | Ask a question for each table row  | Correct reference loaded                  | Fix the row mapping                  |
| Dev profile loads the repo plugin    | `readlink`           | `hermes --profile dev-aphrodite`   | Plugin symlink resolves to repo source    | Re-symlink; re-verify                |
| Watch rebuilds both crates           | `cargo watch` output | Save in `crates/aphrodite-hermes/` | dylib mtime/version changes               | Re-add `-p aphrodite-hermes`         |
| Markers resolve through retrieve     | `aphrodite_test`     | Round-trip compress + retrieve     | Expanded bytes match source payload       | Report proxy failure; fall back      |
| Scratch stays hermetic               | `git status --short` | Write a probe to `.hermes/tmp/`    | No probe artifacts outside `.hermes/tmp/` | Move artifacts; never reset to erase |
