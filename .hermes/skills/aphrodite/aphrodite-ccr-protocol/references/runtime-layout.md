# Runtime layout (loader + runtime home)

`~/.hermes/plugins/aphrodite` holds ONLY the loader (`plugin.yaml` +
`__init__.py`) registering 5 hooks:

- on_session_start
- transform_tool_result
- pre_llm_call
- transform_terminal_output
- post_llm_call

and the 13 CCR tools:

- aphrodite_catalog
- aphrodite_compress
- aphrodite_diff
- aphrodite_directive
- aphrodite_files
- aphrodite_prefetch
- aphrodite_prefetch_status
- aphrodite_rebuild
- aphrodite_reclassify
- aphrodite_retrieve
- aphrodite_search
- aphrodite_stats
- aphrodite_test

Every runtime artifact (binaries, dylib, `aphrodite.toml`, `ccr.db`, logs)
lives under `~/.hermes/aphrodite/`.

Proxy listeners: token proxy `:9798`, cache proxy `:9797`. Port values are
config properties in `[ports]`; read them from the running proxy or the live
toml, never assume, because the values are configuration and can change.
