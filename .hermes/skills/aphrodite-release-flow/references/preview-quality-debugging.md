# Preview-Quality Debugging (what the LLM sees)

The CCR preview string is the ONLY thing the model sees inline - if it
misleads, the agent concludes the tool returned nothing and starts a false
trail ("the search backend is broken"). User principle (stated emphatically):
**the preview the LLM sees must be the final, most in-depth, honest
representation of the STORED payload** - same L/B counts, same type, same
shape as the stored object; never a fragment, never a guess.

## The defect class: envelope-unwrap false positives

`unwrap_hermes_result` (crates/aphrodite-hermes/src/tools.rs ~81-178) is a
heuristic written for the HOOK path - Hermes wraps tool results in envelopes
(`{output,exit_code}`, `{total_count,matches_text}`, `{content,total_lines}`)
and the hook needs unwrapping to preview terminal/search/read results instead
of `[json:...]`. The bug class: the same heuristic runs on EVERY explicit
`aphrodite_compress` call (`compress_into`, tools.rs:193), where there is no
envelope - only the caller's type hint. Any JSON object matching a wrapper
shape gets type + preview rewritten:

| Branch | Trigger | Result | User-visible symptom |
|---|---|---|---|
| success-bool | `success:true` + <=2 keys | literal `"ok"`, type `text` | `[text:1L 2B | ok]` for a 737-byte payload - the issue-#11 shape |
| success-string | `success` is a non-empty string | collapse to that string | real data dropped |
| priority keys | `result`/`message`/`description`/`summary`/`preview`/`found` at ANY size | collapse to one string, type `text` | same `ok` symptom at any object size |
| content | `content` string field | inner fragment, type from fragment | nested data hidden |
| output+exit_code | `output` + `exit_code` keys | terminal/build classification | non-terminal JSON retyped |
| total_count | `total_count` key | fabricated grep lines or `N total`, type `search` | LLM concludes search found nothing (real search_files shape: `{total_count,matches_text}`) |
| diff | `diff` string | fragment, type from diff | rest of object hidden |
| error | `error` string | message only | code/retry_after invisible |

Consequences shared by ALL branches: the caller's explicit hint is discarded
(type flips even when the caller passed `type:"tool_result"`), the preview's
L/B counts describe the FRAGMENT while the marker's size field describes the
payload (lies about shape), and type corruption breaks `aphrodite_search`
type filters. Storage is always lossless (hash/store use the original on
both paths) - which is why the bug always manifests as "preview broken,
content retrieves fine". The hint-poisoning variant is broader than the
`ok` collapse: passing `type=tool_result` retypes EVERY payload to literal
`tool_result` with a first-line preview, killing semantic typing on the
explicit path entirely.

## Empirical battery methodology (verify before believing any preview claim)

Probe the release dylib directly via ctypes - no Hermes session needed:
`ctypes.CDLL("target/release/libaphrodite_hermes.dylib")`, set
`aphrodite_init.argtypes`/`aphrodite_hermes_dispatch_tool.argtypes`
(both `[c_char_p, c_char_p]`, restype `c_void_p`, free via
`aphrodite_hermes_free_string`), call `aphrodite_compress` with
`{"content": <payload>, "type": "tool_result"}` and print the FULL returned
dict (hash/type/size/preview/marker). Run ~40 payloads spanning every major
type + edge cases (empty, whitespace, huge single line, binary-ish, nested
JSON, envelope-shaped objects) × hints (`tool_result`/`text`/`terminal`/
no-hint). ALSO test the production hook path
(`transform_tool_result`/`transform_terminal_output` via
`aphrodite_hermes_call_hook`) - that is what the LLM actually sees, and it
scores differently from the explicit path. Verdict classes per payload:
OK (informative + truthful) / MISLEADING (preview content wrong) /
SHALLOW (preview exists but hides the real shape) / TYPE-WRONG (marker
type != content). Score both paths; both were ~55-65% defective in the
first battery, with test-result wrappers showing `[build:0E 0W 3L]` even
when tests FAILED.

## Fix shape (validated against the pinned test matrix)

- Hint-wins in `compress_into`: when the caller passed a non-default hint
  and the content starts with `{`, build type + preview from the FULL
  content (`(content, hint)`), not the unwrap fragment - zero pinned tests
  break; restores the "trust the type hint" schema contract.
- Drop/guard the success-bool collapse (require a known envelope key such
  as `exit_code`/`total_count`/`matches`/`diff` before unwrapping) - closes
  the same bug on the hook path where no hint exists to disambiguate.
- Surface the REAL `total_count` + truncated flag instead of
  sample-20-as-total.
- No code parses the preview body (`resolve.rs` reads only the
  `<<<CCR:hash|type|size>>>` header); the preview is inert LLM-facing text,
  so longer honest JSON previews are strictly safer - never keep a bug
  "because something might parse the short form".

## Dead config levers (verify with grep before trusting any preview knob)

`model_family`, `code_structure_map`, `preview_max_chars`,
`rust_preview_lines` are declared in `PreviewsConfig` and shipped in
`templates/aphrodite.toml` + `aphrodite.toml.example` but READ NOWHERE in
the crate tree - there is no preview-template system; previews come from
`build_preview` (shared) and `proxy_build_preview` (a parallel, weaker,
and for JSON outright-lying builder that crude-counts `":` occurrences).
A user reporting "config levers have no effect" is the tell for dead
config, not a misreading.

## History lesson (bug birth)

The `success:true -> "ok"` collapse was ADDED in the commit that moved the
unwrap from the core crate (where it was gated on `classify_type.starts_with('json')`
and hook-only) into the bridge, dropping the gate - so a fix that only
touches `compress_into` leaves the hook-path variant alive. Trace a preview
bug's introduction with `git log -S` for the collapse string and grep each
release tag's `tools.rs`; the bug was present in every release from the
first one that carried the move.
