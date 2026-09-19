# Envelope Model - 2026-09-19

Scope: response-envelope handling across the Aphrodite CCR engine and the
Hermes tools that feed it. Repo `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`,
branch `Development`, HEAD `a67bfd0`. Research + design reference only - no
code changes, no commits/pushes. Every claim carries a real `file:line`;
anything not verifiable is marked UNVERIFIED. Sibling catalog:
`RETURN-TYPE-CATALOG-2026-09-19.md` (per-envelope and per-content-type detail).

## 1. Verified envelope model - six handling paths

The paste's six-path model is **substantially correct**; three claims are
corrected below (F2's `success` role, F6's "everything else" wording, and the
layering of the caller hint). Each path:

| #   | Family               | Code branch        | Recognized fields                                                                                                    | Extracted payload                                                                 | Type resolution                                                                | Preview arm                                       | Status                                |
| --- | -------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------- | ------------------------------------- |
| 1   | terminal             | `tools.rs:80-106`  | `output` (str), `exit_code` (any), `error`                                                                           | `output` when non-empty                                                           | heuristic `terminal`/`build_error`/`build_output` (85-103), else `detect_type` | `terminal` (`preview.rs:895-909`) or build arms   | VERIFIED                              |
| 2   | patch / write        | `tools.rs:109-121` | `diff` (str) non-empty → `error` (str) non-empty                                                                     | `diff` text; `error` message                                                      | `detect_type` (diff text); `"text"` (error msg)                                | `diff` arm (790-829) / generic                    | CORRECTED (see below)                 |
| 3   | error-only / success | `tools.rs:122-139` | `success` (str only, single-key object)                                                                              | the success string                                                                | `"text"`                                                                       | generic                                           | CORRECTED (see below)                 |
| 4   | search               | `tools.rs:141-206` | `total_count` (u64), `matches` (array), `matches_text` (str), `truncated` (bool); `matches_format` as shape evidence | per-match `path:line:content` lines; fallback `"N total"`/`"N total (truncated)"` | `"search"` when lines exist; `"text"` for the total-only fallback              | `search` (`build_search_preview`, 1305) / generic | VERIFIED (+extended)                  |
| 5   | read / content       | `tools.rs:208-212` | `content` (str), `total_lines` (N)                                                                                   | `content` verbatim                                                                | `detect_type`                                                                  | whatever content arm matches                      | VERIFIED (label mismatch noted in §8) |
| 6   | generic              | `tools.rs:214-230` | single-key objects with one of `description`/`summary`/`result`/`message`/`preview`/`found` (str)                    | that string                                                                       | `"text"`                                                                       | generic                                           | CORRECTED (see below)                 |

### 1.1 Terminal - VERIFIED

`unwrap_hermes_result` requires the content to start with `{` (67-69), parses
once (72), and the terminal branch (80-106) requires both `output` (str) and
`exit_code` present; empty `output` deliberately falls through so a failed
command's `error` string becomes the payload (76-79, F12 comment). The content
type is heuristic: `"exit code:"`/`"Error:"` → `terminal`; cargo/rustc/test
verb lines → `build_error` (on `error[`/`error: could not`) or `build_output`;
otherwise `detect_type` (85-103). Example (captured, `.hermes/examples/previews/model_family-code_first.md:45`):

```json
{
	"output": "<<<CCR:940fbe9416d6fcf921db14c81580309ed8b98351|ls|5889>>>\n[ls:68 files 32 dirs | .json×13 .txt×13 .py×11]",
	"exit_code": 0,
	"error": null
}
```

### 1.2 Patch / write_file - CORRECTED

The paste claimed the family is keyed on `success`/`diff`. The code keys the
**diff path on `diff` alone** (109-113) and the error path on `error` (114-121)

- `success` is never consulted for a diff result. The `error` branch carries a
  hidden grep-hint special case: `msg.starts_with("Found") && msg.contains("matches")`
  → `"text"` (115-117). Real Hermes patch envelopes carry `success: true` plus
  `diff`/`files_modified`/`lint` etc. (Hermes `tools/file_operations_common.py:73-83`)
- the `diff` field is what unwraps them.

### 1.3 Error-only / success - CORRECTED

The paste described an "error-only" family; the code has no standalone
`{"error": ...}`-only arm - errors are handled inside the patch branch (114-121).
The `success` branch (122-139) only fires for a **single-key** object whose
`success` value is a **string** (e.g. `{"success": "wrote 3 files"}`), gated at 135. The ISSUE-11 WS1 comment (122-133) documents that the bare
`{"success": true}` collapse was **deleted**: a success-bool envelope is now a
user payload and falls through.

### 1.4 Search - VERIFIED (+extended)

The paste's `total_count`/`matches`/`truncated` shape is exact. The code also
normalizes `matches_text` (path-grouped Hermes shape, 164-182) into the same
`path:line:content` lines, treats `matches_format` presence as search-shape
evidence (191-194), and - when no per-match rows exist - surfaces the real
total as `"N total"` / `"N total (truncated)"` (195-202). WS2 comment (144-148):
the old `take(20)` cap is gone; every real match feeds the preview count.

### 1.5 Read / content - VERIFIED

`content` (str) → extracted verbatim, typed via `detect_type` (208-212). The
code comment (209) notes read_file is usually essential-skipped. The label-vs-preview
mismatch for a JSON payload read this way is §8 open question 1.

### 1.6 Generic - CORRECTED

The paste said "everything else → classifier or caller hint". The code is
narrower: only **single-key** objects with one of six priority keys
(`description`/`summary`/`result`/`message`/`preview`/`found`) collapse to
`"text"` (221-230, WS1 gate at 222). Everything else returns `None` (232).
The "caller hint" lives one layer up, in `compress_into` (249-269), not in the
unwrap: a non-`text` hint wins over the unwrap guess, and for JSON-shaped
content the hint path previews the **full** content (257-258). No hint /
`text` hint → the unwrap's `(payload, type)` pair is used.

## 2. The onion invariants

Layers: **L0** raw tool payload → **L1** Hermes JSON envelope → **L2** CCR
marker + preview replacing L1 in context → **L3** optional `aphrodite_retrieve`
returning the original L1.

- **One unwrap - CONFIRMED.** `unwrap_hermes_result` parses exactly once
  (`tools.rs:72`); every branch extracts fields from the parsed object, never
  re-parsing or recursively peeling a nested value. A JSON payload inside a
  result stays a payload: the F6 gate (222), the success gate (135), and the
  search-shape gate (191-195) all exist precisely to stop nested/multi-key
  JSON from collapsing to a fragment (WS1 comments 123-133, 217-220).
- **Verbatim storage - CONFIRMED.** `compress_into` always hashes and stores
  the ORIGINAL `content` (`tools.rs:271`, `:275`); the extracted piece is used
  only for classification/preview (comment 244-248: "retrieval must return
  exactly what was passed in, including wrapper metadata like
  `exit_code`/`error`, and legitimate caller JSON that merely matches a wrapper
  shape (e.g. `{"content":"hi","id":42}`) must round-trip intact").
- **Retrieval = original Layer 1 - CONFIRMED for tool-result tools.**
  `aphrodite_retrieve` expands the stored bytes (`resolve::expand`, handler at
  `tools.rs:327-345`) - the full original envelope including `exit_code`/
  `error`/`truncated`/match data.
- **Terminal nuance (CORRECTED vs the paste's blanket claim).** The terminal
  hook fires _before_ Hermes assembles the envelope: Hermes calls
  `transform_terminal_output` at `~/.hermes/hermes-agent/tools/terminal_tool_result.py:144`
  and only builds `{"output","exit_code","error"}` afterwards (:258). So for
  terminal, the stored/retrieved L1 is the **raw output string**, not the JSON
  envelope (the envelope the model sees has `output` replaced by the marker,
  cf. the ls capture in §1.1). For non-terminal tools the hook
  (`transform_tool_result`, `model_tools.py:854`) compresses the JSON envelope
  and replaces the whole result; retrieval returns that envelope verbatim
  (captured diff: `.hermes/examples/templates/diff-preview-code_first.md:56`).
- **WS1 / WS2 placement.** WS1 = the deleted success-bool collapse
  (`tools.rs:122-133`), the single-key gates (135, 222), the caller-hint-wins
  rule (249-269), and full-content-for-JSON previews (257-258). WS2 = the
  search preview lines (144-205), honest error/warning line tallies
  (`preview.rs:744-789`), the terminal arm's first-meaningful-line
  (895-909), and the error arm's first-error-line (917-928).

## 3. The residual + the principled rule

**The `success`-keyed JSON case - paste claim VERIFIED.** Trace of
`{"success": true, "data": [...]}`:

1. Unwrap: `success` is a bool → `as_str()` is `None` → no return
   (`tools.rs:122-139`); multi-key → F6 gate (222) skips → `None` (232).
2. `detect_type` (headroom) → a JSON **object** is not `json_array`
   (`vendor/headroom/crates/headroom-core/src/transforms/content_detector.rs:272-283`,
   `parsed.as_array()?`; test `json_object_falls_through_to_text` :570) →
   `"text"`.
3. `build_preview("text", …)` upgrades via `detect_semantic_type`
   (`preview.rs:733-734`) → the JSON arm (56-66) → **excluded** because
   `is_envelope_json_object` (255-275) hits the `success` guard key →
   stays `text`.

The exclusion list (`preview.rs:256-273`, quoted):

```rust
const GUARD:&[&str] = &[
    "output", "exit_code", "diff", "error", "success", "total_count",
    "matches", "matches_text", "content", "total_lines", "result",
    "message", "found", "preview", "name", "description",
];
```

Note the asymmetry: `{"data": [...]}` or `{"items": [...]}` alone (no guard
key) → `json`; any object carrying one of the 16 guard keys is treated as an
envelope even when the unwrap did not extract it.

**The principled rule:** unwrap only the exact envelope schemas
`unwrap_hermes_result` actually supports; classify every other valid JSON as
`json`; keep provenance separate from content type (`source = tool_result`,
`content_type = json`). That separation is §5.

## 4. Hermes-side verification - where each envelope shape is produced

Sources: `~/.hermes/hermes-agent/tools/*` (authoritative Hermes source).
Compiled by a delegated research pass that read each file; the integration
files (`lib.rs:409-457`, `hooks.rs:161-198`) were re-verified directly.

| Family                     | Hermes producer (file:line)                                    | Envelope keys                                                                    | Aphrodite unwrap             |
| -------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------- |
| terminal                   | `terminal_tool_result.py:258` (+`_result` wrapper 261-277)     | `output`, `exit_code`, `error` + `cwd`, `full_output_path`, `truncation_note`, … | F1 ✓                         |
| patch                      | `file_tools.py:952-985`; `file_operations_common.py:73-83`     | `success`, `diff`, `files_modified`, `lint`, `error`, …                          | F2 ✓ (via `diff`/`error`)    |
| write_file                 | `file_tools.py:811-868`; `file_operations_common.py:48-49`     | `bytes_written`, `dirs_created`, `verified`, `lint`, `error`, `warning`          | GAP on success; error → F2 ✓ |
| read_file                  | `file_tools.py:650-704`; `file_operations_common.py:29-30`     | `content`, `total_lines`, `file_size`, `truncated`, `hint`, `is_binary`, `error` | F5 ✓                         |
| search_files               | `file_tools.py:1047-1069`; `file_operations_common.py:127-153` | `total_count`, `matches` / `matches_format`+`matches_text`, `truncated`, `files` | F4 ✓                         |
| error envelope (all tools) | `registry.py:999-1002`                                         | `error` (str) + extras                                                           | F2 error ✓                   |

**Recognized-but-nuanced:** `execute_code` local (`code_kernel.py:727-755`,
`status/output/exit_code`) is F1 only when `output` is non-empty; the remote
variant (`code_execution_tool.py:521-526`) ships **no `exit_code`** on success
→ GAP. `process` exited (`process_registry.py:1868-1870`,
`status/command/exit_code/output`) is F1; its timeout shape (:1849-1862) and
its top-level-array list form (:2374) are GAPs.

**Gap list - Hermes envelopes the unwrap does NOT recognize** (each falls to
`detect_type` + semantic upgrade; no envelope fields extracted):

1. write_file success: `bytes_written/dirs_created/verified/lint/lsp_diagnostics/warning`
2. skill_view: `success/name/description/tags/related_skills/content/path/…` (`skills_tool.py:631-644`) - multi-key blocks F5/F6
3. skills_list: `success/skills/categories/message` (`skills_tool.py:248-255`)
4. todo_list: `todos/revision/summary{…}` (`todo_tool.py:210-211`)
5. memory writes: `success/staged/pending_id/message/proposal_staged` (`memory_tool.py:77,159-161`)
6. cronjob: `success/forwarded_to_gateway/note` + job payloads (`cronjob_tools.py:143-157`)
7. browser tools success envelopes: `success+url/title/snapshot/element_count/typed/element/result/analysis/data/…` (`browser_tool.py:768,817,893,1013,1200,1256`)
8. web_search success: `success/data.web[…]` (`web_tools.py:305-311`)
9. web_extract: `results` (array; single-key but `results` ∉ F6 keys) (`web_tools.py:396-399`)
10. image_generate success: `success/image/modality/upscaled` (`image_generation_tool.py:472-476`)
11. execute_code remote success: `status/output` without `exit_code`
12. process_manage timeout / list (see above)
13. delegate_task: `results[{status,summary,…}], total_duration_seconds` (`delegate_tool_dispatch.py:185-196`)
14. session_search success: `success/mode/query/detail/results/count/message` (`session_search_tool.py:310,446,493`)
15. desktop_project: `success/id/slug/name` (`project_tools.py:62-63` - UNVERIFIED, file not independently re-read)
16. annotate_preview fallback: `{"text": …}` (`annotate_preview_tool.py:44-46`)
17. vision_analyze success: `success/analysis/scale_note` (`vision_tools.py:752`)
18. read_file dedup stub: `status/message/path/dedup/content_returned` (`file_tools.py:507-513`)

CCR routing: both hooks are registered in `plugins/aphrodite/__init__.py:1143-1182`;
`transform_tool_result` fires at `model_tools.py:854` and its first string
return **replaces** the tool result (:857). No literal `"tool_result"` hint is
passed to the engine - the marker `type` is content-derived (`hooks.rs:170-191`;
semantic upgrade 182-189; `"terminal"` forced for `exit code:`/`Error:` output
at `hooks.rs:379-381`), with the unwrap's `(payload, type)` threaded as
`classify` (`lib.rs:422,437,443,452`).

## 5. The separation proposal - `envelope_kind` / `content_type` (design)

**Problem:** one string (`type`) today carries content semantics only; the
envelope provenance (which Hermes family produced the result) is dropped after
unwrap, so `{"success": true, "data": …}` is indistinguishable from a plain
user JSON object, and a read_file'd JSON file is labeled `text` while its
preview renders `json` (see §8).

**Design:**

- `envelope_kind` ∈ {`terminal`, `patch`, `error`, `search`, `content`,
  `generic`} - **provenance**: how the result arrived (the six unwrap
  families). `None`/absent = no envelope recognized.
- `content_type` - **what the payload IS**: `json`, `text`, `diff`,
  `terminal`, `build_output`, `code_*`, `search`, … (the full classifier set -
  catalog doc §2).
- `unwrap_hermes_result` returns `(envelope_kind, payload, content_type)`:
  the kind is structural (which branch fired), the content type is
  classification (`detect_type` + semantic upgrade, existing heuristics
  unchanged).
- `compress_into` records both on the `MarkerEntry` (`tools.rs:276-284`) and
  the compress response (`286-292`); the marker/catalog carries `kind` in
  metadata (`marker.rs:67-82` meta key `kind=…`) while the marker line stays
  `<<<CCR:hash|type|size>>>` for backward compatibility.
- **Change points:** `tools.rs` (unwrap return type + every branch's return),
  `compress_into` (thread kind through), `state.rs` `MarkerEntry` (+`kind`
  field), `marker.rs` `ccr_marker` (+kind param → meta), `catalog_mode`
  rendering, `templates/aphrodite.toml` (document `{kind}` var), proxy parity
  (`proxy.rs:1329-1339` `tool_output` is the proxy's pre-unwrap kind guess -
  align on `envelope_kind`), and the tests below.
- **Explicitly NOT proposed:** changing the unwrap branch set itself (F1-F6
  stay as verified), or the marker line format (hash/type/size triple is the
  stable contract).

## 6. Minimal implementation plan

Ordered so a follow-up agent can execute each step and gate on the previous:

1. **Unwrap returns a struct** - `crates/aphrodite-hermes/src/tools.rs:67-233`.
   Change `Option<(String, String)>` → `Option<Envelope { kind: EnvelopeKind, payload: String, content_type: String }>`;
   each branch fills `kind` (`terminal`/`patch`/`error`/`search`/`content`/`generic`).
   Callers: `compress_into` (`tools.rs:249-269`) and `lib.rs:422,443` adapt.
2. **Thread kind through compression** - `compress_into` (`tools.rs:241-293`):
   add `kind: Option<EnvelopeKind>` to the `MarkerEntry` (`state.rs`) and to
   the response JSON (`286-292`).
3. **Marker/catalog carries kind** - `marker.rs:39-47` `ccr_marker` gains a
   kind param emitted as `kind=…` in the meta line (`marker.rs:67-82`); the
   marker line format unchanged. Catalog rendering (`catalog_mode`) shows the
   kind column.
4. **`detect_type` untouched; semantic detector untouched** - content_type
   stays exactly as verified (no behavior change in `preview.rs`).
5. **Proxy parity** - `proxy.rs:1325-1339`: `tool_output`/`json` split becomes
   `envelope_kind=tool_output` + `content_type=json` so both paths emit the
   same two-axis record.
6. **Tests** - `preview.rs` test module (:1493+) and `tools.rs` tests: assert
   each F1-F6 branch returns the right kind+payload+type; add the residual
   case `{"success": true, "data": [...]}` → kind `None`, content_type `text`
   today / `json` under the principled rule; round-trip test that kind survives
   compress→marker→catalog→retrieve.
7. **Gate** - `cargo test -p aphrodite -p aphrodite-hermes` green; `npx prettier --check` on touched `.md`.

## 7. Open questions

1. **Label-vs-preview mismatch (read_file→JSON).** `tools.rs:210-211` types a
   JSON payload via headroom `detect_type` → `"text"`, while `build_preview`
   upgrades it to `json` (733-734). The marker `type` field says `text`, the
   preview renders `[json:…]`. The `type` field therefore already conflates
   envelope and content - the exact conflation §5 removes.
2. **Proxy vs hook classifier divergence.** `proxy_detect_content_type`
   returns `tool_output` for `{exit_code|status}` envelopes (`proxy.rs:1335-1337`)
   while the hook path unwraps them; a JSON envelope compressed via the cache
   proxy and one compressed via the hook can get different `type` labels today.
3. **`matches_format` values** are only used as shape evidence (`tools.rs:193`),
   never parsed - Hermes ships `"path-grouped"`; other values (if any) would
   silently fall to the `"N total"` branch. UNVERIFIED whether Hermes emits
   other formats.
4. **`{"success": …}` single-key vs multi-key asymmetry** - the unwrap honors
   only single-key success-strings, but the semantic GUARD list excludes any
   object carrying `success` from `json` (255-275). A principled `json`
   classification for unwrap-rejected objects (§3 rule) requires the GUARD to
   shrink to exactly the keys the unwrap actually extracts.
5. **Terminal envelope storage** - retrieval returns the raw output, not
   `{"output","exit_code","error"}`; should the terminal envelope (with
   `exit_code`) be the stored L1 for terminal too? Current design: no (hook
   ordering, §2).
