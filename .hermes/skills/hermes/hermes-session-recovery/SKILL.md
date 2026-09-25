---
name: hermes-session-recovery
description: "Use when a Hermes session running the Aphrodite plugin was truncated, died mid-work, or you need to reconstruct what a past session did."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: hermes
category_taxonomy: hermes/hermes-session-recovery
date: 2026-09-25
metadata:
    hermes:
        tags: [hermes, recovering, resuming, reconstructing, exporting, verifying]
        related_skills:
            - hermes-agent
            - hermes-diagnostics
            - parallel-delegation-execution
status: active
---

# Hermes Session Recovery

When an Aphrodite session dies mid-work (truncation during CCR compression,
crash, /stop), the work is almost never lost. Recover from three survivors,
in this order.

## 1. Hermes session DB (the conversation itself)

Every message is persisted, so a "truncated" session is still fully readable.

- `session_search(query, sort='newest')` discovers the dead session; search by
  the work's TOPIC, not the symptom. `detail='adaptive'` hydrates the top hit
  with bookends - the anchor message is usually the session's final report.
- Scroll around the anchor (`session_id` + `around_message_id`) to read the
  last exchange and any tool-call tail.
- Pass `profile=<name>` + `session_id` to read another profile's session
  (e.g. the `dev-aphrodite` profile).
- When you refer the user to a recovered session, write its `link` value
  verbatim inline - it renders as a titled link.
- **Inspect a KNOWN session by ID** (`YYYYMMDD_HHMMSS_hex`, e.g.
  `20260919_065019_e6d636`): resolve it straight from `~/.hermes/state.db`
  (sessions row for metadata, messages replay for the narrative) - never
  `find` the filesystem for the ID. The `~/.hermes/sessions/` dir is only a
  legacy gateway mirror. Recipe: `references/transcript-export.md`.

## 2. Git working tree (uncommitted work)

A dead session's edits stay on disk. `git status --short` + `git diff --stat`
show exactly where it stopped; `git diff -- <path>` recovers the content.
In the Aphrodite monorepo, check BOTH worktrees: the parent
(`PlayForm/Aphrodite`) and the plugin submodule at `plugins/aphrodite`
(`git submodule status`). Pointers and uncommitted submodule edits survive
too.

## 3. Delegation transcripts (subagent findings)

Subagent runs outlive their parent session:

- `~/.hermes/cache/delegation/live/deleg_<id>/task-<n>.log` - each task's
  full run; the FINAL SUMMARY sits in the log tail (search the tail for
  `status=completed`). `manifest.json` holds dispatch metadata.
- A continuation note claiming "subagents still running" is STALE - check the
  transcript logs before trusting it; they may have completed after the parent
  died.

## Exporting a full session record

When the user wants the raw transcript + what was injected + what the plugin
did ("export its full transcript to scratch"): the TUI banner id is NOT the
store key, transcripts live in `state.db` (`messages`/`system_prompts` tables),
and turn-0 injections are not stored anywhere - full recipe with commands:
`references/transcript-export.md`.

## Always-on rules

- **Never trust the dead session's verification report.** Re-run the gates on
  the resumed tree before claiming anything is green: `cargo build -p aphrodite
-p aphrodite-hermes`, then live smoke via `aphrodite_rebuild`,
  `aphrodite_stats`, and `aphrodite_test`. A self-report from a truncated
  session is not evidence.
- **Resume in the dev profile.** Restart with `hermes --profile dev-aphrodite`
  so the plugin source binding and dylib hot-reload state
  (`~/.hermes/aphrodite/hotreload/`) match the recovered worktree.
- **Markers from a reset store never resolve.** If the CCR compression DB was
  cleared (runtime reset), old `<<<CCR:hash|type|size>>>` markers return
  `found: false` on every `aphrodite_retrieve` - don't retry them; recover
  from the three survivors above instead. Before concluding a marker is dead,
  retrieve it once: a live marker IS content, and `aphrodite_retrieve(hash)`
  is the only canonical route.
- **Write a continuation note as you go.** Heavy sessions end with a
  `CONTINUE-<date>.md` (or release summary) recording: repo state (HEAD,
  submodule pointers), uncommitted changes, open research, and next steps. It
  is the fastest resume path - read it FIRST when resuming.
- **Respect the standing gate**: never commit or release while recovering
  unless explicitly asked - verify state and report, then wait for the go.
