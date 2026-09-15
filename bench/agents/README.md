# bench/agents - mock upstream + wire compatibility

Stdlib-only tooling to exercise the aphrodite proxy's wire behavior without a
real LLM API key. Adapted from the archived `.bench/agents/` suite.

## mock_upstream.py

Fake LLM upstream on `127.0.0.1:19899`. Records every request (path, headers,
body) into an in-process log retrievable via `GET /__bench/last`. Returns
canned completions per dialect:

| Request | Response |
| --- | --- |
| `POST /v1/chat/completions` (body `stream: true`) | SSE stream |
| body marker `want_tool_calls` | completion with large `tool_calls` args |
| body marker `want_big_content` | ~30 KB content completion |
| `POST /v1/messages` | Anthropic Messages response |
| `POST /v1beta/models/...:generateContent` | Gemini response |
| anything else | 200 JSON echo |

```sh
python3 mock_upstream.py [port]
```

## wire_compat.py

Starts mock_upstream + a token-mode proxy on override port 19898 and probes
wire compatibility: request recording, dialect dispatch, SSE passthrough,
large-tool-args and large-content handling. Outputs JSON to
`bench/results/wire-compat-<ts>.json` and a stdout table.

```sh
python3 wire_compat.py [--proxy-bin path/to/aphrodite]
```