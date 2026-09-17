# Aphrodite Conversational Benchmark - Conversation Corpus

This directory defines the **conversation scripts** that the conversational
benchmark harness (`harness.py`) drives through the four compression
scenarios (baseline, full, hermes_proxy, proxy_api). The scripts are not
transcripts of real sessions; they
are synthetic, fully anonymized conversations engineered to exercise the
content types that trigger Aphrodite's compression paths.

## Corpus shape (what `harness.py` expects)

`conversations.py` exposes three dataclasses and one registry:

- `ToolCall(id, name, arguments)` - a tool call the assistant would make;
  `arguments` is a JSON string.
- `Turn(role, content, tool_calls=None, tool_call_id=None)` - one message.
  `role` is one of `system | user | assistant | tool`. Assistant turns that
  invoke tools carry `tool_calls`; `tool` turns carry `tool_call_id` tying
  them to the call they answer.
- `Conversation(name, description, system_prompt, turns)` - one script.
- `ALL_CONVERSATIONS` - the registry the harness iterates.

The harness consumes the corpus through these accessor calls
(`harness.py`):

```python
from conversations import Conversation, Turn, ALL_CONVERSATIONS
# run_benchmark / ConversationRunner.run:
conversation.name            # result dirs, manifest, --conversation filter
conversation.description     # run banner
conversation.system_prompt   # first system message (if truthy)
conversation.turns           # for i, turn in enumerate(conversation.turns)
turn.role / turn.content     # message routing + token estimation
turn.tool_calls              # each tc.id / tc.name / tc.arguments
turn.tool_call_id            # tool-result message id (defaults to call_XXXX)
```

Each `tool` message whose `content` is ≥ 4096 bytes is compressed to a
`<<<CCR:hash|type|size>>>` marker by the cache-proxy simulation in the
FULL / HERMES_PROXY scenarios; the token proxy offloads older
messages when the simulated context exceeds ~57.6K tokens. Big tool outputs
are therefore the primary compression drivers in these scripts.

## The five conversations

| # | name | intent | content types exercised | intended compression path |
|---|------|--------|------------------------|---------------------------|
| 1 | `coding_task` | Multi-file Rust refactoring: read sources, extract a shared error type, fix build errors, rebuild | code (large file reads), build output, compiler errors, write/patch calls | cache proxy: large file reads + build output → CCR markers |
| 2 | `exploration_task` | Explore an unfamiliar proxy codebase: find entry points, read core modules, explain architecture | search results (file listings), code (large reads), JSON-ish config | cache proxy: large file reads → CCR markers; long architecture summary |
| 3 | `debugging_task` | Diagnose a failing build: run build, read failing files, fix, rebuild, iterate | build output, compiler error blocks, code, patch/terminal | cache proxy: error + build output → CCR markers |
| 4 | `release_flow_task` | Development → Current release ritual: version parity, gates, plugin-first sync, protected-path restore, submodule float, tag on Current only, restore worktree | git status/log output, version-file reads, gate/build output (clippy, cargo deny, import check), sync/merge/tag/push output, submodule status | cache proxy: large clippy transcript + sync diffstat → CCR markers; token proxy: ritual-length history → offload |
| 5 | `compression_aware_task` | Deliberately minimize context: prefetch files into CCR, search with previews, compact catalog, retrieve only the entry a question needs | CCR marker/preview output, prefetch status, compact catalog, search hits, one targeted retrieve of a large file | cache proxy: the single retrieved file (6.1 KB) → CCR marker; exercises marker-in-marker behavior |

### 1. coding_task
Refactors a fictional three-module Rust crate (`/tmp/test_project`). The two
large `read_file` results (5.9 KB + 4.2 KB) and the failing `cargo build`
transcript are the compression targets. Ends with a clean rebuild.

### 2. exploration_task
Walks an unfamiliar proxy codebase (`/tmp/proxy_project`). A 47-file search
listing, three `read_file` results (the largest 7.0 KB), and a long
architectural synthesis answer. The main/`main.rs` + `proxy.rs` reads model
the "understand a new repo" workload.

### 3. debugging_task
A 7-error `cargo build` transcript (`/tmp/rust_project`), targeted file
reads, a patch, and a green rebuild. Error blocks and build output are the
content types under test; the iteration loop (run → read → fix → rebuild)
is the shape.

### 4. release_flow_task
Models the Development → Current release ritual for a two-branch project
with a plugin submodule (`/tmp/aphrodite-release/`): version-parity grep,
clean-tree check, gate set (clippy `-D warnings`, `cargo deny`, import
check), plugin-first squash sync with tag, parent squash sync with
protected-path restore (`.gitmodules`, `.github/workflows`, submodule
gitlink) and submodule float to the Current tip, tag on Current only,
final invariant verification, worktree restored to Development. Every
command is machine-checkable (`echo EXIT:$?`). The clippy transcript
(4.2 KB) and the sync diffstat (4.7 KB) are the cache-proxy compression
targets; the 17-turn ritual exercises the token proxy's history offload.

### 5. compression_aware_task
Models the behavior Aphrodite is designed to enable: an agent on a tight
context budget prefetches four pipeline files into CCR (markers + previews
only), searches with compact hits, confirms state via a compact catalog
and stats, then performs exactly one targeted `aphrodite_retrieve`
(classifier.rs, 6.1 KB - itself re-compressed by the cache proxy) to
answer the user's question. The assistant explicitly reports what it chose
NOT to expand, and closes with a summary of the token-saving decisions.

## Content-type coverage map

| content type | scripts exercising it |
|--------------|----------------------|
| code | 1, 2, 3, 5 |
| diff | 4 (squash diffstat), 5 (classifier's diff rules) |
| build_output | 1, 3, 4 (clippy transcript) |
| error | 1, 3 |
| terminal | 3, 4 (command + `EXIT:$?` echoes) |
| git | 4 (status/log/merge/tag/push/submodule output) |
| json | 2 (config), 5 (catalog/stats) |
| CCR marker / preview | 5 |
| search results | 2, 5 |
| version-file reads | 4 |

## Anonymization

The corpus is fully anonymized: no usernames, home paths, machine names,
or email addresses appear in any script. All project paths are `/tmp/...`
scratch locations, project/author identifiers are generic placeholders
(`mylang`, `myproject`, `aphrodite` as the project under test), and the
release script uses neutral branch/tag names (`Development`, `Current`,
`Aphrodite/vX.Y.Z`). A regex sweep for `nikola|CORSAIR|@domain|/Users/`
matches nothing in `conversations.py`.

## Running

```bash
# Validate corpus + harness wiring without running anything:
/opt/homebrew/bin/python3.13 bench/conversational/harness.py --dry-run

# Full run (requires DEEPSEEK_API_KEY and the aphrodite binary):
DEEPSEEK_API_KEY=... /opt/homebrew/bin/python3.13 bench/conversational/harness.py

# Single conversation / scenario:
/opt/homebrew/bin/python3.13 bench/conversational/harness.py --conversation release_flow_task --scenario full
```

Results land in `bench/conversational/results/<run_timestamp>/`; see
`harness.py` for the output schema and `visualize.py` for reporting.