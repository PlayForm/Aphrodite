# Counterfactual analysis: "how much did CCR save" from recorded sessions

Retroactively estimating what a session's context WOULD have cost without
Aphrodite/CCR, from the recorded evidence (state.db + request dumps). The
The reusable implementation lives in
`~/.hermes/tmp/session-bench/` (lib.py, analyze.py,
compare.py, README.md) - run those instead of re-deriving the math.

## Data model (verified facts)

- `messages.content` in state.db stores the transcript AS SENT to the model:
  large tool outputs appear as `<<<CCR:hash|type|size>>>` marker blocks.
  Original content is NOT persisted - the inline store is session-scoped and
  wiped on dylib reload. The marker's `size` field (original BYTE count) is the
  only trace of the original.
- Marker hash = BLAKE3(content) truncated to 40 hex
  (`headroom_core::ccr::compute_key`, vendor/headroom/crates/headroom-core/src/ccr/mod.rs).
  blake3 (pip) reproduces it - use it to match markers to full content when
  the content exists somewhere (current session only; old sessions' inline
  stores are gone).
- `sessions.input_tokens + cache_read_tokens` = API BILLING (prompt-cache
  read/write accounting), ~2.9x the local payload size. NEVER use it as the
  "with" side of a context comparison - it inflates the denominator and
  smears the ratio.
- `~/.hermes/sessions/request_dump_*.json` are written ONLY on
  `max_retries_exhausted` (failure/retry samples). Valid for validating marker
  presence (transcript markers == dump markers, observed 0 leaks), NEVER for
  totals.
- `aphrodite_retrieve` tool results re-enter FULL content as new tool
  messages (the transform never re-compresses retrieval responses). Retrieval
  therefore CANCELS that block's savings - count it (a few % of savings in
  practice: ~2M tok re-entered vs ~156M saved over 200 sessions).
- The system prompt is NOT stored in state.db (sessions.system_prompt is
  NULL, no system-role messages). Measure it from a request dump
  (~6,446 tok/call observed); it is fixed overhead on BOTH sides - changes
  only the %, never the delta.

## Method (context-level, not billing)

1. Calibrate chars/tok on full-content (non-markerized) messages: measure
   ~3.87 chars/tok on this workload (code/diff/ls/log-heavy). Report a
   SENSITIVITY BAND (3.0-4.5) around it - the ratio is the one real
   assumption.
2. Per session, walk active messages in order. Assistant message = API call
   boundary (payload = system prompt + all active messages so far).
3. Per call: `stored` = tokenize payload as stored (markers ~33 tok/block
   including preview); `expanded` = stored + (marker_bytes/ratio) per marker,
   minus nothing else. `delta` = expanded - stored.
4. Accumulate per CALL, not per marker: each subsequent call re-sends the
   accumulated history (prompt caching), so a marker's savings apply on every
   later call.
5. Subtract retrieval re-entry tokens for the NET figure.
6. Report: aggregate + per-session median (median is much lower than the
   mean/aggregate - savings concentrate in long tool-heavy sessions),
   per-type compression ratios (diff ~23x, ls ~37x, terminal ~74x,
   source_code ~45x, build ~23x), and the normal-user vs heavy-user framing:
   chat-only ~0%, typical dev ~5-25%, multi-agent fan-out the most.

## A/B protocol (two controlled sessions)

- Session A: Aphrodite on + `aphrodite_debug(on=true)` (writes
  ~/.hermes/aphrodite/debug.<session-id> with per-turn compression events -
  cross-check marker counts there).
- Session B: stock Hermes (plugin disabled).
- Same ~1M-token workload in both (atomization/refactor with fan-out
  delegation produces the big-output profile CCR feeds on).
- Compare: context saved % (A vs ~0% for B), per-call stored-token growth
  (A stays flat where B grows), billed input+cache totals, tool-call counts,
  retrievals, wall time. Analyze with `compare.py --aphrodite <id> --stock
<id>`.

## Known limits (state them in every report)

- Exact originals behind markers are gone; byte-size + ratio band is an
  estimate, not a measurement.
- "Same transcript, markers expanded" is not a true no-Aphrodite run - the
  model behaves differently with full content visible (fewer reads, different
  outputs). Two valid questions: (1) how much context did marker substitution
  remove from OBSERVED requests (this method); (2) how much does Aphrodite
  change end-to-end usage on real tasks (needs matched runs, question (2) is
  the A/B protocol above).
- Exact per-request pre/post capture with the provider tokenizer requires
  proxy instrumentation (crates/aphrodite/src/proxy.rs) - the follow-up that
  converts the estimate into a measurement.
