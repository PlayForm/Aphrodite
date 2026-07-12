# Callbacks

Tool relay and CCR create operations can optionally notify the Hermes agent
via HTTP callback when done, enabling asynchronous patterns. Callbacks run on
the proxy's task tracker, so graceful shutdown always waits for any in-flight
callback to finish before the process exits.

## Architecture

```
Hermes Agent
    │
    │ POST /tool/relay
    │  {tool, params, callback_url}
    ▼
Aphrodite Proxy
    │
    ├─ Validate callback_url (https only)
    ├─ Execute tool synchronously
    ├─ Build ToolRelayResponse
    ├─ POST callback_url (5s timeout, Bearer auth)
    │  → Hermes receives result asynchronously
    │
    └─ Return {async_call: true} immediately
```

## Tool Relay Callback

### Request

```json
POST /tool/relay
{
    "tool": "aphrodite_retrieve",
    "params": {"hash": "abc123..."},
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

### Callback Delivery

After tool execution, a POST is made to `callback_url`:

```json
POST https://hermes.internal/callback
Authorization: Bearer {notify_key}
Content-Type: application/json

{
    "success": true,
    "result": {"found": true, "content": "..."},
    "error": null,
    "async_call": false
}
```

## CCR Create Notification

Fires when a new CCR entry is created and a notification URL is configured.

### Notification Payload

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

## Security

### SSRF Protection

Only `https://` URLs are accepted for `callback_url`; HTTP, file, and other
schemes are rejected and the callback is silently dropped (a warning is
logged, and the proxy still returns a normal synchronous-style response).

### Authentication

Both the callback and the notification request are sent with Bearer token
auth using `notify_key`, when one is configured.

## Timeouts

| Operation         | Timeout   |
| ------------------ | ----------- |
| Callback POST     | 5 seconds |
| Notification POST | 5 seconds |

There are no retries - delivery is fire-and-forget. Success and failure are
tracked via the `notify_success` / `notify_failure` counters.

## Task Tracker

All callback and notification requests are spawned as tracked background
tasks. On shutdown, the tracker stops accepting new tasks and then waits for
any in-flight callback or notification to complete, ensuring none are lost
during a graceful shutdown.

## Configuration

Callbacks are configured per-proxy in `aphrodite.toml`:

```toml
[[proxies]]
notify_url = "https://hermes.internal/aphrodite/callback"
notify_key = "hermes-api-key-123"
```

Both `notify_url` and `notify_key` must be set for callbacks to fire. If
`notify_url` is unset (the default), no callbacks are sent.

## Metrics

| Counter          | Description                                                        |
| ------------------ | --------------------------------------------------------------------- |
| `notify_success` | Incremented when a callback/notification POST returns success status |
| `notify_failure` | Incremented on timeout, connection error, or non-success status      |

Exposed at `/metrics` as `aphrodite_notify_success` and
`aphrodite_notify_failure`. See [Prometheus Metrics](../metrics/prometheus.md)
for the full metrics reference.
