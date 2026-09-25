# Hermes session compression trigger diagnostics

Recipe for "why did my session compact at N tokens when the model has a much
larger window?" - or the inverse (a session riding into the provider limit
without compacting).

## Evidence, in order

1. Compaction telemetry: `grep -h "compression attempt telemetry" ~/.hermes/logs/agent.log`
    - one JSON line per attempt, the smoking gun:
    - `effective_threshold` - the trigger that won (tokens).
    - `main_context_limit` - the model window as Hermes knows it.
    - `effective_aux_context` - the aux compression model's window (equal to the
      main window when auto resolution picks the main model).
    - `current_estimated_tokens`, `trigger_source`, `session_id`.
      Per-session confirmation: `grep -h "threshold=" ~/.hermes/logs/agent.log`
      ("Compression budget rearmed ... prompt=... < threshold=N" lines).
      Aphrodite-side check: `aphrodite_stats` reports session + proxy health
      counters for the CCR engine, and `aphrodite_rebuild` reports the loaded
      dylib version - rule out proxy issues before blaming core compaction.
2. `~/.hermes/config.yaml` - `compression:` (`threshold` ratio, `threshold_tokens`
   cap - often absent, see the merged-default gotcha below) and `model_overrides:`
   per provider/model `context_window`.
3. `~/.hermes/context_length_cache.yaml` - detected lengths per `model@endpoint`.
4. `~/.hermes/state.db` - `sqlite3 ~/.hermes/state.db "SELECT id, model, input_tokens FROM sessions ORDER BY started_at DESC LIMIT 8"` ties sessions to models.
5. Source of truth for the math (read before explaining):
   `~/.hermes/hermes-agent/agent/context_compressor.py` (`_derive_trigger`,
   `_effective_threshold_percent`, `_compute_threshold_tokens`,
   `_apply_threshold_tokens_cap`), `agent/agent_init.py` (compression settings
   parse), `hermes_cli/config_defaults.py` (defaults).

## The trigger math

```
threshold_tokens = min( ratio × effective_window, threshold_tokens_cap )
effective_window = context_length − max_tokens   (provider default → no reservation)
ratio = compression.threshold                    (default 0.50)
        floored at 0.75 - raise-only - for windows < 512_000
        (_SMALL_CTX_WINDOW_LIMIT / _SMALL_CTX_THRESHOLD_PERCENT)
cap   = compression.threshold_tokens             (defaults to 256_000)
```

Example that motivated this file: window 1,310,720, ratio 0.75 → ratio trigger
983,040, but `min(983,040, 256_000) = 256_000` → compaction at ~256K. That is
the standard answer to "compacted at 256K with a 1M+ window".

## Overriding the cap

The 256_000 default is MERGED into the effective config, so removing the key
does nothing - set it explicitly:

```yaml
compression:
    threshold: 0.75
    threshold_tokens: 786432 # 0.75 × 1,048,576 - or null / 0 for ratio-only
```

`agent_init._positive_int` maps 0 → None → cap disabled, so `null` and `0` both
mean ratio-only.

Trade-off to state when recommending a raise: compaction at ~250K already took
~175-210 s per summary call (deepseek-flash on Cloudflare); a higher threshold
is slower per compaction and risks riding into the provider's hard limit -
that is why the cap exists.

## Other paths that move the trigger

- Aux-model lowering: when the aux compression model's window is below the main
  trigger, `_lower_threshold_to_aux_context` (agent/conversation_compression.py)
  sets the session threshold to the aux window and logs "Auto-lowered this
  session's threshold to N tokens" (+ a ⚠ notice). Rule it out by checking
  `effective_aux_context` in the telemetry. Config: `auxiliary.compression`
  (`provider: auto`, `model: ''`, `free_only`).
- Aphrodite is NOT the compactor: the CCR engine's `should_compress()` always
  returns False (crates/aphrodite/templates/**init**.py) and `engine_threshold_pct`
  has no consumer in the proxy (crates/aphrodite/src/proxy.rs comment). CCR only
  shrinks tool output into markers; a token-threshold "compression" is Hermes
  core compaction.
