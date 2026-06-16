# Architectural Subtask Plan
See `.hermes/plans/architectural-subtasks.md` in the HermesCompress repo for the full plan.

## Summary

50 subtasks across 7 execution groups:

1. **Phase 5-D**: Single-mode simplification — collapse cache+token into token-only proxy
2. **Phase 5-E**: Streaming compression — progressive chunk forwarding, inline CCR
3. **Phase 3-A**: In-process file index — symbol extraction, project structure
4. **Phase 5-G**: Prometheus metrics — /metrics endpoint (done in v0.5.22)
5. **Phase 4-H**: Git diff + pre-cache hooks — git summary in catalog (done in v0.5.21)
6. **Phase 5-F**: Conversation memory — topic tracking, smarter thresholds
7. **Phase 3-B/C**: Relevance scoring + performance — code-aware matching

## Completed in v0.5.20-v0.5.22

- D-95: Inline CCR for tiny entries (<100 bytes, in-memory HashMap)
- H-55: Git diff --stat summary in pre_llm_hook (30s cache)
- G-90: Prometheus /metrics endpoint (requests, ccr, tokens, latency, ratio)

## Remaining (prioritized)

1. D-76-80: Single-mode simplification (removes cache complexity, ~200 lines removed)
2. D-94-96: Streaming compression + batch CCR writes
3. D-36-44: In-process file index + code-aware relevance
4. D-57: pre_tool_call hook for file dependency pre-caching
5. D-85-86: Conversation topic memory + diff detection enhancement
