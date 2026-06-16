# Re-Compression Guard

## Problem

When `aphrodite_retrieve` returns content from CCR, that content passes through
`_transform_tool_result` and `_transform_terminal_hook`. If the content is larger than
the compression threshold, it gets re-CCR'd — the LLM sees `[CCR:...]` markers
instead of the actual content it just retrieved.

Same issue with terminal output that already contains CCR markers (e.g., curl to
the retrieve endpoint directly).

## Fix

In both `_transform_tool_result` and `_transform_terminal_hook`, after the size
threshold check, add:

```python
# Don't re-compress content that already has CCR markers (retrieved/compressed)
if _CCR_RE.search(result):
    return result
```

Also add `"aphrodite_compress"` to the `skip` set in `_transform_tool_result`.

## Read-Intent Detection

In `_pre_llm_hook`, detect when the user's last message suggests they want to
read/view/inspect something. When detected, surface explicit `aphrodite_retrieve(hash)`
hints for the 3 most recent CCR markers.

```python
READ_KEYWORDS = {"read", "show", "view", "get", "cat", "display",
                 "retrieve", "fetch", "look", "see", "open",
                 "inspect", "check", "print", "dump", "output"}
# ... scan last user message for keywords ...
if has_read_intent and markers:
    recent_markers = markers[-3:]
    parts.append("  intent=read | recent CCRs available: " +
                 " ".join(f"aphrodite_retrieve({m['hash'][:8]})"
                          for m in recent_markers))
```

## Editing Tool Discipline

**Never** use terminal `sed`/`awk` or `execute_code` for file modifications.
Always use `patch` or `write_file`. Sed commands against YAML files silently
corrupt indentation — a single bad sed call cascades into cascading key damage
that requires manual recovery.

## headroom: is Hermes Built-In

The `headroom:` YAML key in `config.yaml` is a Hermes built-in config section
for context window headroom management. It is NOT part of the aphrodite plugin.
Do NOT rename `headroom:` to `aphrodite:` in config.yaml.

Only rename plugin-internal identifiers:
- Tool names: `headroom_*` → `aphrodite_*`
- Toolset entry: `headroom` → `aphrodite`
- The built-in `headroom:` config section stays.
