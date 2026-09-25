# Memory pending-approval queue (`/memory pending`)

## Where records live

`$HERMES_HOME/pending/{memory,skills}/<8-hex-uuid>.json`, written by
`tools.write_approval.stage_write` via `atomic_json_write`. Skills queue is
usually empty; the memory queue accumulates because background reviews keep
proposing consolidations and every one is staged. One profile-scoped dir -
check `~/.hermes/pending` and `~/.hermes/profiles/*/pending`.

Record schema: `id`, `subsystem`, `action`, `summary`, `origin`
(`foreground` | `background_review` | `assistant_tool`), `created_at`,
`payload` (`action`/`target`/`content`/`old_text`/`operations`). The payload
is the EXACT kwargs to replay the write on approval.

## Staging paths (why records appear)

- `_background_delete_gate` (memory_tool.py): runs BEFORE the approval gate,
  fail-closed - a background-review `replace`/`remove`, single or inside a
  batch, is always staged, never applied, never denied. `write_approval`
  config is irrelevant to it. A batch containing any replace/remove stages the
  WHOLE batch - its `add` ops ride along and are lost when the record is
  dropped.
- `_apply_write_gate`: stages when `memory.write_approval: true` and no
  inline approval channel exists.

## Resolution APIs (what `/memory pending` does)

```python
import sys, os
sys.path.insert(0, os.path.expanduser("~/.hermes/hermes-agent"))
os.environ.setdefault("HERMES_HOME", os.path.expanduser("~/.hermes"))
from tools.write_approval import list_pending, discard_pending
from tools.memory_tool import apply_memory_pending, load_on_disk_store
```

- `list_pending(subsystem)` - records oldest-first.
- `discard_pending(subsystem, id)` - delete the record (the "discard" path).
- `apply_memory_pending(payload, store)` - replay the payload against the
  store, bypassing the gate (the "approve" path). Use `load_on_disk_store()`
  for a fresh store with configured caps.
- Verify after dropping: `list_pending` returns `[]` and the dir glob is
  empty.

## Matching semantics to simulate in a dry run

From `memory_tool_store.MemoryStore._find_unique_match`: a whole-entry EXACT
match (`old_text == entry`) takes absolute priority; substring matches are
considered only when no entry equals `old_text` (so a short entry stays
addressable inside a longer sibling). Multiple DISTINCT matches → ambiguous →
op fails. Batches are all-or-nothing: any malformed/unmatched op or over-limit
final state writes NOTHING. Budget is checked against the FINAL state only;
a batch that would empty a previously non-empty store is refused.

## Decision table (classify every record before approving)

| Class     | Meaning                         | Action                                                                                                                         |
| --------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| ALL-STALE | no op matches any current entry | drop - target entries already rewritten                                                                                        |
| PARTIAL   | some ops match, some don't      | drop unless the matching ops add genuinely NEW facts; matching replaces usually revert newer content to the older staged draft |
| CLEAN     | every op would apply            | still check: is the current store already newer/better? near cap? then drop                                                    |

Two-week-old backlogs are overwhelmingly superseded: foreground sessions
rewrote the same entries in newer form while the reviews' proposals sat
staged. Memory stores at 97-99% of their char limit reject most adds anyway.
Dropping leaves the current MEMORY.md/USER.md - which already contain the
newer versions - untouched; approving churns near the cap and reverts.

## In an Aphrodite-enabled session

The queue is independent of the CCR engine - CCR only shrinks tool output
into `<<<CCR:...>>>` markers and never stages or drops memory records. When
records are listed through session tools in a compressed session, resolve
markers with `aphrodite_retrieve(hash)` before classifying (see
`scripts/classify_pending_memory.py` in this skill).
