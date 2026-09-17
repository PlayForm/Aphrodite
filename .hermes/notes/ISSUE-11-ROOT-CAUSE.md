# ISSUE-11 ROOT CAUSE - Broken CCR preview for JSON tool results (`[text:1L 2B | ok]`)

**Repo:** PlayForm/Aphrodite, branch `Development`, engine `aphrodite` 1.4.5 + bridge `aphrodite-hermes` 1.4.5
**Date:** 2026-09-17
**Verdict:** bug EXISTS in the current 1.4.5 tree (both crates). Not fixed above 1.3.7.

---

## 1. Exact code path that emits `ok` + `2B` + type `text`

The bug is in the **bridge crate** (`crates/aphrodite-hermes`), not in `crates/aphrodite/src/preview.rs`. Reproduction
(`aphrodite_hermes_dispatch_tool("aphrodite_compress", {"content": <json>, "type": "tool_result"})`) funnels through:

```
crates/aphrodite-hermes/src/lib.rs:235  aphrodite_hermes_dispatch_tool
  -> lib.rs:240                         tools::dispatch("aphrodite_compress", args)
  -> tools.rs:229-237                   aphrodite_compress handler: content = args["content"], hint = args["type"]
  -> tools.rs:236                       compress_into(state, content, hint="tool_result", None)
  -> tools.rs:193                       unwrap_hermes_result(content)   <-- THE BUG SITE (unwrap succeeds)
  -> tools.rs:123-126 | 127-131 | 167-172   returns Some(("ok", "text"))
  -> tools.rs:202                       aphrodite::build_preview(&"text", &"ok")
  -> crates/aphrodite/src/preview.rs:178-181  effective = "text" (no semantic match for "ok")
  -> preview.rs:294-304                 generic `_` arm: format!("[{}:{}L {}B | {}]", "text", 1, 2, "ok")
  -> tools.rs:203                       ccr_marker(&hash, "text", content.len()=737, "[text:1L 2B | ok]")
  -> crates/aphrodite/src/marker.rs:109 <<<CCR:{hash}|text|737>>>\n[text:1L 2B | ok]
```

Precise emitters:
- **`"ok"` (2-byte fragment):** `tools.rs:123-126` - `{"success": true}` with `obj.len() <= 2` returns the hardcoded
  literal `("ok", "text")` (the "boolean-ish extractor"). Same result via `tools.rs:127-131` (`{"success": "ok"}`) and
  `tools.rs:167-172` (priority keys `description|summary|result|message|preview|found` - e.g. `{"result": "ok", ...}`
  of any size), and `tools.rs:81-106` (`{"output": "ok", "exit_code": 0}` → `detect_type("ok")` = `"text"`).
- **`2B` + `1L`:** `preview.rs:170-171` - `build_preview` computes line/byte counts of the **fragment** `"ok"`, not
  the stored 737-byte payload.
- **type `text`:** `tools.rs:193-194` - when `unwrap_hermes_result` returns `Some`, its own type (`"text"`) is used
  verbatim and the caller's explicit `hint` (`"tool_result"`) is **silently dropped** (the hint is only consulted in
  the `else` branch, `tools.rs:196-198`).
- **marker size 737:** `tools.rs:203` - `ccr_marker(..., content.len(), ...)` uses the ORIGINAL full content, which is
  why `<<<CCR:hash|text|737>>>` is right while the inline preview is wrong. Storage/hash is lossless (`tools.rs:205`
  stores original content; retrieval round-trips byte-for-byte - matches the issue).

## 2. Mechanism - why JSON lands there

`compress_into` (tools.rs:185) applies `unwrap_hermes_result` (tools.rs:68-177) to **every** `aphrodite_compress`
call. That heuristic was written for the **hook path** (`transform_tool_result`, lib.rs:414), where Hermes genuinely
wraps tool results in envelopes (`{"output":..., "exit_code":N}`, `{"total_count":N,"matches":[...]}`, etc.) so the
classifier doesn't produce a useless `[json:1items 1L]` preview (design pinned by
`lib.rs:847-890` `test_call_hook_transform_tool_result_unwraps_hermes_wrapper` and `tools.rs:654-667`).

The heuristic cannot distinguish a Hermes envelope from a **user JSON payload** (both are JSON objects). For any
object whose first meaningful string field is a tiny status word, or that is a bare `{"success": true}`, it collapses
the whole payload to that fragment **and** re-classifies it as `text`. Consequences on the explicit compress path:

1. **Type flip:** `type: "tool_result"` (passed explicitly by the caller) is discarded; marker type becomes the
   unwrap's `"text"`. (Issue fact: JSON reports `text`, plain text correctly reports `tool_result` - plain text isn't
   JSON, so `unwrap_hermes_result` returns `None` (tools.rs:70-72), the hint is honored in the `else` branch, and the
   preview shows `[tool_result:3L 37B | === CRON LIST ===]`.)
2. **Fragment preview:** `build_preview` runs on `"ok"` instead of the 737-byte payload → `1L 2B` and a useless hint.
   The classifier (`detect_type`) never even sees the JSON body - it sees the extracted 2-byte fragment (hypothesis
   (b) confirmed, with (a) as the extractor: the success-bool branch is a boolean-ish extractor returning literal
   `"ok"`).
3. **Config levers no-op (confirmed):** `model_family` (config.rs:256) and `preview_max_chars` (config.rs:258) are
   **declared but never read anywhere** (exactly 2 occurrences in the whole crates tree, both declarations). There is
   no preview-template system in this codebase - previews are always `build_preview` (preview.rs:169). Nothing in the
   engine consults these knobs, so no config can change the outcome.

Note the engine's C-ABI `aphrodite_compress` (lib.rs:272-299) does NOT unwrap - the bridge tool path (compress_into)
is a parallel reimplementation that adds the unwrap. Same content via C ABI vs bridge dispatch yields different
type/preview. The bug is exclusive to the bridge.

## 3. Exists in the current 1.4.5 tree?

**YES.** Both crates are version 1.4.5 (Cargo.toml). `unwrap_hermes_result` + `compress_into` are present in
`crates/aphrodite-hermes/src/tools.rs` (moved to the bridge in commit `8f138c1`, 2026-07-14, rewritten `e25aa85`,
`26218a5`). The exact issue output is reproducible: `dispatch("aphrodite_compress", {"content": "{\"success\": true}"
or any JSON with a `result`/`message`/`description`/`summary`/`output` field whose value is a short word,
"type": "tool_result"})` returns `{"type": "text", "preview": "[text:1L 2B | ok]", "size": <json len>, "marker":
"<<<CCR:<hash>|text|<size>>>"}`. The existing test table (tools.rs:570) even pins the offending collapse:
`{"success": true}` → `("ok", "text")`.

## 4. Minimal fix (recommended)

**Function:** `compress_into` - `crates/aphrodite-hermes/src/tools.rs:185-223` (one function; leave
`unwrap_hermes_result`, the hook path, and `preview.rs` untouched).

**Root cause to fix:** on the *explicit* tool path the caller's `type` hint is the authoritative intent, and the
content IS the payload - there is no envelope to unwrap. When the caller passes an explicit non-default hint, the
unwrap must not override it, and for JSON payloads the preview must be built from the FULL content (fragments lie
about L/B counts and hide the payload shape).

**Change** - in the `if let Some((c, t)) = unwrap_hermes_result(content)` branch (tools.rs:193-194):

```rust
let (classify_content, eff_type) = if let Some((c, t)) = unwrap_hermes_result(content) {
    // Explicit caller type hint wins over the envelope heuristic: the
    // unwrap is a guess, the hint is intent. For JSON payloads also build
    // the preview from the FULL content - an unwrap fragment ("ok")
    // produces a wrong `[text:1L 2B | ok]` (L/B count the fragment, not
    // the stored payload) and drops the shape signal entirely.
    if !hint.is_empty() && hint != "text" {
        if content.trim_start().starts_with('{') {
            (content.to_string(), hint.to_string())
        } else {
            (c, hint.to_string())
        }
    } else {
        (c, t)
    }
} else {
    let detected = aphrodite::detect_type(content);
    let ccr_type = if hint.is_empty() || hint == "text" { detected } else { hint.to_string() };
    (content.to_string(), ccr_type)
};
```

**Effect:**
- Repro (`type: "tool_result"` + JSON): type stays `tool_result`, preview becomes the generic arm over the full
  payload (`[tool_result:1L 737B | {"..."...}]` - first 60 chars) - correct type, honest preview.
- Plain text with `type: "tool_result"`: unchanged (`[tool_result:3L 37B | === CRON LIST ===]`).
- No hint (default): behavior unchanged - Hermes envelopes still unwrap (`{"output"..., "exit_code"}` → `terminal`),
  so pinned tests `tools.rs:654` (type "terminal") and `lib.rs:852` (hook path unwraps) still pass.
- Storage/retrieval: untouched (hash/store still use original `content`).
- The `_` arm of `build_preview` (preview.rs:294-304) needs no change - it already renders `[<type>:L B | first-line]`
  for any unknown type; for a JSON object classified as `json`/`json_array` (no-hint case) the json arm (preview.rs:574)
  already produces a good `[json:Nkeys ...]` preview.

**Alternative/complementary (classifier-side, optional, lower priority):** in `unwrap_hermes_result`, drop the
hardcoded `"ok"` collapse (tools.rs:123-126, `obj.len() <= 2` + bool `success`) - a bare `{"success": true}` IS the
whole payload, so returning `None` there lets `detect_type` classify it as `json_array` (or the hint type) and
prevents the literal-`ok` preview on the *hook* path too, where no hint exists to disambiguate.

## 5. Related preview-path issues spotted

1. **Hook path has the same fragment-preview bug for user JSON** (lib.rs:414 → hooks.rs:172-197): a tool result
   whose content is `{"result": "ok", ...}` or `{"success": true}` also collapses to `[text:1L 2B | ok]` with type
   `text` (no hint exists there; `unwrap_hermes_result` runs unconditionally). The success-bool collapse
   (tools.rs:123-126) and priority-key collapse (tools.rs:167-172, no size/type guard on the object) are the
   offenders. Fix 4's optional classifier change addresses this; alternatively the unwrap could require a
   *recognized envelope signature* (presence of `exit_code`/`total_count`/`matches`/`diff` keys) before collapsing.
2. **Bridge/ABI asymmetry:** engine C-ABI `aphrodite_compress` (lib.rs:272) does not unwrap; bridge
   `compress_into` does. Same input, different type/preview depending on entry point. The C-ABI behavior is the
   correct reference for the explicit tool path.
3. **`detect_semantic_type` never sees JSON** (preview.rs:33-105): irrelevant for JSON (it only upgrades generic
   buckets), but the `tool_result` type itself has no dedicated preview arm - it always lands in the generic `_`
   arm. If `tool_result` markers become common, a `"tool_result" =>` arm (reuse the JSON/semantic logic) would be a
   natural follow-up; not required for this fix.
4. **`preview_max_chars` / `model_family` dead config** (config.rs:256/258): unused levers - either wire them into
   `build_preview` or remove them; keeping dead knobs that the issue reporter already tried makes diagnosis harder.