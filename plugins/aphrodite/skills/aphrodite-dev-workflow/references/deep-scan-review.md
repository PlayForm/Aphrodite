# Deep-Scan Review Response Pattern

## Context

The user periodically submits comprehensive multi-section code reviews covering
commits, files, and architectural concerns across the entire repo. These reviews
identify issues at multiple levels: profil config mismatches, missing safety guards
(trap, dry-run), placeholder values, and cross-cutting concerns.

## Response Workflow

### 1. Triage — Which Items Apply Here?

Not all items in a deep-scan apply to the current repo. Some reference files in
other codebases (Hermes agent, headroom fork, etc.). For each item:

```
File referenced in review → search_files in current repo → exists? yes/no
```

Flag "not in this repo" items explicitly. Don't search the universe.

### 2. Todo-Driven Execution

```python
todo(todos=[
    {"id": "1", "content": "Fix barebone profile: remove aphrodite-dev-workflow skill", "status": "pending"},
    {"id": "2", "content": "Fix compress-off profile: remove aphrodite-dev-workflow", "status": "pending"},
    ...
])
```

One item at a time, mark completed as you go. This gives the user visibility.

### 3. Batch Fix Pattern

All fixes in a review response should be done in a single pass:
- Read all affected files first
- Apply all patches
- Verify with dry-run where applicable
- Single commit with `fix(review):` prefix

### 4. Common Fix Categories

| Category | Fix Pattern |
|---|---|
| Profile config mismatch | `patch` the specific config.yaml, remove/add fields |
| Missing safety guard | Add `trap`, `--dry-run`, dirty check, comment |
| Placeholder values | Replace with sensible defaults + explanatory comment |
| Threshold wrong for mode | Set `enabled: false` + `threshold: 0.0` for cache-only |

## Session Example (2026-06-16)

Review covered 8 items across multiple repos. 6 applicable to HermesCompress, 2 in
other repos. All 6 fixed in one pass, single commit `2f1f925`.

Items not in this repo: `hermes_agent_eval.py` (Hermes agent), `hermes_mcp_client.py`
(Hermes agent).
