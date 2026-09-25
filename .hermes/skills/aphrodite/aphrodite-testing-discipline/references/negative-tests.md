# Negative tests (past failures -> permanent prevention)

Every known past failure becomes a permanent negative test or lint rule. The goal is not recording lessons; it is ensuring they cannot silently recur. A prevention must be an automated test or a lint rule - a prose warning is not a prevention.

| Past failure | Permanent prevention |
| --- | --- |
| `stdout` used instead of `output` | Hook signature/invocation compatibility test (sentinel-output test) |
| `sessionstart` registered instead of `onsessionstart` | Valid-hook registration test |
| Pre-LLM history edited in place | Test asserts original conversation remains unchanged; compression uses engine API |
| Retrieval result recompressed | End-to-end nested-marker test (no re-marking of resolved payloads) |
| Auto-expand setting assumed functional | Inert-setting test; documentation lint (env-var consumer status) |
| Old hook script recreated | Repository policy/lint rejects `.githooks` restoration (removed) |
| Self-referential gitlink returns | Recursive mode-160000 scan (submodule diagnosis, owner: aphrodite-orientation) |
| Tag triggers unexpected publish | Mandatory pre-tag workflow trigger audit (owner: aphrodite-release-flow) |
| Missing embedded template key | Template/live-config drift test (e.g. drift-guard diff) |
| Stale dylib masks source change | Fresh-process version/behavior test (Rule 2 handshake after rebuild) |