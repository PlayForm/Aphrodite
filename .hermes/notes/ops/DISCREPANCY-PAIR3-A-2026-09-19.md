# Discrepancy log - PAIR-3-A (docs/config/ rewrite, 2026-09-19)

Verified against live source (branch Development, binary 1.4.6): `aphrodite.toml.example`,
`crates/aphrodite/src/config.rs`, `config_loader.rs`, `main.rs`, `proxy.rs`, `setup.rs`,
`crates/aphrodite-hermes/src/{lib.rs,directives.rs,tools.rs}`, `plugins/aphrodite/__init__.py`.
Append-only.

- [2026-09-19] aphrodite-toml.md + env-vars.md: API-key fallback chain claimed
  `proxy.api_key -> defaults.api_key -> APHRODITE_API_KEY -> DEEPSEEK_API_KEY ->
HEADROOM_DEEPSEEK_KEY -> error`; source: chain is `proxy.api_key -> defaults.api_key ->
APHRODITE_API_KEY` then error - the two provider-specific keys have NO reader anywhere
  (crates/aphrodite/src/config.rs:307-318; DEEPSEEK_API_KEY/HEADROOM_DEEPSEEK_KEY appear
  only in a test that removes them, config.rs:657-658).
- [2026-09-19] env-vars.md: `APHRODITE_TOOL_THRESHOLD_TOKEN` / `APHRODITE_TOOL_THRESHOLD_CACHE` /
  `APHRODITE_INLINE_THRESHOLD` "default (no override)" claimed 512 / 4096 / 2048 bytes; source:
  compiled-in consts are 1024 / 8192 / 256 (crates/aphrodite/src/proxy.rs:76-82); the shipped
  aphrodite.toml.example overrides them to 512 / 4096 / 2048 - the old doc conflated shipped
  example values with compiled defaults.
- [2026-09-19] env-vars.md: dylib-path `APHRODITE_TOOL_THRESHOLD_TOKEN` default claimed 512;
  source: 4096 (crates/aphrodite/src/config_loader.rs:142).
- [2026-09-19] env-vars.md: `APHRODITE_CONTEXT_ENGINE` default claimed off; source: the dylib
  status flag defaults TRUE via `get_bool(..., true)` (crates/aphrodite/src/config_loader.rs:126),
  while Hermes-side ContextEngine registration stays gated on the env var being "1"/"true"
  (plugins/aphrodite/**init**.py:1206).
- [2026-09-19] aphrodite-toml.md: `[previews]`/`[prompts]` caveat claimed neither section "has
  any effect"; source: `[previews] preview_max_chars` is wired end-to-end (main.rs:141,
  proxy.rs:2646, config_loader.rs:313-315) and `[prompts] session_inject` is live
  (config_loader.rs:297-302 consumed by flow.rs:38-45) - only the other keys in those sections
  remain unread.
- [2026-09-19] aphrodite-toml.md: `[previews] preview_max_chars` described as "max chars per
  rendered preview line"; source: it caps the whole rendered preview string (config_loader.rs:314,
  preview.rs:978); env > TOML > default unlimited (0), shipped example sets 120.
- [2026-09-19] env-vars.md: `APHRODITE_PREVIEW_MAX_CHARS` was undocumented; source: live reader
  env > TOML > code default unlimited (config_loader.rs:314); shipped example value 120.
- [2026-09-19] env-vars.md: undocumented env vars with live readers: `APHRODITE_HOME`
  (plugins/aphrodite/**init**.py:150, crates/aphrodite/templates/**init**.py:150,
  crates/aphrodite-hermes/src/directives.rs:51), `APHRODITE_DIRECTIVES_DIR`
  (config_loader.rs:222, **init**.py:88), `APHRODITE_POLL_WORKER` (config_loader.rs:155),
  `APHRODITE_CHAIN_SPLIT` / `APHRODITE_CHAIN_SPLIT_MIN_SEGMENTS` /
  `APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS` (config_loader.rs:162-184), `APHRODITE_FLOW_BUDGET_CHARS`
  (config_loader.rs:152), `APHRODITE_SESSION_INJECT` (config_loader.rs:297-302).
- [2026-09-19] aphrodite-toml.md: Modes table claimed cache mode has tool relay "No"; source:
  `tool_relay` is an independent per-proxy flag defaulting to false, and the shipped example
  enables it on BOTH proxies (aphrodite.toml.example:26,34; config.rs:388) - mode does not
  determine it.
- [2026-09-19] aphrodite-toml.md: "The repo's own root aphrodite.toml sets them explicitly as a
  worked example" - no root `aphrodite.toml` exists in the repo; the shipped reference is
  `aphrodite.toml.example` (repo root).
- [2026-09-19] aphrodite-toml.md: `[compression] prefetch` claimed "parsed by the TOML schema,
  echoed by /reload"; source: `CompressionConfig` has no `prefetch` field (config.rs:241-255) and
  /reload's parsed_only omits it (proxy.rs:2667-2673) - the key is ignored entirely; the
  `aphrodite_prefetch` tool exists unconditionally (crates/aphrodite/src/lib.rs:643).
- [2026-09-19] aphrodite-toml.md: `[directives] active` omitted the fallback: when the TOML list
  resolves empty but directives ARE loaded (disk or builtins), the session seeds
  focus/foresight/lazy from the loaded set (config_loader.rs:283-291).
- [2026-09-19] env-vars.md: claimed "Every field on `Cli` is a clap arg with an env attribute, so
  all of the multi-proxy-mode vars above also work here via clap"; source: only
  mode/listen/api_url/api_key/model/ccr_db_path/ccr_ttl_seconds/notify_url/notify_key/log_compact
  carry clap env attrs (config.rs:149-209); max_context/max_output/tool_relay/dev/no_ccr_marker/
  timeout are flag-only with no env var.
- [2026-09-19] aphrodite-toml.md: `[compression] context_engine`/`poll_worker`/`chain_split` keys
  were absent from the doc's section breakdown; source: dylib-only live keys (config_loader.rs:126,
  155, 162) - context_engine is a status flag, poll_worker gates auto-backgrounding,
  chain_split gates fine-grained command splitting (aphrodite-hermes/src/lib.rs pre_tool_call arm).
