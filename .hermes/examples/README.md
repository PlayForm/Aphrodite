# Aphrodite Example Catalog - Coverage Matrix (Authoritative)

Maintained by AGENT B (cataloger). Source of truth for schema coverage: `crates/aphrodite/templates/aphrodite.toml` + `docs/config/env-vars.md`.
Every row = one captured variant. `UNCAPTURED <reason>` = capture failed twice, do not re-request.
Exchange protocol: requests flow via `.hermes/tmp/example-exchange/notes.md` (B → A `REQUEST <id>`, A → B `DONE <id>: <file>`).

Last updated: 2026-09-19 (mission 1.4.6 start - **0 captured / 0 total**). Matrix populated with all variants requested in the queue; coverage fills in as `DONE` lines land.

---

## 1. proxies - `[[proxies]]` (name, listen, mode, tool_relay, timeout)

| Key         | Variant                                                        | File                        | Status                    |
| ----------- | -------------------------------------------------------------- | --------------------------- | ------------------------- |
| cache proxy | default (name=cache, mode=cache, tool_relay=true, timeout=120) | proxies/cache-default.md    | UNCAPTURED (requested P3) |
| token proxy | default (name=token, mode=token, tool_relay=true, timeout=300) | proxies/token-default.md    | UNCAPTURED (requested P3) |
| tool_relay  | true                                                           | proxies/tool_relay-true.md  | UNCAPTURED (requested P3) |
| tool_relay  | false                                                          | proxies/tool_relay-false.md | UNCAPTURED (requested P3) |
| timeout     | 120 (cache) / 300 (token)                                      | proxies/timeout-default.md  | UNCAPTURED (requested P3) |

## 2. defaults - `[defaults]`

| Key             | Variant                      | File                             | Status                    |
| --------------- | ---------------------------- | -------------------------------- | ------------------------- |
| ccr_ttl_seconds | 3600 (default)               | defaults/ccr_ttl_seconds-3600.md | UNCAPTURED (requested P3) |
| ccr_ttl_seconds | 60                           | defaults/ccr_ttl_seconds-60.md   | UNCAPTURED (requested P3) |
| api_url         | env-only (APHRODITE_API_URL) | env-vars/api_url-env.md          | UNCAPTURED (requested P3) |
| model           | env-only (APHRODITE_MODEL)   | env-vars/model-env.md            | UNCAPTURED (requested P3) |

## 3. compression - `[compression]`

| Key                  | Variant          | File                                     | Status                    |
| -------------------- | ---------------- | ---------------------------------------- | ------------------------- |
| engine_threshold_pct | 45 (default)     | compression/engine_threshold_pct-45.md   | UNCAPTURED (requested P0) |
| engine_threshold_pct | 100 (disabled)   | compression/engine_threshold_pct-100.md  | UNCAPTURED (requested P0) |
| engine_threshold_pct | 20 (aggressive)  | compression/engine_threshold_pct-20.md   | UNCAPTURED (requested P0) |
| engine_protect_first | 2 (default)      | compression/engine_protect_first-2.md    | UNCAPTURED (requested P3) |
| engine_protect_first | 0                | compression/engine_protect_first-0.md    | UNCAPTURED (requested P3) |
| engine_protect_last  | 5 (default)      | compression/engine_protect_last-5.md     | UNCAPTURED (requested P3) |
| engine_protect_last  | 0                | compression/engine_protect_last-0.md     | UNCAPTURED (requested P3) |
| engine_min_msgs      | 8 (default)      | compression/engine_min_msgs-8.md         | UNCAPTURED (requested P3) |
| engine_min_msgs      | 2                | compression/engine_min_msgs-2.md         | UNCAPTURED (requested P3) |
| tool_threshold_token | 512 (default)    | compression/tool_threshold_token-512.md  | UNCAPTURED (requested P0) |
| tool_threshold_token | 4096             | compression/tool_threshold_token-4096.md | UNCAPTURED (requested P0) |
| tool_threshold_cache | 4096 (default)   | compression/tool_threshold_cache-4096.md | UNCAPTURED (requested P3) |
| tool_threshold_cache | 512              | compression/tool_threshold_cache-512.md  | UNCAPTURED (requested P3) |
| terminal_threshold   | 512              | compression/terminal_threshold-512.md    | UNCAPTURED (requested P0) |
| terminal_threshold   | 1024 (default)   | compression/terminal_threshold-1024.md   | UNCAPTURED (requested P0) |
| terminal_threshold   | 4096             | compression/terminal_threshold-4096.md   | UNCAPTURED (requested P0) |
| inline_threshold     | 2048 (default)   | compression/inline_threshold-2048.md     | UNCAPTURED (requested P0) |
| inline_threshold     | 512              | compression/inline_threshold-512.md      | UNCAPTURED (requested P0) |
| auto_expand          | true (default)   | compression/auto_expand-true.md          | UNCAPTURED (requested P0) |
| auto_expand          | false            | compression/auto_expand-false.md         | UNCAPTURED (requested P0) |
| auto_expand_limit    | 102400 (default) | compression/auto_expand_limit-102400.md  | UNCAPTURED (requested P3) |
| auto_expand_limit    | 0                | compression/auto_expand_limit-0.md       | UNCAPTURED (requested P3) |
| catalog_mode         | tool (default)   | compression/catalog_mode-tool.md         | UNCAPTURED (requested P0) |
| catalog_mode         | compact          | compression/catalog_mode-compact.md      | UNCAPTURED (requested P0) |
| catalog_mode         | full             | compression/catalog_mode-full.md         | UNCAPTURED (requested P0) |
| classifier_poll      | true (default)   | compression/classifier_poll-true.md      | UNCAPTURED (requested P0) |
| classifier_poll      | false            | compression/classifier_poll-false.md     | UNCAPTURED (requested P0) |
| code_multiplier      | 3.0 (default)    | compression/code_multiplier-3.0.md       | UNCAPTURED (requested P0) |
| code_multiplier      | 1.0              | compression/code_multiplier-1.0.md       | UNCAPTURED (requested P0) |
| context_engine       | true (default)   | compression/context_engine-true.md       | UNCAPTURED (requested P3) |
| context_engine       | false            | compression/context_engine-false.md      | UNCAPTURED (requested P3) |
| prefetch             | true (default)   | compression/prefetch-true.md             | UNCAPTURED (requested P3) |
| prefetch             | false            | compression/prefetch-false.md            | UNCAPTURED (requested P3) |
| poll_worker          | true (default)   | compression/poll_worker-true.md          | UNCAPTURED (requested P3) |
| poll_worker          | false            | compression/poll_worker-false.md         | UNCAPTURED (requested P3) |
| chain_split          | false (default)  | compression/chain_split-false.md         | UNCAPTURED (requested P3) |
| chain_split          | true             | compression/chain_split-true.md          | UNCAPTURED (requested P3) |

## 4. previews - `[previews]`

| Key                | Variant         | File                                     | Status                    |
| ------------------ | --------------- | ---------------------------------------- | ------------------------- |
| model_family       | compact         | previews/model_family-compact-code.md    | UNCAPTURED (requested P1) |
| model_family       | compact (ls)    | previews/model_family-compact-ls.md      | UNCAPTURED (requested P1) |
| model_family       | code_first      | previews/model_family-code_first-code.md | UNCAPTURED (requested P1) |
| model_family       | code_first (ls) | previews/model_family-code_first-ls.md   | UNCAPTURED (requested P1) |
| model_family       | balance         | previews/model_family-balance-code.md    | UNCAPTURED (requested P1) |
| model_family       | balance (ls)    | previews/model_family-balance-ls.md      | UNCAPTURED (requested P1) |
| code_structure_map | true (default)  | previews/code_structure_map-true.md      | UNCAPTURED (requested P1) |
| code_structure_map | false           | previews/code_structure_map-false.md     | UNCAPTURED (requested P1) |
| preview_max_chars  | 120 (default)   | previews/preview_max_chars-120.md        | UNCAPTURED (requested P1) |
| preview_max_chars  | 0               | previews/preview_max_chars-0.md          | UNCAPTURED (requested P1) |

## 5. prompts - `[prompts]`

| Key                  | Variant                | File                                  | Status                    |
| -------------------- | ---------------------- | ------------------------------------- | ------------------------- |
| retrieve_guidance    | minimal (default)      | prompts/retrieve_guidance-minimal.md  | UNCAPTURED (requested P2) |
| retrieve_guidance    | standard               | prompts/retrieve_guidance-standard.md | UNCAPTURED (requested P2) |
| retrieve_guidance    | verbose                | prompts/retrieve_guidance-verbose.md  | UNCAPTURED (requested P2) |
| ccr_marker_hint      | false (default)        | prompts/ccr_marker_hint-false.md      | UNCAPTURED (requested P2) |
| ccr_marker_hint      | true                   | prompts/ccr_marker_hint-true.md       | UNCAPTURED (requested P2) |
| catalog_intent_hints | false (default)        | prompts/catalog_intent_hints-false.md | UNCAPTURED (requested P2) |
| catalog_intent_hints | true                   | prompts/catalog_intent_hints-true.md  | UNCAPTURED (requested P2) |
| session_inject       | default (shipped text) | prompts/session_inject-default.md     | UNCAPTURED (requested P2) |
| session_inject       | "" (disabled)          | prompts/session_inject-empty.md       | UNCAPTURED (requested P2) |

## 6. templates - `[templates.preview.{compact,code_first,balance}]` rendered previews

Family-specific RENDERED preview strings for the 4 core content types (diff, build_output, terminal, json).

| Family     | Type         | File                                         | Status                    |
| ---------- | ------------ | -------------------------------------------- | ------------------------- |
| compact    | diff         | templates/preview-compact-diff.md            | UNCAPTURED (requested P3) |
| compact    | build_output | templates/preview-compact-build_output.md    | UNCAPTURED (requested P3) |
| compact    | terminal     | templates/preview-compact-terminal.md        | UNCAPTURED (requested P3) |
| compact    | json         | templates/preview-compact-json.md            | UNCAPTURED (requested P3) |
| code_first | diff         | templates/preview-code_first-diff.md         | UNCAPTURED (requested P3) |
| code_first | build_output | templates/preview-code_first-build_output.md | UNCAPTURED (requested P3) |
| code_first | terminal     | templates/preview-code_first-terminal.md     | UNCAPTURED (requested P3) |
| code_first | json         | templates/preview-code_first-json.md         | UNCAPTURED (requested P3) |
| balance    | diff         | templates/preview-balance-diff.md            | UNCAPTURED (requested P3) |
| balance    | build_output | templates/preview-balance-build_output.md    | UNCAPTURED (requested P3) |
| balance    | terminal     | templates/preview-balance-terminal.md        | UNCAPTURED (requested P3) |
| balance    | json         | templates/preview-balance-json.md            | UNCAPTURED (requested P3) |

### 7. templates - `[templates.marker]`

| Key    | Variant                   | File                               | Status                    |
| ------ | ------------------------- | ---------------------------------- | ------------------------- |
| format | default                   | templates/marker-format-default.md | UNCAPTURED (requested P2) |
| format | custom `[{type} {size}B]` | templates/marker-format-custom.md  | UNCAPTURED (requested P2) |
| hint   | on (default)              | templates/marker-hint-on.md        | UNCAPTURED (requested P2) |
| hint   | off (`hint = ""`)         | templates/marker-hint-off.md       | UNCAPTURED (requested P2) |

### 8. templates - `[templates.prompts]` + `[templates.reverse]`

| Key                  | Variant                  | File                                      | Status                    |
| -------------------- | ------------------------ | ----------------------------------------- | ------------------------- |
| session_inject       | default                  | templates/prompts-session_inject.md       | UNCAPTURED (requested P3) |
| engine_offload       | default                  | templates/prompts-engine_offload.md       | UNCAPTURED (requested P3) |
| auto_expand_guidance | default                  | templates/prompts-auto_expand_guidance.md | UNCAPTURED (requested P3) |
| catalog_context_warn | default                  | templates/prompts-catalog_context_warn.md | UNCAPTURED (requested P3) |
| search_hint          | default                  | templates/prompts-search_hint.md          | UNCAPTURED (requested P3) |
| reverse map          | default (type-alias map) | templates/reverse-default.md              | UNCAPTURED (requested P3) |

## 9. directives - `[directives]`

| Key    | Variant                         | File                                 | Status                    |
| ------ | ------------------------------- | ------------------------------------ | ------------------------- |
| active | ["focus","foresight"] (default) | directives/active-focus-foresight.md | UNCAPTURED (requested P3) |
| active | ["explore"]                     | directives/active-explore.md         | UNCAPTURED (requested P3) |
| active | [] (none)                       | directives/active-empty.md           | UNCAPTURED (requested P3) |

## 10. flow - `[flow]`

| Key          | Variant        | File                       | Status                    |
| ------------ | -------------- | -------------------------- | ------------------------- |
| budget_chars | 2600 (default) | flow/budget_chars-2600.md  | UNCAPTURED (requested P3) |
| budget_chars | 10000          | flow/budget_chars-10000.md | UNCAPTURED (requested P3) |

## 11. env-vars - APHRODITE_* overrides (docs/config/env-vars.md)

| Var                               | Variant         | File                                | Status                    |
| --------------------------------- | --------------- | ----------------------------------- | ------------------------- |
| APHRODITE_ENGINE_THRESHOLD_PCT    | 90              | env-vars/ENGINE_THRESHOLD_PCT-90.md | UNCAPTURED (requested P3) |
| APHRODITE_TERMINAL_THRESHOLD      | 256             | env-vars/TERMINAL_THRESHOLD-256.md  | UNCAPTURED (requested P3) |
| APHRODITE_CACHE_PORT / TOKEN_PORT | alternate ports | env-vars/PORTS-alternate.md         | UNCAPTURED (requested P3) |
| APHRODITE_CCR_TTL_SECONDS         | 60              | env-vars/CCR_TTL_SECONDS-60.md      | UNCAPTURED (requested P3) |
| APHRODITE_CCR_DB_PATH             | alternate path  | env-vars/CCR_DB_PATH-alt.md         | UNCAPTURED (requested P3) |
| APHRODITE_NOTIFY_URL / KEY        | set             | env-vars/NOTIFY-set.md              | UNCAPTURED (requested P3) |
| APHRODITE_API_URL / MODEL         | env set         | env-vars/API_URL_MODEL-env.md       | UNCAPTURED (requested P3) |
| APHRODITE_FLOW_BUDGET_CHARS       | 5000            | env-vars/FLOW_BUDGET_CHARS-5000.md  | UNCAPTURED (requested P3) |

---

## Coverage summary

| Category                    | Requested | Captured | UNCAPTURED |
| --------------------------- | --------- | -------- | ---------- |
| proxies                     | 5         | 0        | 0          |
| defaults                    | 4         | 0        | 0          |
| compression                 | 37        | 0        | 0          |
| previews                    | 10        | 0        | 0          |
| prompts                     | 9         | 0        | 0          |
| templates (preview)         | 12        | 0        | 0          |
| templates (marker)          | 4         | 0        | 0          |
| templates (prompts+reverse) | 6         | 0        | 0          |
| directives                  | 3         | 0        | 0          |
| flow                        | 2         | 0        | 0          |
| env-vars                    | 8         | 0        | 0          |
| **TOTAL**                   | **100**   | **0**    | **0**      |

## Gaps (requested, awaiting capture)

- P0: engine_threshold_pct {45,100,20}; terminal_threshold {512,1024,4096}; inline_threshold {2048,512}; tool_threshold_token {512,4096}; auto_expand {true,false}; catalog_mode {tool,compact,full}; classifier_poll {true,false}; code_multiplier {3.0,1.0} - 18 captures
- P1: model_family {compact,code_first,balance} × {code read, ls}; code_structure_map {true,false}; preview_max_chars {120,0} - 10 captures
- P2: retrieve_guidance {minimal,standard,verbose}; ccr_marker_hint {true,false}; catalog_intent_hints {true,false}; session_inject {default,""}; marker format {default,custom}; marker hint {on,off} - 13 captures
- P3: directives {default,[explore],[]}; flow budget_chars {2600,10000}; chain_split {true,false}; prefetch {true,false}; poll_worker {true,false}; context_engine {true,false}; engine_protect_first {2,0}; engine_protect_last {5,0}; engine_min_msgs {8,2}; auto_expand_limit {102400,0}; tool_threshold_cache {4096,512}; ccr_ttl_seconds {3600,60}; proxies (default, tool_relay, timeout); templates (12 preview + 4 marker + 6 prompts/reverse); env-vars (8) - 59 captures

## Quality spot-check log

- No examples captured yet at mission start - spot-check will run on the first 3 `DONE` files (verify: real captured shape, marker string preserved exactly, front-matter complete, trigger prompt present).
