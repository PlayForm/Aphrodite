# Context Engine Activation

## Config
```bash
hermes config set context.engine aphrodite
hermes config set compression.enabled false   # MUST be false - see pitfall below
# Revert: hermes config set context.engine compressor
```

`compression.enabled` is Hermes' built-in summarizer - a separate system. It MUST be `false` when using the aphrodite context engine. If both are active, they fight each other and degrade quality.

## How it works

The context engine receives the FULL message list via `compress()`. Mutations take effect (unlike `pre_llm_call` which gets a copy). Engine offloads middle messages to CCR and returns a shorter list: `head + [system_placeholder] + tail`.

## Current Defaults (v1.56.0)

| Config | Env Var | Default | Meaning |
|--------|---------|---------|---------|
| Threshold % | `APHRODITE_ENGINE_THRESHOLD_PCT` | 50 | Compress at N% context fill. -1 = always, 0 = disabled, >0 = fill% |
| Protect first | `APHRODITE_ENGINE_PROTECT_FIRST` | 1 | Keep first N messages raw |
| Protect last | `APHRODITE_ENGINE_PROTECT_LAST` | 1 | Keep last N messages raw |
| Min messages | `APHRODITE_ENGINE_MIN_MSGS` | 4 | Don't compress below N total messages |

## should_compress() - Token-Based Gating

`should_compress()` returns `False` when:
- `threshold_percent <= 0` AND `threshold_percent != -1` (disabled; -1 = always compress)
- `prompt_tokens` AND `last_prompt_tokens` are both 0/missing (unknown token count)
- `context_length` is missing
- `(tokens / context_length) * 100 < threshold_percent` (below fill threshold)

`last_prompt_tokens` is set by `update_from_response(usage)` which is called after each LLM response. On first turn, `last_prompt_tokens` is 0, so `should_compress` returns `False` (can't determine fill % yet). Compression typically kicks in on turn 2+ after the first response's token count is known.

**PITFALL**: The v1.55.0 code had `tokens = prompt_tokens or self.last_prompt_tokens or (self.context_length or 1000000)` - using `context_length` as a fallback token value always returned `pct=100%` and `should_compress` was always `True`. Fixed in v1.56.0: requires actual token data.

## compress() - Internal Guards

`compress()` returns early (no compression) when:
- `len(messages) <= min_messages_to_compress` (default: 4) - not enough messages
- `len(middle) < 3` - middle slice too thin after head/tail protection
- `len(packed) < 200` - packed middle content too small to bother

**Message-count dependency**: With `protect_first=1` and `protect_last=1`, you need 5+ total messages to produce `middle = messages[1:-1]` with ≥3 messages. Formula: `total >= protect_first + protect_last + 3`.

## Testing the Engine

### Quick test config (aggressive compression)
```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1   # compress at 1% fill (quick trigger)
APHRODITE_ENGINE_MIN_MSGS=4        # allow compression on short conversations  
APHRODITE_ENGINE_PROTECT_FIRST=1   # keep only first message raw
APHRODITE_ENGINE_PROTECT_LAST=1    # keep only last message raw
```

With this config and a 1M context window, compression triggers at 10K tokens (about 5-6 tool-heavy turns). Need 5+ messages total.

### Verify engine loaded
Check Hermes startup logs for: `Using context engine: aphrodite`

### Verify compression works
1. Start Hermes with `APHRODITE_ENGINE_THRESHOLD_PCT=1` env var
2. Build context (5+ messages including tool calls)
3. Check proxy logs for: `context_engine: compressed N msgs → CCR:hash`
4. The message count should drop as middle messages are offloaded to CCR

### Debugging 0 compressions
If `aphrodite_stats` shows `compressions: 0` despite high token count:
1. Check `protect_first_n`/`protect_last_n` in stats - if 0, engine may not be loaded or env vars aren't set
2. Check `last_prompt_tokens` - must be non-zero (set after first LLM response)
3. Check message count - need `≥ protect_first + protect_last + 3` messages
4. Check `threshold_tokens` in stats - should be 1 (means engine is calculating threshold)
5. Engine only activates mid-session; on session start `last_prompt_tokens` is 0
