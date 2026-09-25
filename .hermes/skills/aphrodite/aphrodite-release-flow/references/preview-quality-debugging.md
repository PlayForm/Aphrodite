# Preview-Quality Debugging (decision-tree runbook)

The CCR preview string is the ONLY thing the model sees inline - if it
misleads, the agent concludes the tool returned nothing and starts a false
trail ("the search backend is broken"). User principle (stated emphatically):
**the preview the LLM sees must be the final, most in-depth, honest
representation of the STORED payload** - same L/B counts, same type, same
shape as the stored object; never a fragment, never a guess.

## Step 0 - Classify the symptom BEFORE suggesting implementation changes

A "preview is wrong" report is five different bugs. Run the discriminating
test for each class before touching preview code:

| Class                      | Discriminating test                                                                                                 | If true → go to |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------- |
| **C1 Preview corruption**  | Compare the preview's type/L/B counts against the marker's size field and the STORED payload (ctypes battery below) | C1              |
| **C2 Preview truncation**  | Is the preview merely cut short, with honest counts (expected cap), or does it LIE about size/shape (bug)?          | C2              |
| **C3 Transport failure**   | Does the marker resolve through the canonical retrieval route? If retrieval fails, the preview is not the defect    | C3              |
| **C4 Configuration drift** | Does the user report "config levers have no effect"? Dead-config tell → C4                                          | C4              |
| **C5 Stale process**       | Is the loaded dylib version older than the build under test? Restart first, then re-probe                           | C5              |

## C1 - Preview corruption (content/type lies about the stored payload)

**Evidence to collect**

- The full returned dict from `aphrodite_compress`
  (`{hash,type,size,preview,marker}`) for a known payload + hint.
- Marker size field vs preview L/B counts (they disagree = lying about
  shape).

**Bounded likely causes (discriminating test → safe repair)**

1. **Envelope-unwrap heuristic on the explicit path** - `unwrap_hermes_result`
   (crates/aphrodite-hermes/src/tools.rs) was written for the HOOK path
   (Hermes wraps tool results in `{output,exit_code}`,
   `{total_count,matches_text}`, `{content,total_lines}` envelopes) but runs
   on EVERY explicit `aphrodite_compress` call, where there is no envelope -
   only the caller's type hint. Any JSON object matching a wrapper shape gets
   type + preview rewritten (the `success:true → "ok"` collapse is the
   issue-#11 shape: `[text:1L 2B | ok]` for a 737-byte payload). Discriminate:
   compress the same payload with a `type:"tool_result"` hint and a `text`
   hint; a type flip = this class.
2. **Hook-path variant** - the same heuristic on `transform_tool_result` /
   `transform_terminal_output` (what the LLM actually sees), where no hint
   exists to disambiguate. A fix that only touches `compress_into` leaves
   this variant alive (the bug was ADDED by moving the unwrap from the core
   crate, where it was gated on `classify_type.starts_with('json')` and
   hook-only, into the bridge, dropping the gate).

**Safe repair (validated against the pinned test matrix)**

- **Hint-wins in `compress_into`**: when the caller passed a non-default
  hint and the content starts with `{`, build type + preview from the FULL
  content (`(content, hint)`), not the unwrap fragment - zero pinned tests
  break; restores the "trust the type hint" schema contract.
- **Drop/guard the success-bool collapse**: require a known envelope key
  (`exit_code`/`total_count`/`matches`/`diff`) before unwrapping - closes
  the same bug on the hook path where no hint exists.
- **Surface the REAL `total_count` + truncated flag** instead of
  sample-20-as-total.
- No code parses the preview body (`resolve.rs` reads only the
  `<<<CCR:hash|type|size>>>` header); the preview is inert LLM-facing text,
  so longer honest JSON previews are strictly safer - never keep a bug
  "because something might parse the short form".

**Exit criteria**

- Battery verdicts for both paths are OK (informative + truthful) across the
  fixture matrix; no type flips; marker size == payload size; preview L/B
  counts describe the FULL payload.

**Prohibited**

- Treating a misleading preview as cosmetic; fixing only the explicit path
  while the hook path stays broken; editing `_bindings.py` by hand.

## C2 - Preview truncation (cut short, honest vs lying)

**Evidence to collect**

- Live `preview_max_chars` value: env `APHRODITE_PREVIEW_MAX_CHARS` > TOML >
  default 120; absent key = unlimited (read the active config, never a
  literal).
- Preview byte length vs the configured cap; L/B counts in the preview vs
  the payload's real shape.

**Bounded likely causes (discriminating test → safe repair)**

1. Expected cap - preview honestly cut at the configured maximum with correct
   counts → adjust the cap deliberately or accept it; NOT a bug.
2. Lying truncation - counts/size claim the FULL payload while the preview
   shows a fragment (a lie about shape) → the corruption classes (C1)
   overlap here; fix the preview builder.

**Exit criteria**

- Truncation is either an explicit, documented cap or the counts are honest;
  the model can decide whether to retrieve.

**Prohibited**

- Raising the cap to hide a lying-count bug; assuming the default 120 without
  reading the active config.

## C3 - Transport failure (retrieval path, not preview code)

**Evidence to collect**

- Does the marker resolve via the canonical retrieval route? Is the session
  inline-only (engine runbook S2 - session-scoped entries, `ccr.db` 0 bytes)?

**Bounded likely causes (discriminating test → safe repair)**

1. Inline-only mode - entries vanish on restart; expected degraded mode, not
   a preview defect.
2. Retrieval/resolver failure - the marker does not resolve, or a malformed
   marker returns a non-diagnostic error → engine-health runbook S4.

**Exit criteria**

- Retrieval returns the normalized source, or the failure is classified as
  transport/inline, not preview.

**Prohibited**

- Re-architecting preview code because retrieval is down; re-compressing a
  retrieval response (marker-resolution loop).

## C4 - Configuration drift (dead config levers)

**Evidence to collect**

- Grep the crate tree for reads of the reported knob; the shipped
  `templates/aphrodite.toml` + `aphrodite.toml.example` entries; user report
  "config levers have no effect".

**Bounded likely causes (discriminating test → safe repair)**

1. **Dead config**: `model_family`, `code_structure_map`, `preview_max_chars`,
   `rust_preview_lines` are declared in `PreviewsConfig` and shipped in the
   templates but READ NOWHERE in the crate tree - there is no
   preview-template system; previews come from `build_preview` (shared) and
   `proxy_build_preview` (a parallel, weaker, and for JSON outright-lying
   builder that crude-counts `:` occurrences). A user reporting "config
   levers have no effect" is the tell for dead config, not a misreading.

**Safe repair**

- Fix the preview builder (C1 shape), not the config plumbing; document the
  levers as inert or wire them with a real consumer.

**Exit criteria**

- Behavior matches the documented consumer status of each lever (active /
  inactive / removed).

**Prohibited**

- Adding a config field to "fix" a preview bug (anti-feature record pattern);
  trusting a knob's effect without a grep-verified consumer.

## C5 - Stale process (old dylib still loaded)

**Evidence to collect**

- Loaded dylib version (`aphrodite_stats`) vs the build under test;
  whether Hermes was restarted after the change.

**Bounded likely causes (discriminating test → safe repair)**

1. The new dylib only takes effect on the next load - the live session holds
   the OLD dylib until restart → restart, re-probe.

**Exit criteria**

- The fresh process exhibits the new preview behavior.

**Prohibited**

- Concluding "the fix does not work" from a stale process.

## The defect class: envelope-unwrap false positives

`unwrap_hermes_result` (crates/aphrodite-hermes/src/tools.rs) is a heuristic
written for the HOOK path - Hermes wraps tool results in envelopes
(`{output,exit_code}`, `{total_count,matches_text}`, `{content,total_lines}`)
and the hook needs unwrapping to preview terminal/search/read results instead
of `[json:...]`. The bug class: the same heuristic runs on EVERY explicit
`aphrodite_compress` call (`compress_into`), where there is no envelope -
only the caller's type hint. Any JSON object matching a wrapper shape gets
type + preview rewritten:

| Branch           | Trigger                                                                  | Result                                            | User-visible symptom                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------ | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| success-bool     | `success:true` + <=2 keys                                                | literal `"ok"`, type `text`                       | `[text:1L 2B                                                                               \| ok]` for a 737-byte payload - the issue-#11 shape |
| success-string   | `success` is a non-empty string                                          | collapse to that string                           | real data dropped                                                                                                                               |
| priority keys    | `result`/`message`/`description`/`summary`/`preview`/`found` at ANY size | collapse to one string, type `text`               | same `ok` symptom at any object size                                                                                                            |
| content          | `content` string field                                                   | inner fragment, type from fragment                | nested data hidden                                                                                                                              |
| output+exit_code | `output` + `exit_code` keys                                              | terminal/build classification                     | non-terminal JSON retyped                                                                                                                       |
| total_count      | `total_count` key                                                        | fabricated grep lines or `N total`, type `search` | LLM concludes search found nothing (real search_files shape: `{total_count,matches_text}`)                                                      |
| diff             | `diff` string                                                            | fragment, type from diff                          | rest of object hidden                                                                                                                           |
| error            | `error` string                                                           | message only                                      | code/retry_after invisible                                                                                                                      |

Consequences shared by ALL branches: the caller's explicit hint is discarded
(type flips even when the caller passed `type:"tool_result"`), the preview's
L/B counts describe the FRAGMENT while the marker's size field describes the
payload (lies about shape), and type corruption breaks `aphrodite_search`
type filters. Storage is always lossless (hash/store use the original on
both paths) - which is why the bug always manifests as "preview broken,
content retrieves fine". The hint-poisoning variant is broader than the `ok`
collapse: passing `type=tool_result` retypes EVERY payload to literal
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

## History lesson (bug birth)

The `success:true -> "ok"` collapse was ADDED in the commit that moved the
unwrap from the core crate (where it was gated on
`classify_type.starts_with('json')` and hook-only) into the bridge, dropping
the gate - so a fix that only touches `compress_into` leaves the hook-path
variant alive. Trace a preview bug's introduction with `git log -S` for the
collapse string and grep each release tag's `tools.rs`; the bug was present
in every release from the first one that carried the move.
