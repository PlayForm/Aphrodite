# Latency audit - slow Hermes agents/subagents

Audit procedure when agents or subagents take long even on simple tasks,
typically after a config change to "execution properties". Applies to the
default profile and to `dev-aphrodite` sessions alike (the dev profile only
overlays the default config).

## Procedure

1. Read `~/.hermes/config.yaml` fully (the active runtime config; profile
   configs at `~/.hermes/profiles/<name>/config.yaml` only overlay it; CLI-side
   settings live in `~/.hermes/cli-config.yaml`).
2. Diff suspect keys against canonical defaults: grep the key in
   `~/.hermes/hermes-agent/hermes_cli/config_defaults.py`. That file is the
   ground truth for "what changed" - config.yaml holds only non-stock values.
3. Trace consumption before asserting impact: `grep -rn '<key>'` in the core
   checkout and read the consuming function.
4. Measure what is measurable: hook latency (`time` each hook script with a
   representative stdin payload), shell startup (`time bash -ic true` vs
   `bash -c true`), and inspect hook matchers for overlap/double-fire.
5. Deep dive = read-only pair of subagents with disjoint ownership and an
   output_schema; both must return line-numbered evidence and may not edit
   anything or run git.

## Verified defaults map (config_defaults.py)

| Key                                | Stock default         | Notes                                                                                   |
| ---------------------------------- | --------------------- | --------------------------------------------------------------------------------------- |
| agent.reasoning_effort             | '' (provider default) | 'high' multiplies reasoning tokens on every tool-call round; worst on flash-tier models |
| agent.verify_on_stop               | False                 | true adds verification LLM turns at the end of every run; max_verify_nudges defaults 3  |
| agent.max_turns                    | None                  | main-loop turn ceiling                                                                  |
| goals.max_turns                    | 20                    | far below agent.max_turns; a high value here allows long goal runs                      |
| delegation.max_iterations          | 250                   | child loop bound                                                                        |
| delegation.child_timeout_seconds   | 0                     | 0 = NO timeout - a looping child runs to max_iterations                                 |
| delegation.max_concurrent_children | 10                    | pair-shaped waves typically run 4                                                       |
| delegation.model/provider/base_url | ''                    | empty = children inherit the parent model and its latency profile                       |
| skills.inline_shell                | False                 | see inline-shell mechanics below                                                        |
| skills.inline_shell_timeout        | 10                    | seconds per snippet, sequential                                                         |
| terminal.persistent_shell          | True                  | reused shell; fresh shells source bashrc when auto_source_bashrc is on                  |

## Values to weigh when auditing (not defaults)

tool_output max_bytes/max_lines set far above stock (~1M) flood context and
multiply compression events; compression threshold/target_ratio far below the
stock shape make compression fire constantly - and with auxiliary.free_only
true every compression event is an aux-LLM call on the slowest (free) tier.

## Inline-shell mechanics (agent/skill_preprocessing.py)

- Pattern: `!` followed by a backtick, single line inside, closing backtick;
  runs `bash -c <cmd>` with the skill dir as cwd on EVERY skill load
  (preprocess_skill_content), sequentially, per-snippet timeout, output capped
  at 4000 chars.
- False-positive trap: a naive `!` search hits Rust `//!` doc comments and JS
  template literals - audit with the real regex (bang + backtick), not a bare
  `grep '!`'`.
- With the feature on and zero real snippets, the only cost is a regex scan;
  the cost model is per-snippet timeout × snippet count, so one slow snippet
  adds up to 10s to every skill load in every session AND every subagent.

## Where each suspect is consumed (trace points)

- reasoning_effort → provider request assembly (agent/turn_*.py,
  plugins/model-providers/)
- verify_on_stop / max_verify_nudges → agent/turn_stop_gates.py
- child_timeout_seconds → tools/delegate_tool.py
- compression threshold/target_ratio → agent/turn_context_compaction.py /
  conversation_compression.py; aux model resolution → agent/auxiliary_client.py
  (_resolve_auto_route; free_only resolves to the free-tier pool)
- terminal backend → tools/environments/local*.py
- hook timeout application → hook invocation path in agent/ or hermes_cli

## Aphrodite plugin hook overhead

The CCR hooks (`transform_tool_result`, `transform_terminal_output`,
`pre_llm_call`, `post_llm_call`) run on every matching event in a compressed
session; `on_session_start` runs once per session. When a dev-aphrodite
session feels slow:

- Profile with `aphrodite_stats` (session + proxy health counters) and
  `aphrodite_diff` (per-turn history of what was compressed) before touching
  Hermes latency knobs.
- Large terminal output that crosses the compression threshold returns a
  `<<<CCR:hash|type|size>>>` marker; each `aphrodite_retrieve` is an extra
  round-trip.
- The plugin hot-reloads `libaphrodite_hermes.dylib` from
  `~/.hermes/aphrodite/hotreload/`; a stale dylib cannot cause latency, but a
  rebuild loop (`cargo watch -x 'build -p aphrodite -p aphrodite-hermes'`)
  can compete for CPU with the audited session.

## Hook double-fire

hooks.post_tool_call matchers are regex alternations; overlapping matchers fire
one tool call multiple times (a config that runs two hooks for execute_code and
two for write_file/patch doubles per-call overhead). Each hook is a full bash +
jq + perl spawn. On write-heavy subagent work, trim to one hook per event or
merge scripts into one.

## Claim-to-test

| Claim                                        | Test                                                       | Pass condition                                  |
| -------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------- |
| Defaults map matches the stock core          | Grep each row's key in config_defaults.py                  | Table value equals the code default             |
| Consumer trace points are accurate           | Grep the key, read the consuming function                  | Function listed is the one that consumes        |
| Hook double-fire is measurable               | `time` two overlapping hooks on one event                  | Both fire; runtime of the second is visible     |
| CCR hook overhead isolated from Hermes knobs | Run `aphrodite_stats` + `aphrodite_diff` on a slow session | Counters localize the cost before tuning config |
