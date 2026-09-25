# Durable & Background Systems

Use when an Aphrodite workflow needs work that outlives a single turn -
delegated children, scheduled jobs, skill curation, or a shared multi-worker
board. Four systems run alongside the main conversation loop. Quick reference
here; repo dev notes live in `.hermes/AGENTS.md` and the dev skills in
`.hermes/skills/`; user-facing docs under
https://hermes-agent.nousresearch.com/docs/user-guide/features/

## Delegation (`delegate_task`)

Spawn a subagent with an isolated context + terminal session.

- **Single:** `delegate_task(goal, context)`.
- **Batch:** `delegate_task(tasks=[{goal, ...}, ...])` runs children in
  parallel, capped by `delegation.max_concurrent_children` (default 3).
- **Background:** `delegate_task(background=true)` returns a handle
  immediately and keeps the parent loop going; the child's result
  re-enters the conversation as a new turn when it finishes.
- **Roles:** `leaf` (default; cannot re-delegate) vs `orchestrator`
  (can spawn its own workers, bounded by `delegation.max_spawn_depth`).
- **Not durable.** A backgrounded child is still process-local - if the
  parent process exits, the child is lost. For work that must outlive
  the process, use `cronjob` or
  `terminal(background=True, notify_on_complete=True)`.

Aphrodite parallel sweeps (parallel crate benches, parallel skill-library
audits, the `parallel-delegation-execution` pattern) run under the same caps.
Set the cap per profile: `hermes config set
delegation.max_concurrent_children N --profile dev-aphrodite`. Children
inherit the parent profile's `delegation.*` config.

Config: `delegation.*` in `config.yaml`.

## Cron (scheduled jobs)

Durable scheduler - `cron/jobs.py` + `cron/scheduler.py`. Drive it via
the `cronjob` tool, the `hermes cron` CLI (`list`, `add`, `edit`,
`pause`, `resume`, `run`, `remove`), or the `/cron` slash command.

- **Schedules:** duration (`"30m"`, `"2h"`), "every" phrase
  (`"every monday 9am"`), 5-field cron (`"0 9 * * *"`), or ISO timestamp.
- **Per-job knobs:** `skills`, `model`/`provider` override, `script`
  (pre-run data collection; `no_agent=True` makes the script the whole
  job), `context_from` (chain job A's output into job B), `workdir`
  (run in a specific dir with its `AGENTS.md` / `CLAUDE.md` loaded),
  multi-platform delivery.
- **Invariants:** 3-minute hard interrupt per run, `.tick.lock` file
  prevents duplicate ticks across processes, cron sessions pass
  `skip_memory=True` by default, and cron deliveries are framed with a
  header/footer instead of being mirrored into the target gateway
  session (keeps role alternation intact).

Flavor for the Aphrodite repo: a nightly engine-bench job can set
`workdir` to the repo root so its rules load - and must keep any probe
artifacts in `.hermes/tmp/`, never a system temp dir.

User docs: https://hermes-agent.nousresearch.com/docs/user-guide/features/cron

## Curator (skill lifecycle)

Background maintenance for agent-created skills. Tracks usage, marks
idle skills stale, archives stale ones, keeps a pre-run tar.gz backup
so nothing is lost.

- **CLI:** `hermes curator <verb>` - `status`, `usage`, `run`, `pause`,
  `resume`, `pin`, `unpin`, `archive`, `restore`, `list-archived`, `prune`,
  `backup`, `rollback`.
- **Slash:** `/curator <subcommand>` mirrors the CLI.
- **Scope:** only touches skills with `created_by: "agent"` provenance.
  Bundled + hub-installed skills are off-limits. **Never deletes** -
  max destructive action is archive. Pinned skills are exempt from
  every auto-transition and every LLM review pass.
- **Cost:** the deterministic inactivity/prune sweep runs for free. The
  aux-model "consolidate overlapping skills into umbrellas" pass is
  **off by default** - opt in with `curator.consolidate: true` or
  `hermes curator run --consolidate`. Routine background curation costs
  zero tokens.
- **Telemetry:** sidecar at `~/.hermes/skills/.usage.json` holds
  per-skill `use_count`, `view_count`, `patch_count`,
  `last_activity_at`, `state`, `pinned`.

Repo dev skills under `.hermes/skills/` are authored files (not
`created_by: "agent"`), so the curator's auto-transitions never touch them.

Config: `curator.*` (`enabled`, `interval_hours`, `min_idle_hours`,
`stale_after_days`, `archive_after_days`, `backup.*`).
User docs: https://hermes-agent.nousresearch.com/docs/user-guide/features/curator

## Kanban (multi-agent work queue)

Durable SQLite board for multi-profile / multi-worker collaboration.
Users drive it via `hermes kanban <verb>`; dispatcher-spawned workers
see a focused `kanban_*` toolset gated by `HERMES_KANBAN_TASK`, and
orchestrator profiles can opt into the broader `kanban` toolset. Normal
sessions still have zero `kanban_*` schema footprint unless configured.

- **CLI verbs (common):** `init`, `create`, `list` (alias `ls`),
  `show`, `assign`, `link`, `unlink`, `comment`, `complete`, `block`,
  `unblock`, `archive`, `tail`. Less common: `watch`, `stats`, `runs`,
  `log`, `dispatch`, `daemon`, `gc`.
- **Worker/orchestrator toolset:** `kanban_show`, `kanban_complete`,
  `kanban_block`, `kanban_heartbeat`, `kanban_comment`, `kanban_create`,
  `kanban_link`; profiles that explicitly enable the `kanban` toolset
  outside a dispatcher-spawned task also get `kanban_list` and
  `kanban_unblock` for board routing.
- **Dispatcher** runs inside the gateway by default
  (`kanban.dispatch_in_gateway: true`) - reclaims stale claims,
  promotes ready tasks, atomically claims, spawns assigned profiles.
  Auto-blocks a task after `failure_limit` consecutive spawn failures
  (default 2; configurable via `kanban.failure_limit` or per-task
  `max_retries`).
- **Isolation:** board is the hard boundary (workers get
  `HERMES_KANBAN_BOARD` pinned in env); tenant is a soft namespace
  within a board for workspace-path + memory-key isolation.

A multi-worker Aphrodite release could route crate tasks (proxy vs
bridge vs plugin) across profiles on a shared board, with the plugin
profiles pinned to the `kanban` toolset.

User docs: https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban

**Stop if** a backgrounded child vanishes after the parent exits - that is
expected; it was process-local.

**Recovery** - permitted: convert the job to `cronjob` or a background
`terminal` process. Prohibited: re-running a delegate batch "until it
sticks" without changing durability.

## Verification

| Claim                         | Test                                                   | Pass condition                           |
| ----------------------------- | ------------------------------------------------------ | ---------------------------------------- |
| Batch cap is configurable     | `hermes config get delegation.max_concurrent_children` | Matches the profile value                |
| Cron run is interrupt-safe    | Run a job with a 10s sleep                             | Killed at 3-minute hard interrupt        |
| Curator skips authored skills | `hermes curator status` in the Aphrodite repo          | No `.hermes/skills/` entry auto-archived |
