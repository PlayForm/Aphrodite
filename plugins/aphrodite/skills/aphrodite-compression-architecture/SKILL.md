---
name: aphrodite-compression-architecture
description: "Compression architecture reference — Headroom's 9 semantic compressors vs Aphrodite's content-addressed storage, integration opportunities, benchmark pipeline, and model-aware preview design."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-dev-workflow, aphrodite-release-workflow, aphrodite-hook-reference]
---

# Aphrodite Compression Architecture

Knowledge bank for the compression landscape: Headroom's semantic compressors, Aphrodite's content-addressed storage, integration opportunities, and performance measurement.

## Remote Name

Repo remote is `Source` (ssh), NOT `origin` or `aphrodite`. All `git push` commands must use `git push Source`. Auto-release scripts, dev workflows, and skills must use the correct remote. Verify with `git remote -v`.

## Benchmark Pipeline

Combined performance + correctness pipeline:

1. `python3 scripts/benchmark.py` — direct HTTP benchmark against :9798
   - Phase 1: proxy health + stats
   - Phase 2: compression across 5 sizes × 3 types (15 variants, 3–5 iterations each)
   - Phase 3: retrieve 10 random hashes
   - Phase 4: catalog entry count
   - Output: `.hermes/benchmark-<ts>.json` + `.hermes/benchmark-history.jsonl`
   - Compares against previous run (Δ latency)

2. `aphrodite_test mode=pipeline` — correctness + feature toggles
   - 9 smoke tests (compress, retrieve, stats, health, metrics)
   - Feature toggles: debug on/off, engine on/off
   - Saves `.hermes/aphrodite/.test-results.json` with regression delta

Together they measure: HTTP latency distribution, compression ratios by size/type, proxy health, and correctness across feature toggle combinations.

## Headroom Compressor Comparison

See `references/headroom-compressor-comparison.md` for the full 9-strategy analysis.

Quick summary:
- Aphrodite: 1× ratio (content-addressed storage, no semantic reduction)
- Headroom: 9 strategies — CODE_AWARE (5–8×), SMART_CRUSHER, SEARCH, LOG, KOMPRESS (3–5×), DIFF, HTML, MIXED, PASSTHROUGH
- Integration: proxy could accept `strategy=` param, call Headroom compressor, store reduced content in CCR

## Model-Aware Preview Templates

Noted for later: different LLM families process previews differently. Design:
- `model_family` → template selector (compact/claude, code-first/deepseek, balanced/gpt)
- Pass through `_state["model"]` from session start
- Dispatch in `_make_ccr_preview()` and `build_preview()`
