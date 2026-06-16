---
name: aphrodite-session-patterns
description: "Release pipeline, budget tuning, centers, worker config, and pitfalls from development sessions."
version: 1.0.0
platforms: [macos]
---

# Aphrodite Session Patterns

Accumulated from development sessions. Covers the full dev cycle.

## Auto-Release Pipeline
```bash
GIT_EDITOR=true bash scripts/auto-release.sh "message"
```
Submodule sync → stage → commit → bump → build → test → tag → push.

## Budget Tuning
- Threshold: 72% (env: APHRODITE_ENGINE_THRESHOLD_PCT)
- Protect: 5 head / 7 tail
- MIN_MSGS: dynamic (max(12, msgs/10, 50))
- Budget curve: linear 0.50 + fill%×0.50
- System messages excluded from compression

## Centers
LLM memory deposits: `_ccr_center="code_rust"` travels with markers.

## Workers
poll_worker.py always includes `aphrodite` toolset.

## Pitfalls
- API rejects emoji in function names (^[a-zA-Z0-9_-]+$)
- Normalize em dashes (— → -)
- GIT_EDITOR=true for tag prompts
- Submodule: commit locally before --remote sync
