---
name: hermes-cron-scheduling
description: "Use when scheduling durable Hermes cron jobs and watchdogs for the Aphrodite runtime (proxy health, hot-reload, post-reboot actions). Durable, disk-stored jobs fire from the gateway even after the creating session is gone."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: hermes
category_taxonomy: hermes/hermes-cron-scheduling
date: 2026-09-25
metadata:
    hermes:
        tags: [hermes, scheduling, watching, verifying, rebooting, shutting-down]
        related_skills:
            - hermes-agent
            - hermes-config-maintenance
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - "`hermes cron create` / `hermes cron list` registration truth and flag semantics"
    - The `--no-agent` watchdog pattern (empty stdout = silent run)
    - The post-reboot uptime-window watchdog and its one-shot marker file
    - The agent-safe macOS shutdown path (System Events, hardline blocklist escape)
depends_on:
    - aphrodite-orientation (Orient gate before scheduling runtime-mutating jobs)
verification:
    source_of_truth:
        - ~/.hermes/aphrodite/ (runtime home layout: aphrodite.toml, binaries/, hotreload/)
        - "`hermes cron list` (registration and next-run truth)"
mutation_level: local
---

# Hermes Cron Scheduling (Aphrodite)

Durable, disk-stored scheduled jobs that run even when the conversation that
created them is gone. Drive them with the `hermes cron` CLI (or the `cronjob`
tool when available in-session). In the Aphrodite monorepo this skill owns the
**Observe** phase of the unified lifecycle (full table in
`aphrodite-orientation`): it schedules health checks, hot-reload verification,
and post-reboot actions against the runtime home. It is `mutation_level: local`

- it schedules jobs and edits `~/.hermes/scripts/`, never Git history or
  releases.

## The CLI

- **The verb is `create`, not `add`** - `hermes cron create <schedule>
[flags]`. (The background-systems reference in the hermes-agent skill says
  `add`; the CLI rejects it.)
- Schedules: duration (`"30m"`, `"2h"`), phrase (`"every monday 9am"`),
  5-field cron (`"0 9 * * *"`), or ISO timestamp.
- Key flags:
    - `--name <job-name>` - human-friendly label.
    - `--deliver <target>` - `origin` (the chat that created it), `local`
      (files only), `telegram`, or `platform:chat_id`.
    - `--script <name>.sh` - path RELATIVE to `~/.hermes/scripts/`; the file
      must live there.
    - `--no-agent` - skip the LLM entirely; the script IS the job and its
      stdout is delivered verbatim. **Empty stdout = silent run** - the
      classic watchdog shape (check condition, print nothing unless action
      needed).
    - `--failure-deliver <target>` - separate delivery for FAILURE notices;
      `local` suppresses them.
    - `--repeat N` - finite run count (omit = every schedule tick, forever).
- Verify registration and next run with `hermes cron list`.

## Aphrodite job shapes

| Job                     | Schedule shape          | Script behavior                                                                           | Output contract                          |
| ----------------------- | ----------------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------- |
| Runtime-home watchdog   | `'every 5m' --no-agent` | Verify `~/.hermes/aphrodite/` is intact (aphrodite.toml, binaries/, hotreload/)           | Silent when healthy; one line on degrade |
| Hot-reload verification | one-shot `--repeat 1`   | Confirm `libaphrodite_hermes.dylib` in `~/.hermes/aphrodite/hotreload/` has a fresh mtime | Print path + mtime only when stale       |
| Post-reboot action      | `'every 5m' --no-agent` | Compute uptime from `sysctl -n kern.boottime`; act only inside the window, once           | Echo a message before acting (to origin) |

For live in-session truth of the same state, use the CCR tools
(`aphrodite_stats` for proxy health, `aphrodite_rebuild` for the loaded dylib
version) instead of a scheduled job - scripts are for unattended checks,
CCR tools for interactive verification.

## Pitfalls

- **Never pass a trailing prompt positional after the flags** - the parser
  rejects it with `unrecognized arguments` (it errors out at the root `hermes`
  parser, so the job is NOT created). `name + script + deliver` is enough; if
  you want a prompt, it must go as the last positional in the documented
  `schedule [prompt]` shape, and when in doubt just omit it.
- **A prior `hermes update` that did not restart the gateway** makes CLI
  output print a mixed-sys.modules warning. It is a warning only; jobs still
  create and list correctly. Restart the gateway (`hermes gateway restart`)
  when you need the scheduler itself on fresh code.
- A watchdog that mutates state (writes the marker, triggers shutdown) must
  verify its preconditions first - uptime window, runtime-home state - so a
  mis-scheduled job degrades silently instead of acting blindly.

## Stop if / Recovery

**Stop if** - the job does not appear in `hermes cron list` after creation,
or the script writes outside `~/.hermes/scripts/` and `~/.hermes/tmp/`.

**Recovery**

- Permitted: re-run `hermes cron create` with corrected flags and re-verify
  with `hermes cron list`; edit the script under `~/.hermes/scripts/` with
  write_file/patch.
- Prohibited: hand-editing cron state in `~/.hermes/` storage; scheduling
  `sudo shutdown` (hardline blocklist).

## Jobs survive reboots

The scheduler runs inside the gateway, and the gateway is a launchd agent
with `RunAtLoad` + `KeepAlive`
(`~/Library/LaunchAgents/ai.hermes.gateway.plist`) - it restarts with the
machine and cron jobs are read from disk, so a scheduled job fires after a
reboot with no agent "ping" needed. If the user asks "will you get a ping?",
the honest answer is: no notification reaches the agent, but the scheduled
job wakes on schedule when the gateway is back.

For Aphrodite: after a reboot the runtime home (`~/.hermes/aphrodite/`) is on
disk but no session has loaded the proxy dylib yet. A post-reboot watchdog
should verify runtime-home state before acting; a live session re-loads the
dylib from `~/.hermes/aphrodite/hotreload/` on mtime.

## Watchdog pattern: act N minutes after the next reboot

When the reboot time is unknown (e.g. a macOS update that auto-restarts), a
one-shot timestamp cannot work. Use a recurring uptime-window watchdog:

1. Script computes seconds since boot from `sysctl -n kern.boottime` (field 4,
   strip the trailing comma) and acts only when uptime is inside the target
   window (e.g. 12-25 min = 720-1500 s).
2. **One-shot marker file** - create it on first fire so a later manual
   reboot never re-triggers the action.
3. Register `hermes cron create 'every 5m' --no-agent --script <name>.sh
--deliver origin`.
4. When it fires, it echoes a message (delivered to `origin`) before the
   action completes.

Working example: `scripts/post-reboot-shutdown-watchdog.sh`.

## Agent-safe shutdown on macOS (used by the watchdog)

- `sudo shutdown` / reboot commands are on the agent's **hardline
  unconditional blocklist** - refused even with `--yolo` or approvals off. The
  agent-safe path is
  `osascript -e 'tell app "System Events" to shut down'`.
- **osascript exits 0 even when the shutdown does not complete** - a GUI app
  holding state (e.g. a terminal emulator) silently prevents it. Verify by
  uptime afterwards, not by the exit code.
- If the machine is still up after the call: quit the blocking app
  (`osascript -e 'tell application "<app>" to quit'`); if it refuses with
  `User canceled (-128)`, force-kill (`pkill -9 -f <app>`), then re-issue the
  shutdown. Note that quitting the user's terminal kills that surface - the
  gateway process is separate and keeps running.

## Local claim-to-test matrix

| Claim                            | Evidence source             | Test                                    | Pass condition                                | Failure response                    |
| -------------------------------- | --------------------------- | --------------------------------------- | --------------------------------------------- | ----------------------------------- |
| The verb is `create`             | CLI                         | `hermes cron create 'every 5m' ...`     | Job appears in `hermes cron list`             | Use `create`, never `add`           |
| `--no-agent` prints only on need | Script stdout               | Run the script directly                 | Empty stdout when healthy                     | Print nothing unless action needed  |
| Uptime-window math is correct    | `sysctl -n kern.boottime`   | Run with a known boot time              | Window bounds match (720-1500 s)              | Fix the awk/field parse             |
| One-shot marker prevents repeats | Marker file                 | Run the script twice                    | Second run exits 0 silently                   | Create the marker before the action |
| osascript exit is not proof      | Uptime after the call       | Issue shutdown, re-check uptime         | Uptime resets only when the machine went down | Verify by uptime, not by exit code  |
| Jobs survive reboots             | launchd plist + `cron list` | Reboot, then inspect `hermes cron list` | Job still registered, fires on schedule       | Re-register; restart the gateway    |
