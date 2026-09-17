# ISSUE-11 FIX DESIGN - Complete fix design space for the JSON-preview bug (`[text:1L 2B | ok]`)

**Repo:** PlayForm/Aphrodite, branch `Development`, aphrodite + aphrodite-hermes 1.4.5

**Date:** 2026-09-17

**Predecessors:** `.hermes/notes/ISSUE-11-ROOT-CAUSE.md` (root cause), `.hermes/notes/ISSUE-11-VERIFY-1.4.5.md` (verified NOT FIXED on 1.4.5)

**Scope of this doc:** complete fix design space, test constraints, recommendation, regression tests, preview-format coupling audit. No code changes, no commit.

---

## 0. Anatomy recap (what any fix must respect)

- `compress_into` (`crates/aphrodite-hermes/src/tools.rs:185-223`) runs `unwrap_hermes_result` on **every** `aphrodite_compress` call, before the caller's `type` hint is consulted. `unwrap` returning `Some((fragment, type))` **silently discards the caller's hint** (hint is only used in the `else` branch, tools.rs:196-198).
- Offending branch: tools.rs:123-126 - `{"success": true}` with `obj.len() <= 2` → `Some(("ok", "text"))`. Same bug class, ungated: tools.rs:127-131 (string `success` field, any object size) and tools.rs:167-172 (priority keys `description|summary|result|message|preview|found`, any object size).
- Storage/retrieval is lossless and must stay lossless: `unwrap` only picks the classify/preview fragment; the original `content` is what gets hashed and stored (tools.rs:201-205). Every candidate below must preserve this.
- Two entry points share the heuristic: the **explicit tool path** (`aphrodite_compress` handler → `compress_into`, tools.rs:193) and the **hook path** (`transform_tool_result` lib.rs:414, `transform_terminal_output` lib.rs:435). The hook path has **no hint** to disambiguate - Hermes hands it the raw tool-result envelope (`{"output":...,"exit_code":N}`, `{"total_count":...,"matches":[...]}`, or for plugin/custom tools the tool's own JSON, e.g. `{"success":true,"data":{...}}`).
- Caller hint values: the agent passes `type` (issue repro uses `"tool_result"`; schema enum is `["code","log","diff","error","json","build_output","text"]` - `"tool_result"` is off-enum but the handler accepts any non-empty, non-`"text"` string; `"text"` is documented as "treated as no hint", schemas.rs:83-87). The plugin `__init__.py` passes nothing - it is a thin registration shim; smoke scripts pass `type=text`/`type=code` on plain text.

---

## 1. Fix candidates - complete design space

**#:** (a)

**Candidate:** **Hint wins + full-content preview for JSON** (prior agent's minimal fix, ROOT-CAUSE §4)

**Mechanism:** In `compress_into` unwrap branch: if `!hint.is_empty() && hint != "text"` → if `content.trim_start().starts_with('{')` use `(content, hint)` else use `(c, hint)`. Unwrap result only used when hint is empty/"text".

**Tests at risk (pinned):** **None.** `test_unwrap_hermes_result_table` (tools.rs:538) tests the fn directly (untouched). `test_compress_preserves_terminal_wrapper_metadata` (tools.rs:654) passes no hint → unchanged path → still `terminal`. `test_compress_preserves_original_when_content_looks_like_a_wrapper` (tools.rs:636) no hint → unchanged. Hook tests (lib.rs:852, 778) untouched. All roundtrip tests pass (storage untouched).

**New tests needed:** (1) Explicit-path repro test (issue shape + `type:"tool_result"`); (2) no-hint regression guards so default behavior is pinned.

**Risk / notes:** **Low.** Hint is caller intent (schema contract: "trusts the `type` hint"). Caveat: with a hint + genuine envelope (`{"output":...,"exit_code":1}`), preview shows the wrapper JSON instead of the extracted output (a' refinement: use `(c, hint)` for known envelope shapes, full content only for non-envelope JSON). Hook path **not fixed** by (a) alone.

---

**#:** (b)

**Candidate:** **Remove the success-bool `"ok"` collapse** (tools.rs:123-126) so `{"success":true}` classifies as json

**Mechanism:** Delete/never return the `("ok","text")` fragment; `{"success":true}` and `{"success":true,"data":...}` fall through to `None` → `detect_type` (json bucket) or the hint path.

**Tests at risk (pinned):** **tools.rs:570 table case `("success bool only", {"success":true}, Some(("ok","text")))` BREAKS** - must be updated to `None` (this pin is the buggy behavior; ROOT-CAUSE flags it as "pins the offending collapse"). Nothing else: diff branch (tools.rs:110) fires before success; terminal/search/content/error branches unaffected; lib.rs hook tests use `output`/`exit_code` - unaffected.

**New tests needed:** (1) bare `{"success":true}` → json type, honest preview (no `| ok]`); (2) hook-path repro (see §3); (3) update table case + add `{"success":true,"data":{...}}` → `None` row.

**Risk / notes:** **Low on hook path:** per `Maintain/hermes_tool_output_formats.json` **no Hermes tool returns a bare `{"success":true}` envelope** (write_file is `{"status":"written","path":...}`, patch carries `diff`). Even if one did, collapsing it to `"ok"` loses the payload - the collapse is lossy for any 2-key object. Fixes the **hook-path manifestation** (no hint exists there).

---

**#:** (b')

**Candidate:** (b) + guard the same-bug-class branches: success-**string** (tools.rs:127-131) and priority-keys (tools.rs:167-172) must not collapse multi-key objects

**Mechanism:** Restrict both branches to single-key objects (`obj.len() <= 1`), or require the extracted string to be the object's dominant content.

**Tests at risk (pinned):** Breaks table cases **if** scoped strictly: `"success string message"` (tools.rs:573, single key - survives a `len<=1` guard, breaks if branch removed entirely) and `"priority key fallback"` (tools.rs:592, `{"name","description"}` - skill_view is a genuine Hermes envelope; breaking it regresses skill_view previews). Recommend keeping priority keys (they serve real shapes) and only gating the success branches.

**New tests needed:** Priority-key/success-string regression cases with multi-key payloads (`{"success":"ok","data":{...}}` must not collapse).

**Risk / notes:** **Medium** - wider behavior change than the issue needs; the issue repro only exercises the bool branch. Optional hardening.

---

**#:** (c)

**Candidate:** **Gate `unwrap_hermes_result` to the hook path only** - never on explicit `aphrodite_compress`

**Mechanism:** `compress_into` skips unwrap; explicit path always uses hint/detect_type.

**Tests at risk (pinned):** **tools.rs:654 BREAKS**: pins that the explicit path unwraps `{"output":"error: broke\nexit code: 1\n","exit_code":1}` → type `terminal` **with no hint** (deliberate 01-F2 design: the agent may compress a Hermes-wrapped terminal envelope verbatim). Hook tests still pass.

**New tests needed:** Would need to rewrite/repurpose tools.rs:654 (assert json bucket instead).

**Risk / notes:** **High.** The explicit path _does_ need unwrapping when the agent parks a wrapped tool result without a hint; and (c) does **not** fix the hook-path collapse (no hint there), so the issue's automatic-compression path stays broken. Rejected as a standalone fix.

---

**#:** (d)

**Candidate:** **Stronger envelope signature** - unwrap only on known Hermes envelope shapes (`output`+`exit_code`, `diff`, `error`, `total_count`/`matches`, `content`+`total_lines`); drop/guard ambiguous branches

**Mechanism:** Scoped D1: drop only success-bool. Scoped D2: drop success-bool + gate success-string. Scoped D3: also gate priority keys.

**Tests at risk (pinned):** D1 breaks only tools.rs:570. D2 additionally breaks tools.rs:573 if string branch removed (survives a `len<=1` guard). D3 additionally breaks tools.rs:592.

**New tests needed:** Same as (b)/(b') + explicit-path repro.

**Risk / notes:** **D1 ≈ (b)**, and because `{"success":true,"data":...}` then returns `None` on the explicit path too, D1 **also fixes the explicit repro without (a)** (hint honored in the `else` branch). D3 risks skill_view/aphrodite-tool previews (priority keys are the only thing serving them) - keep them.

---

**#:** (e)

**Candidate:** **`unwrap` returns original content + a json type instead of a fragment** when the object is not a known envelope shape

**Mechanism:** Change the success-bool branch to `Some((content.to_string(), "json"))` (or detect_type) instead of `("ok","text")`.

**Tests at risk (pinned):** tools.rs:570 BREAKS (expected value changes). Everything else passes.

**New tests needed:** Same as (b) + a type-assertion variant.

**Risk / notes:** **Medium - strictly worse than (b) for the explicit path:** `Some` still suppresses the caller's hint, so `type:"tool_result"` would become `json` (type-flip persists, only the preview is honest). Only sensible combined with (a).

---

**#:** (f)

**Candidate:** **Combinations**

**Mechanism:** (a)+(b): explicit path fixed by hint-priority, hook path fixed by dropping the collapse - complementary, disjoint surfaces. (a)+(e): hook path gets `json` + full preview (≈(b) outcome) but (e)'s `Some` is redundant once (a) handles the explicit path. (a)+(c): (c)'s breakage of tools.rs:654 persists - pointless. (a)+(d-D1) ≡ (a)+(b).

**Tests at risk (pinned):** For (a)+(b): only tools.rs:570 (the buggy pin) needs updating.

**New tests needed:** Full set from (a) + (b).

**Risk / notes:** **This is the recommendation - see §2.**

Additional candidate considered and rejected: **classifier-side fix in `crates/aphrodite/src/preview.rs`** - no change needed there; the generic arm already renders any type honestly from whatever content it is given (60-char hint cap, preview.rs:294-304), and the json arm (`json_array|json|json_list`, preview.rs:273) already produces a good `[json:N...]` preview. The bug is entirely in the bridge's _choice of fragment + type_, not in preview rendering.

---

## 2. Recommended fix

**Primary - (a): honor the caller hint + full-content preview for JSON on the explicit tool path.**

In `compress_into` (tools.rs:193-194), when `unwrap_hermes_result` returns `Some((c, t))`:

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

**Complementary - (b): remove the success-bool `"ok"` collapse (tools.rs:123-126).**

The `{"success":true} && obj.len() <= 2` branch is unreachable-by-design for every genuine envelope (output/exit_code, diff, error, total_count/matches, content/total_lines all fire above it) and only ever false-positives on user payloads - exactly the issue shape. Deleting it makes `{"success":true,"data":{...}}` fall through to `None` on **both** paths: the hook path (no hint) then classifies it as json with an honest full-content preview, and the explicit path falls into the `else` branch where the hint is honored.

**Justification:**

1. **Zero risk on the pinned suite for (a)** - it only changes behavior when a non-default hint is present, which no pinned test exercises; the no-hint envelope behavior (tools.rs:654 `terminal`, lib.rs:852 hook unwrap) is bit-identical.
2. **(b) fixes the same bug on the hook path, where no hint exists** - this is the automatic-compression path every real JSON tool result flows through; without (b), `(a)` leaves the issue half-open for hook compressions.
3. Both changes are local to `compress_into` / `unwrap_hermes_result`; `preview.rs`, `marker.rs`, `resolve.rs`, storage, and retrieval are untouched. `aphrodite_reclassify` (tools.rs:431-458) already recomputes type+preview from the full stored content, confirming the honest rendering exists and the fix aligns with it.
4. The `(b)` pin that breaks (tools.rs:570) pins the buggy collapse itself; updating it to `None` is the test catching up with intent (ROOT-CAUSE §3 already flags this).

Optional hardening (not required to close the issue): (b') gate the success-string and priority-key branches with a `len<=1` guard so multi-key objects like `{"success":"ok","data":{...}}` or `{"result":"ok",...}` cannot collapse; keep priority keys for skill_view/aphrodite-tool shapes. `(a')` refinement: for JSON that matches a _known_ envelope shape, keep the extracted content for the preview but the hint for the type (`(c, hint)`) - full content only for non-envelope JSON.

---

## 3. Exact regression tests to add

**3.1 tools.rs - explicit-path repro (the issue's shape, primary regression test):**

```rust
// ── ISSUE-11: JSON tool results must not collapse to a literal "ok"
// fragment. {"success":true,"data":{...}} with an explicit type hint
// must keep the hinted type and preview the FULL payload (honest L/B),
// never "[text:1L 2B | ok]".
#[test]
fn test_compress_json_payload_with_hint_keeps_type_and_full_preview() {
    let _g = crate::test_guard();
    let content = serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}).to_string();
    let compressed = dispatch(
        "aphrodite_compress",
        &serde_json::json!({"content": content, "type": "tool_result"}).to_string(),
    );
    assert_eq!(compressed["type"], "tool_result", "explicit hint must win over the envelope heuristic");
    let preview = compressed["preview"].as_str().unwrap();
    assert!(preview.starts_with("[tool_result:"), "preview must reflect the hinted type: {preview}");
    assert!(!preview.contains("| ok]"), "preview must not collapse to the literal 'ok' fragment: {preview}");
    assert!(preview.contains("\"success\""), "preview must show the real JSON payload: {preview}");
    assert_eq!(compressed["size"], content.len(), "size must count the full payload, not the fragment");
    // Round-trip stays lossless regardless of preview/type.
    let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": compressed["hash"]}).to_string());
    assert_eq!(retrieved["content"], content);
}
```

**3.2 tools.rs - no-hint bare success never collapses (complementary to (b)):**

```rust
#[test]
fn test_compress_bare_success_object_is_json_not_ok() {
    let _g = crate::test_guard();
    let content = serde_json::json!({"success": true}).to_string();
    let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
    assert_ne!(compressed["type"], "text", "bare success object must classify as json, got {:?}", compressed["type"]);
    assert!(!compressed["preview"].as_str().unwrap().contains("| ok]"));
}
```

**3.3 tools.rs - update the pinned table (tools.rs:570):** change `("success bool only", json!({"success": true}), Some(("ok", "text")))` to expect `None`, and add `("success bool with data payload", json!({"success": true, "data": {"web": [{"title": "x"}]}}), None)` plus a success-string variant `("success string with data payload", json!({"success": "ok", "data": [1]}), None)`.

**3.4 lib.rs - hook path must preserve user JSON payloads (the automatic-compression path):**

```rust
// ── ISSUE-11: transform_tool_result fires on EVERY tool result with no
// hint available - a JSON payload containing success:true must not
// collapse to "[text:1L 2B | ok]" there either.
#[test]
fn test_call_hook_transform_tool_result_preserves_user_json_payload() {
    let _g = crate::test_guard();
    aphrodite_hermes_call_hook(
        CString::new("session_start").unwrap().as_ptr(),
        CString::new("{}").unwrap().as_ptr(),
    );
    let wrapped = serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}).to_string();
    let args = serde_json::json!({"tool_name": "custom_tool", "result": wrapped}).to_string();
    let hook_ptr = aphrodite_hermes_call_hook(
        CString::new("transform_tool_result").unwrap().as_ptr(),
        CString::new(args).unwrap().as_ptr(),
    );
    let marker_str: String = serde_json::from_str(&unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned())
        .expect("a marker string");
    aphrodite_hermes_free_string(hook_ptr);
    assert!(!marker_str.contains("| ok]"), "hook path must not collapse user JSON: {marker_str}");
    let preview = with_shared(|state| state.recent_markers.last().unwrap().preview.clone());
    assert!(!preview.contains("| ok]") && !preview.starts_with("[text:"), "recorded preview must reflect the JSON payload: {preview}");
}
```

**3.5 (Optional hardening, if (b') lands)** - table rows for priority/success-string multi-key shapes asserting `None`, and a hook-path case for `{"result": "ok", "items": [...large...]}`.

All new tests must keep the existing `test_guard()` isolation and must not assert anything about retrieval other than losslessness (storage is out of scope of this fix).

---

## 4. Preview-format coupling audit (does anything depend on tiny previews?)

**Verdict: NO code depends on the preview format or its size. Longer JSON previews are safe.**

- **Plugin `plugins/aphrodite/__init__.py`** - thin registration shim (verified full read): forwards tool args/hook kwargs to the dylib via ctypes, zero parsing of previews or markers. No length assumptions.
- **`plugins/aphrodite/tests/*`** - roundtrip/CCR-presence checks only (test_perf_probe.py asserts `<<<CCR:` in output and roundtrip equality); no preview-format parsing.
- **`resolve.rs`** parses only the `<<<CCR:hash|type|size>>>` header (and tolerates a `hash|type|size` suffix in the _hash argument_, resolve.rs:55-61). The preview body after the header is inert LLM-facing content - never parsed.
- **`marker.rs:93-95, 240`** - the only structural constraint is generation-side: `build_preview` must not re-wrap an already-bracketed preview (the `[text:[text:53L 1913B]]` doubling bug). The fix (a) passes raw content to `build_preview`, which renders a single bracket - no interaction.
- **`hooks.rs:314-326`** (chain_split) merges per-segment error hints _inside_ the preview brackets by trimming the trailing `]` - tail-only string surgery; content-agnostic. Longer previews fine.
- **`aphrodite_search`** (tools.rs:341-344) matches `preview.to_lowercase().contains(query)` - substring match on the text; a JSON preview is _more_ searchable, not less.
- **`aphrodite_catalog` / `aphrodite_reclassify` / `aphrodite_diff`** - pass preview/type through as opaque strings; reclassify recomputes from full content (already produces the honest preview for the issue shape - a no-code workaround).
- **The `type` field** is compared exactly in `aphrodite_search` type_filter and rendered in `<<<CCR:hash|type|size>>>`. `"tool_result"` is off the schema enum (schemas.rs:85) but accepted by the handler and fully searchable/round-trippable; the fix does not change type semantics, only which type wins.
- **LLM-facing consumption** (flow.rs:25-31 directive text): the agent is instructed to read the marker's type/size and "skip when the preview or type/size already answers" - previews are prose for the model; longer, honest JSON previews are strictly better signal.
- **Preview length is bounded anyway**: the generic `_` arm truncates the hint to 60 chars (preview.rs:294-304); verified empirically (VERIFY table: 18.9 KB JSON → `[tool_result:1L 18931B | {"items": [{"id": 0, "name": "item-0", ...]`).
- **Docs** (README.md, docs/proxy/compression.md, plugins README) document preview formats as tables - documentation only; the "15 tokens of metadata" claim is descriptive, not contractual.

The only "coupling" is the inverse: the _bug_ couples preview correctness to the fragment choice. Fixing the fragment restores the documented contract ("trusts the `type` hint", schemas.rs:68).

---

## 5. Test matrix - pinned tests vs each candidate (summary)

| Pinned test                                                                 | (a)      | (b)    | (b')                    | (c)      | (d-D1) | (d-D2)                  | (d-D3)         | (e)      | (a)+(b) |
| --------------------------------------------------------------------------- | -------- | ------ | ----------------------- | -------- | ------ | ----------------------- | -------------- | -------- | ------- |
| tools.rs:538 table - terminal/diff/error/search/content/priority rows       | ✔        | ✔      | ✔*                      | ✔        | ✔      | ✔*                      | ✘ priority row | ✔        | ✔       |
| tools.rs:570 `{"success":true}` → ("ok","text")                             | ✔ (kept) | ✘→None | ✘→None                  | ✔ (kept) | ✘→None | ✘→None                  | ✘→None         | ✘→change | ✘→None  |
| tools.rs:573 success-string → extract                                       | ✔        | ✔      | ✔ (len≤1) / ✘ (removed) | ✔        | ✔      | ✔ (len≤1) / ✘ (removed) | ✘              | ✔        | ✔       |
| tools.rs:636 wrapper roundtrip (retrieve lossless)                          | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |
| tools.rs:654 explicit terminal envelope → type `terminal` (no hint)         | ✔        | ✔      | ✔                       | **✘**    | ✔      | ✔                       | ✔              | ✔        | ✔       |
| tools.rs:616/721/786/801 roundtrip/center/catalog/test                      | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |
| lib.rs:852 hook unwraps terminal envelope (no `[json:`/`[text:`)            | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |
| lib.rs:591/631 chain-split marker rendering                                 | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |
| lib.rs:778 hook telemetry (error event)                                     | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |
| tests/test_aphrodite_hermes_plugin.py (FFI roundtrips, list/schema/version) | ✔        | ✔      | ✔                       | ✔        | ✔      | ✔                       | ✔              | ✔        | ✔       |

✔ = passes unchanged; ✘ = breaks (pin must be updated); * = depends on exact guard scope.

## 6. Files

- This report: `.hermes/notes/ISSUE-11-FIXDESIGN.md`.
- No repo source files modified; nothing committed.
