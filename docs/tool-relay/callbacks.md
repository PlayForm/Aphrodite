# Callbacks

Two distinct callback surfaces exist around the tool relay: the Hermes-side
hooks that fire when a tool result comes back, and the proxy-side HTTP
deliveries (`callback_url` on the relay, `notify_url` on CCR creation). Both
are described below.

## What fires when a tool result is transformed

The plugin registers every hook the dylib exposes through a single generic
dispatcher (`aphrodite_hermes_call_hook`), so the wrappers live in Rust, not
Python. The hooks that fire around tool results:

| Hook                        | Fires when                          | What it does                                                                                                                 |
| --------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `pre_tool_call`             | A tool is about to run              | Auto-backgrounds long-running `terminal`/`process` commands; optionally rewrites chained commands for fine-grained CCR       |
| `transform_tool_result`     | A tool result comes back            | Unwraps Hermes' JSON result wrapper, classifies the payload, compresses oversize output to CCR and replaces it with a marker |
| `transform_terminal_output` | A terminal output stream comes back | Same classification + compression path for terminal output                                                                   |
| `pre_llm_call`              | Every turn before the model call    | Injects directives, nudges, and the recall catalog into context                                                              |
| `post_llm_call`             | After the model call returns        | Post-turn bookkeeping (e.g. ephemeral directive expiry)                                                                      |

A compressed result is not lost - the marker handed back is resolvable through
`aphrodite_retrieve` for the life of the session. See
[Hooks](https://github.com/PlayForm/Aphrodite/tree/Current/docs/plugin/hooks.md)
for the full hook reference.

## Tool relay callback

`POST /tool/relay` runs the tool synchronously unless the request carries a
`callback_url`, in which case the call becomes asynchronous: the proxy answers
immediately and POSTs the real result to the callback URL later.

```
Hermes Agent
    │
    │ POST /tool/relay
    │  {tool, params, callback_url}
    ▼
Aphrodite Proxy
    │
    ├─ Validate callback_url (https only; else HTTP 400)
    ├─ Execute tool via execute_tool_relay
    ├─ Build ToolRelayResponse
    ├─ Spawn onto task tracker:
    │    └─ POST callback_url (5s timeout, no auth)
    │       → Hermes receives result asynchronously
    │
    └─ Return {success: true, result: null, error: null, async_call: true}
```

### Request

```json
POST /tool/relay
{
	"tool": "aphrodite_retrieve",
	"params": { "hash": "abc123..." },
	"callback_url": "https://hermes.internal/callback"
}
```

### Response (immediate)

```json
{
	"success": true,
	"result": null,
	"error": null,
	"async_call": true
}
```

### Callback delivery

After tool execution, the result is POSTed to `callback_url`:

```json
POST https://hermes.internal/callback
Content-Type: application/json

{
	"success": true,
	"result": { "found": true, "content": "..." },
	"error": null,
	"async_call": false
}
```

The relay accepts `aphrodite_retrieve`, `aphrodite_compress`, and
`aphrodite_list`; any other tool name returns an error. A relayed
`aphrodite_retrieve` without a `hash` (e.g. only a `query`) is rejected up
front with HTTP 400.

### SSRF protection

Only `https://` URLs are accepted for `callback_url`. Anything else - `http://`
including loopback, `file://`, and other schemes - is rejected with HTTP 400
and `{"success": false, "error": "callback_url must use the https scheme"}`.
Nothing is executed for a rejected request.

### Timeouts and delivery

| Operation                    | Timeout   |
| ---------------------------- | --------- |
| Tool relay callback POST     | 5 seconds |
| CCR-create notification POST | 5 seconds |

There are no retries - delivery is fire-and-forget. The callback POST carries
no auth header (the relay result is posted as-is), and its outcome is not
tracked: relay calls are counted in `/stats` under `tool_relay` as `total` /
`success` / `failure`.

## CCR create notification

`POST /ccr/create` stores content directly in the CCR backend (JSON body or
raw octet-stream). When a `notify_url` is configured, every successful create
fires a webhook so external subscribers can track store growth without
polling. The chat-compression and tool-compress paths do not trigger it.

```json
POST {notify_url}
Authorization: Bearer {notify_key}
Content-Type: application/json

{
	"event": "ccr_created",
	"hash": "abc123...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1", "tag2"]
}
```

`ttl` defaults to 3600 seconds and `tags` to an empty list when the request
omits them. Unlike the tool-relay callback, this notification is sent with
`Authorization: Bearer {notify_key}` whenever a key is configured, and its
outcome is counted in the `notify` counters.

## Task tracker

Both delivery paths are spawned onto the proxy's task tracker. On graceful
shutdown the tracker stops accepting new tasks and then waits for any in-flight
callback or notification to finish, so none are lost mid-delivery (a second
shutdown signal aborts remaining tasks).

## Configuration

`notify_url` and `notify_key` are configured per-proxy in `aphrodite.toml`:

```toml
[[proxies]]
notify_url = "https://hermes.internal/aphrodite/callback"
notify_key = "hermes-api-key-123"
```

They can also be set via CLI flags (`--notify-url`, `--notify-key`) or
environment variables (`APHRODITE_NOTIFY_URL`, `APHRODITE_NOTIFY_KEY`), with
the environment taking precedence over TOML. Only `notify_url` is required for
notifications to fire - if it is unset (the default), no notifications are
sent. `notify_key` is optional and only adds the Bearer header when present.

## Metrics

| Counter                                      | Description                                   |
| -------------------------------------------- | --------------------------------------------- |
| `notify.success` / `notify.failure`          | CCR-create notification outcomes, in `/stats` |
| `tool_relay.total` / `.success` / `.failure` | Tool relay call outcomes, in `/stats`         |

Exposed at `/metrics` as `aphrodite_notify_success_total`,
`aphrodite_notify_failure_total`, `aphrodite_tool_relay_calls_total`,
`aphrodite_tool_relay_success_total`, and `aphrodite_tool_relay_failure_total`.
See [Prometheus Metrics](https://github.com/PlayForm/Aphrodite/tree/Current/docs/metrics/prometheus.md)
for the full metrics reference.

`/tool/relay` is one of the loopback-restricted management routes: it accepts
any loopback caller without a credential until `APHRODITE_MGMT_TOKEN` is set,
after which the bearer token is required.
