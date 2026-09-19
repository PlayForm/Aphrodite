# Aphrodite Example Catalog - Coverage Matrix (Authoritative)

Maintained by AGENT B (cataloger). Source of truth for schema coverage: `crates/aphrodite/templates/aphrodite.toml` + `docs/config/env-vars.md`.
Every row = one captured variant. `UNCAPTURED <reason>` = capture failed twice, do not re-request.
Exchange protocol: requests flow via `.hermes/tmp/example-exchange/notes.md` (B → A `REQUEST <id>`, A → B `DONE <id>: <file>`).

Last updated: 2026-09-19 (mission 1.4.6 - **reconciled by BATCH 4 against the 69 example files on disk**). Rows whose file exists on disk are marked CAPTURED; rows pointing at files that don't exist were fixed to the real file or left UNCAPTURED.

---

## 1. proxies - `[[proxies]]` (name, listen, mode, tool_relay, timeout)

| Key         | Variant                                                        | File                        | Status                    |
| ----------- | -------------------------------------------------------------- | --------------------------- | ------------------------- |
| cache proxy | default (name=cache, mode=cache, tool_relay=true, timeout=120) | proxies/cache-default.md    | CAPTURED (T26)            |
| token proxy | default (name=token, mode=token, tool_relay=true, timeout=300) | proxies/cache-default.md    | CAPTURED (T26, joint - same file documents both proxies) |
| tool_relay  | true                                                           | proxies/cache-default.md    | CAPTURED (T26, joint - shipped value documented in file) |
| tool_relay  | false                                                          | proxies/tool_relay-false.md | UNCAPTURED (not hot-reloadable - proxy restart required; documented in cache-default.md) |
| timeout     | 120 (cache) / 300 (token)                                      | proxies/cache-default.md    | CAPTURED (T26, joint)     |

## 2. defaults - `[defaults]`

| Key             | Variant                      | File                             | Status                    |
| --------------- | ---------------------------- | -------------------------------- | ------------------------- |
| ccr_ttl_seconds | 3600 (default)               | defaults/ccr_ttl_seconds-3600.md | CAPTURED (T24)            |
| ccr_ttl_seconds | 60 (TOML variant)            | defaults/ccr_ttl_seconds-60.md   | UNCAPTURED (env override form captured instead: env-vars/CCR_TTL_SECONDS-60.md, E06) |
| api_url         | env-only (APHRODITE_API_URL) | env-vars/api_url-env.md          | UNCAPTURED (not in BATCH 4 scope) |
| model           | env-only (APHRODITE_MODEL)   | env-vars/model-env.md            | UNCAPTURED (not in BATCH 4 scope) |

## 3. compression - `[compression]`

| Key                  | Variant          | File                                     | Status                    |
| -------------------- | ---------------- | ---------------------------------------- | ------------------------- |
| engine_threshold_pct | 45 (default)     | compression/engine_threshold_pct-45.md   | CAPTURED (C01)            |
| engine_threshold_pct | 100 (disabled)   | compression/engine_threshold_pct-100.md  | CAPTURED (C02)            |
| engine_threshold_pct | 20 (aggressive)  | compression/engine_threshold_pct-20.md   | CAPTURED (C03)            |
| engine_protect_first | 2 (default)      | compression/engine_protect_first-2.md    | CAPTURED (T14)            |
| engine_protect_first | 0                | compression/engine_protect_first-0.md    | UNCAPTURED                |
| engine_protect_last  | 5 (default)      | compression/engine_protect_last-5.md     | CAPTURED (T16)            |
| engine_protect_last  | 0                | compression/engine_protect_last-0.md     | UNCAPTURED                |
| engine_min_msgs      | 8 (default)      | compression/engine_min_msgs-8.md         | CAPTURED (T18)            |
| engine_min_msgs      | 2                | compression/engine_min_msgs-2.md         | UNCAPTURED                |
| tool_threshold_token | 512 (default)    | compression/tool_threshold_token-512.md  | CAPTURED (C09)            |
| tool_threshold_token | 4096             | compression/tool_threshold_token-4096.md | CAPTURED (C10)            |
| tool_threshold_cache | 4096 (default)   | compression/tool_threshold_cache-4096.md | CAPTURED (T22)            |
| tool_threshold_cache | 512              | compression/tool_threshold_cache-512.md  | UNCAPTURED                |
| terminal_threshold   | 512              | compression/terminal_threshold-512.md    | CAPTURED (C04)            |
| terminal_threshold   | 1024 (default)   | compression/terminal_threshold-1024.md   | CAPTURED (C05)            |
| terminal_threshold   | 4096             | compression/terminal_threshold-4096.md   | CAPTURED (C06)            |
| inline_threshold     | 2048 (default)   | compression/inline_threshold-2048.md     | CAPTURED (C07)            |
| inline_threshold     | 512              | compression/inline_threshold-512.md      | CAPTURED (C08)            |
| auto_expand          | true (default)   | compression/auto_expand-true.md          | CAPTURED (C11)            |
| auto_expand          | false            | compression/auto_expand-false.md         | CAPTURED (C12)            |
| auto_expand_limit    | 102400 (default) | compression/auto_expand_limit-102400.md  | CAPTURED (T20)            |
| auto_expand_limit    | 0                | compression/auto_expand_limit-0.md       | UNCAPTURED                |
| catalog_mode         | tool (default)   | compression/catalog_mode-tool.md         | CAPTURED (C13)            |
| catalog_mode         | compact          | compression/catalog_mode-compact.md      | CAPTURED (C14)            |
| catalog_mode         | full             | compression/catalog_mode-full.md         | CAPTURED (C15)            |
| classifier_poll      | true (default)   | compression/classifier_poll-true.md      | CAPTURED (C16)            |
| classifier_poll      | false            | compression/classifier_poll-false.md     | CAPTURED (C17)            |
| code_multiplier      | 3.0 (default)    | compression/code_multiplier-3.0.md       | CAPTURED (C18)            |
| code_multiplier      | 1.0              | compression/code_multiplier-1.0.md       | CAPTURED (C19)            |
| context_engine       | true (default)   | compression/context_engine-true.md       | CAPTURED (T12)            |
| context_engine       | false            | compression/context_engine-false.md      | UNCAPTURED                |
| prefetch             | true (default)   | compression/prefetch-true.md             | CAPTURED (T08)            |
| prefetch             | false            | compression/prefetch-false.md            | CAPTURED (T09)            |
| poll_worker          | true (default)   | compression/poll_worker-true.md          | CAPTURED (T10)            |
| poll_worker          | false            | compression/poll_worker-false.md         | CAPTURED (T11)            |
| chain_split          | false (default)  | compression/chain_split-false.md         | CAPTURED (T06)            |
| chain_split          | true             | compression/chain_split-true.md          | CAPTURED (T07)            |

## 4. previews - `[previews]`

| Key                | Variant         | File                                     | Status                    |
| ------------------ | --------------- | ---------------------------------------- | ------------------------- |
| model_family       | compact         | previews/model_family-compact.md         | CAPTURED (P01/P02 - ls + code + diff + build blocks in one file) |
| model_family       | compact (ls)    | previews/model_family-compact.md         | CAPTURED (P02, joint - ls block in same file) |
| model_family       | code_first      | previews/model_family-code_first.md      | CAPTURED (P03/P04)        |
| model_family       | code_first (ls) | previews/model_family-code_first.md      | CAPTURED (P04, joint)     |
| model_family       | balance         | previews/model_family-balance.md         | CAPTURED (P05/P06)        |
| model_family       | balance (ls)    | previews/model_family-balance.md         | CAPTURED (P06, joint)     |
| code_structure_map | true (default)  | previews/code_structure_map-true.md      | CAPTURED (P07)            |
| code_structure_map | false           | previews/code_structure_map-false.md     | CAPTURED (P08)            |
| preview_max_chars  | 120 (default)   | previews/preview_max_chars-120.md        | CAPTURED (P09)            |
| preview_max_chars  | 0               | previews/preview_max_chars-0.md          | CAPTURED (P10)            |

## 5. prompts - `[prompts]`

| Key                  | Variant                | File                                  | Status                    |
| -------------------- | ---------------------- | ------------------------------------- | ------------------------- |
| retrieve_guidance    | minimal (default)      | prompts/retrieve_guidance-minimal.md  | CAPTURED (M01)            |
| retrieve_guidance    | standard               | prompts/retrieve_guidance-standard.md | CAPTURED (M02)            |
| retrieve_guidance    | verbose                | prompts/retrieve_guidance-verbose.md  | CAPTURED (M03)            |
| ccr_marker_hint      | false (default)        | prompts/retrieve_guidance-minimal.md  | CAPTURED (M01, joint - folded into the minimal capture; no separate file) |
| ccr_marker_hint      | true                   | prompts/retrieve_guidance-verbose.md  | CAPTURED (M03, joint - folded into the verbose capture) |
| catalog_intent_hints | false (default)        | prompts/catalog_intent_hints-false.md | CAPTURED (M07)            |
| catalog_intent_hints | true                   | prompts/catalog_intent_hints-true.md  | CAPTURED (M06)            |
| session_inject       | default (shipped text) | prompts/session_inject-empty.md       | CAPTURED (M08, joint - default orientation active in every session; noted in session_inject-empty.md) |
| session_inject       | "" (disabled)          | prompts/session_inject-empty.md       | CAPTURED (M09)            |

## 6. templates - `[templates.preview.{compact,code_first,balance}]` rendered previews

Family-specific RENDERED preview strings for the core content types.

| Family     | Type         | File                                         | Status                    |
| ---------- | ------------ | -------------------------------------------- | ------------------------- |
| compact    | diff         | templates/diff-preview-compact.md            | CAPTURED (T29 - BATCH 4 real diff marker) |
| compact    | build_output | templates/preview-compact-build_output.md    | UNCAPTURED                |
| compact    | terminal     | templates/preview-compact-terminal.md        | UNCAPTURED                |
| compact    | json         | templates/preview-compact-json.md            | UNCAPTURED                |
| code_first | diff         | templates/diff-preview-code_first.md         | CAPTURED (T30 - BATCH 4 real diff marker) |
| code_first | build_output | templates/preview-code_first-build_output.md | UNCAPTURED (build_error type captured instead: templates/build_error-preview-code_first.md) |
| code_first | terminal     | templates/preview-code_first-terminal.md     | UNCAPTURED                |
| code_first | json         | templates/json-preview-code_first.md         | CAPTURED (T30 - BATCH 4 real json marker, 1728B payload) |
| code_first | build_error  | templates/build_error-preview-code_first.md  | CAPTURED (BATCH 4 - real rustc E0308 marker; added row) |
| balance    | diff         | templates/preview-balance-diff.md            | UNCAPTURED                |
| balance    | build_output | templates/preview-balance-build_output.md    | UNCAPTURED                |
| balance    | terminal     | templates/preview-balance-terminal.md        | UNCAPTURED                |
| balance    | json         | templates/preview-balance-json.md            | UNCAPTURED                |

### 7. templates - `[templates.marker]`

| Key    | Variant                   | File                               | Status                    |
| ------ | ------------------------- | ---------------------------------- | ------------------------- |
| format | default                   | templates/marker-format-default.md | CAPTURED (T32 - docs-only reference: shipped string + observed marker) |
| format | custom `[{type} {size}B]` | templates/marker-format-custom.md  | CAPTURED (M10)            |
| hint   | on (default)              | templates/marker-hint-on.md        | UNCAPTURED (unwired - M11 finding: hint string never relayed) |
| hint   | off (`hint = ""`)         | prompts/marker-hint-off.md         | CAPTURED (M11 - file lives in prompts/ not templates/; row fixed) |

### 8. templates - `[templates.prompts]` + `[templates.reverse]`

| Key                  | Variant                  | File                                      | Status                    |
| -------------------- | ------------------------ | ----------------------------------------- | ------------------------- |
| session_inject       | default                  | templates/prompts-templates.md            | CAPTURED (T33 - docs-only: all 5 strings in one reference file) |
| engine_offload       | default                  | templates/prompts-templates.md            | CAPTURED (T33, joint)     |
| auto_expand_guidance | default                  | templates/prompts-templates.md            | CAPTURED (T33, joint)     |
| catalog_context_warn | default                  | templates/prompts-templates.md            | CAPTURED (T33, joint)     |
| search_hint          | default                  | templates/prompts-templates.md            | CAPTURED (T33, joint)     |
| reverse map          | default (type-alias map) | templates/reverse-map.md                  | CAPTURED (T34 - docs-only: full 28-row table) |

## 9. directives - `[directives]`

| Key    | Variant                         | File                                 | Status                    |
| ------ | ------------------------------- | ------------------------------------ | ------------------------- |
| active | ["focus","foresight"] (default) | directives/active-focus-foresight.md | CAPTURED (T01)            |
| active | ["explore"]                     | directives/active-explore.md         | CAPTURED (T02)            |
| active | [] (none)                       | directives/active-empty.md           | CAPTURED (T03)            |

## 10. flow - `[flow]`

| Key          | Variant        | File                       | Status                    |
| ------------ | -------------- | -------------------------- | ------------------------- |
| budget_chars | 2600 (default) | flow/budget_chars-2600.md  | CAPTURED (T04)            |
| budget_chars | 10000          | flow/budget_chars-10000.md | CAPTURED (T05)            |

## 11. env-vars - APHRODITE_* overrides (docs/config/env-vars.md)

| Var                               | Variant         | File                                    | Status                    |
| --------------------------------- | --------------- | --------------------------------------- | ------------------------- |
| APHRODITE_ENGINE_THRESHOLD_PCT    | 90              | env-vars/ENGINE_THRESHOLD_PCT-90.md     | CAPTURED (E01 - BATCH 4)  |
| APHRODITE_TERMINAL_THRESHOLD      | 256             | env-vars/TERMINAL_THRESHOLD-256.md      | CAPTURED (E02 - BATCH 4)  |
| APHRODITE_TOOL_THRESHOLD_TOKEN    | 1024            | env-vars/TOOL_THRESHOLD_TOKEN-1024.md   | CAPTURED (E03 - BATCH 4; added row) |
| APHRODITE_INLINE_THRESHOLD        | 512             | env-vars/INLINE_THRESHOLD-512.md        | CAPTURED (E04 - BATCH 4; added row) |
| APHRODITE_CACHE_PORT / TOKEN_PORT | alternate ports | env-vars/PORTS-alternate.md             | CAPTURED (E05 - BATCH 4: 19797/19798 healthy + defaults still serve) |
| APHRODITE_CCR_TTL_SECONDS         | 60              | env-vars/CCR_TTL_SECONDS-60.md          | CAPTURED (E06 - BATCH 4)  |
| APHRODITE_DIRECTIVES_DIR          | scratch dir     | env-vars/DIRECTIVES_DIR-scratch.md      | CAPTURED (E07 - BATCH 4; added row) |
| APHRODITE_FLOW_BUDGET_CHARS       | 10000           | env-vars/FLOW_BUDGET_CHARS-10000.md     | CAPTURED (E08 - BATCH 4; variant fixed from 5000 → 10000) |
| APHRODITE_CCR_DB_PATH             | alternate path  | env-vars/CCR_DB_PATH-alt.md             | UNCAPTURED (not in BATCH 4 scope) |
| APHRODITE_NOTIFY_URL / KEY        | set             | env-vars/NOTIFY-set.md                  | UNCAPTURED (not in BATCH 4 scope) |
| APHRODITE_API_URL / MODEL         | env set         | env-vars/API_URL_MODEL-env.md           | UNCAPTURED (not in BATCH 4 scope) |

---

## Coverage summary (post BATCH 4 reconciliation)

| Category                    | Requested | Captured | UNCAPTURED |
| --------------------------- | --------- | -------- | ---------- |
| proxies                     | 5         | 4        | 1          |
| defaults                    | 4         | 1        | 3          |
| compression                 | 37        | 31       | 6          |
| previews                    | 10        | 10       | 0          |
| prompts                     | 9         | 9        | 0          |
| templates (preview)         | 13        | 4        | 9          |
| templates (marker)          | 4         | 3        | 1          |
| templates (prompts+reverse) | 6         | 6        | 0          |
| directives                  | 3         | 3        | 0          |
| flow                        | 2         | 2        | 0          |
| env-vars                    | 11        | 8        | 3          |
| **TOTAL**                   | **104**   | **81**   | **23**     |

Captured includes joint rows (a single file covering 2+ matrix rows - model_family ls rows, ccr_marker_hint folds, session_inject default, proxy defaults, [templates.prompts] 5-in-1). Files on disk: **69** (54 from batches 1-3 + 15 from BATCH 4).

## Gaps (UNCAPTURED, do not re-request)

- proxies: tool_relay false (not hot-reloadable - restart required; values documented in cache-default.md)
- defaults: ccr_ttl_seconds 60 TOML variant (env form captured as E06); api_url / model env-only (not in BATCH 4 scope)
- compression: engine_protect_first 0; engine_protect_last 0; engine_min_msgs 2; tool_threshold_cache 512; auto_expand_limit 0; context_engine false
- templates preview: compact {build_output,terminal,json}; code_first {build_output,terminal}; balance {diff,build_output,terminal,json} (9)
- templates marker: hint on (unwired per M11 - the hint string is never relayed, no observable variant)
- env-vars: CCR_DB_PATH alternate; NOTIFY_URL/KEY set; API_URL/MODEL set (3)

## Quality spot-check log

- BATCH 4 (final): every new capture verified against its stream-json session transcript (sess-*.jsonl in `.hermes/tmp/example-exchange/`): marker strings quoted byte-for-byte (e357cd16…|ls|5896, 940fbe94…|ls|5889, 997cb298…|ls|576, b1b5c7c7…|diff|987, 2607dcd7…|json|1728, fafc789b…|build_error|801); session IDs recorded; scratch changes removed (git status clean; grep EXAMPLE-CAPTURE in runtime config == 0).