# Changelog

## v0.7.12 - 2026-06-14
- feat: winged sandal logo + headroom kwargs passthrough

## v0.7.11 - 2026-06-14
- chore: bump version 0.7.10 → 0.7.11

## v0.7.6 - 2026-06-13
- chore: v0.7.6 - regenerate reports, freeze-cache tuning, save

## v0.7.4 - 2026-06-13
- feat: v0.7.4 - template-based report, cumulative benchmarks, linear arrows

## v0.5.57 - 2026-06-16
- release(aphrodite): v0.5.57/1.62.3 - all medium+low bugs fixed

## v0.5.56 - 2026-06-16
- release(aphrodite): v0.5.56/1.62.2 - critical + high bug fixes

## v0.5.55 - 2026-06-16
- feat(headroom): cache/benchmark/token modes, 1-8 workers, Hermes default provider

## v0.5.54 - 2026-06-16
- chore(aphrodite): remove duplicate shared-state definitions from _hooks/\_tools/\_resolve

## v0.5.53 - 2026-06-16
- refactor(aphrodite): consolidate shared state into _core.py - break circular imports

## v0.5.52 - 2026-06-16
- fix(aphrodite): 8 bugs - mode warning, listen optional, first-turn skip, threshold_tokens, wildcard routes, filter_content zero-match, compress size + bump v0.5.52/v1.61.0

## v0.5.51 - 2026-06-16
- fix(aphrodite): 13 bugs - cache_alive crash, \_recent_markers shadow, EMA ratio, health check, double detect, false Rust+, body read, double elapsed, port 9797 default, XDG DB path, path read security + bump v0.5.51/v1.60.0

## v0.5.50 - 2026-06-16
- fix(aphrodite): restore engine fallback + dedup in catalog - context_length needed when update_from_response not called + bump v0.5.50/v1.59.0

## v0.5.49 - 2026-06-16
- fix(aphrodite): engine defaults to context_length tokens when unknown - always compresses on threshold + bump v0.5.49/v1.58.0

## v0.5.48 - 2026-06-16
- fix(aphrodite): engine should_compress falls back to 1 token minimum - works even when update_from_response not called + bump v0.5.48/v1.57.0

## v0.5.47 - 2026-06-16
- fix(aphrodite): should_compress uses self.last_prompt_tokens as fallback - engine actually compresses now + bump v0.5.47/v1.56.0

## v0.5.46 - 2026-06-16
- feat(aphrodite): auto-expand cached CCR markers <10KB - LLM never sees aphrodite_retrieve for small cached items + bump v0.5.46/v1.55.0

## v0.5.45 - 2026-06-16
- fix(aphrodite): saturating_sub on tokens_saved - prevents overflow panic when hash > content + bump v0.5.45/v1.54.0

## v0.5.44 - 2026-06-16
- fix(aphrodite): liveness filter on catalog - only show markers with retrievable content, skip ghosts + bump v0.5.44/v1.53.0

## v0.5.43 - 2026-06-16
- fix(aphrodite): hex validation on CCR hash filter - ≥8 hex chars, removes abc123 placeholders + bump v0.5.43/v1.52.0

## v0.5.42 - 2026-06-16
- feat(aphrodite): debug info injected into [APHRODITE] catalog block - ⚙ lines show version, mode, thresholds in conversation + bump v0.5.42/v1.51.0
