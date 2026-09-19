# Proxy Handlers

The proxy exposes HTTP handlers for proxying LLM API requests, managing CCR
entries, executing tool relay calls, and health checks. This page documents
each handler's endpoint, request/response contract, and behavior; the routing
table, auth model, and middleware are in
[Proxy Architecture](https://github.com/PlayForm/Aphrodite/tree/Current/docs/proxy/architecture.md).

Every management handler below (everything except `proxy_handler` and
`health_check`) additionally requires `Authorization: Bearer <token>` once
`APHRODITE_MGMT_TOKEN` is set - see
[Architecture: Management-Route Authentication](https://github.com/PlayForm/Aphrodite/tree/Current/docs/proxy/architecture.md#management-route-authentication).

## proxy_handler

Catch-all handler. Forwards any request to the upstream LLM API.

### Endpoint

```
ANY /{*path}  (e.g., POST /v1/chat/completions)
```

Registered as the fallback route with a 64 MB body limit.

### Signature

```rust
pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    path: axum::extract::OriginalUri,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse
```

### Flow

```
1. Increment requests_total, request_body_bytes; generate a short UUID request ID
2. In dev mode, log incoming headers (authorization redacted)
3. Build upstream URL: {api_url}/{path} (query string included)
4. Detect Chat Completions (path == /v1/chat/completions); compute the
   response-cache key (FNV-1a) - streaming requests produce no key
5. Check the response cache:
   a. HIT → return cached body with X-Aphrodite-Cache: HIT (no upstream call)
   b. MISS → continue
6. Pick the HTTP client: "stream": true requests go out on stream_client
   (no total timeout)
7. Retry loop (3 attempts, connect-phase errors only):
   a. Build the reqwest request, stripping hop-by-hop and internal headers
   b. Forward to upstream
   c. On connect error: exponential backoff + jitter, retry
   d. On other transport errors: fail fast on the first attempt
8. On success:
   a. Track upstream_errors_4xx/5xx by status code
   b. If Content-Type is text/event-stream: forward chunk-by-chunk via a body
      stream, propagate upstream headers, add X-Aphrodite-Streamed: true,
      count bytes into response_body_bytes and mid-stream chunk errors into
      sse_stream_errors - skip compression and caching entirely, done
   c. Buffer the body (64 MB cap); track upstream_latency_micros and
      response_body_bytes
   d. If Chat Completions + CCR enabled:
      - Read the x-headroom-budget header for adaptive aggressiveness
      - compress_chat_completion() - message.content only; tool_calls[].function.arguments is never compressed
      - If compressed: set X-Aphrodite-Compressed: true, store in the response cache
   e. Otherwise: return raw, store in the response cache (2xx, ≤1 MB)
9. On failure: track upstream_timeouts (timeout) or upstream_connect_errors
   (other transport failures), return 502 BAD_GATEWAY with a generic
   {"error": "upstream request failed"} body - the transport error's detail
   (which can embed the upstream URL/host) is recorded server-side in
   last_errors, never leaked to the client
```

### Response Headers

| Header                   | Value                           | When                                                                              |
| ------------------------ | ------------------------------- | --------------------------------------------------------------------------------- |
| `Content-Type`           | application/json; charset=utf-8 | Cache-hit and compressed paths; the raw path propagates the upstream content type |
| `X-Aphrodite-Cache`      | HIT or MISS                     | Chat Completions                                                                  |
| `X-Aphrodite-Compressed` | true                            | When compression occurred                                                         |
| `X-Aphrodite-Streamed`   | true                            | SSE (text/event-stream) responses                                                 |
| `X-Aphrodite-Fill-Pct`   | float (0.0-99.0)                | Chat Completions (from fill_pct/100)                                              |

Upstream response headers are propagated on the JSON path, with hop-by-hop
headers (`content-length`, `content-type`, `transfer-encoding`, `connection`,
`keep-alive`) skipped.

### Forwarded Headers

Stripped or overridden before forwarding:

| Header            | Reason                                               |
| ----------------- | ---------------------------------------------------- |
| `host`            | Removed                                              |
| `authorization`   | Replaced with the configured upstream API key        |
| `content-length`  | Recalculated from the body                           |
| `content-type`    | Force-set to application/json; charset=utf-8         |
| `accept`          | Force-set to application/json                        |
| `accept-encoding` | Stripped entirely - client has no gzip/brotli decode |
| `x-aphrodite-*`   | Internal                                             |

## handle_tool_relay

Executes aphrodite tools (retrieve, compress, list) with optional async
callback.

### Endpoint

```
POST /tool/relay
```

### Request

```json
{
	"tool": "aphrodite_retrieve",
	"params": { "hash": "abc123..." },
	"callback_url": "https://..." // optional
}
```

### Response (sync)

```json
{
	"success": true,
	"result": { ... },
	"error": null,
	"async_call": false
}
```

### Tools Handled

| Tool                 | Behavior                                                                                                                                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aphrodite_retrieve` | inline_ccr first, then the CCR store; returns `{"found": true, "content": ...}`                                                                         |
| `aphrodite_compress` | inline store (below the inline threshold, also mirrored to the durable backend) or CCR store; returns `{"compressed": marker, "hash", "original_size"}` |
| `aphrodite_list`     | `{"entries": N, "backend": "in_memory"                                                                                                                  | "sqlite"}` |

### Validation

- `aphrodite_retrieve` requires the `hash` param - a request with only `query`
  and no `hash` returns 400 BAD_REQUEST.
- Unknown tool names return an error result.
- `_ccr_center` is an optional param for `aphrodite_compress` (marker center
  annotation).

### Callback (async mode)

When `callback_url` is provided:

1. SSRF protection: only `https://` URLs accepted - anything else returns
   400 with `callback_url must use the https scheme`.
2. Tool execution spawns on the `task_tracker`.
3. Result POSTed to the callback URL with a 5s timeout; success/failure is
   counted into `tool_relay_success`/`tool_relay_failure`.
4. Response: `async_call: true`, stateless.

The callback POST carries no Authorization header (unlike `/ccr/create`
notifications, which attach `notify_key`).

## handle_ccr_create

Programmatic CCR entry creation. Accepts JSON or a raw octet-stream body.

### Endpoint

```
POST /ccr/create
```

### JSON Mode (Content-Type: application/json)

```json
{
	"content": "string content to store",
	"key": "optional_custom_hash",
	"ttl_seconds": 3600,
	"tags": ["tag1", "tag2"]
}
```

### Octet-Stream Mode

Raw bytes treated as the content; the hash is computed via BLAKE3
(`compute_key`), a `key` cannot be supplied.

### Response

```json
{
	"hash": "abc123...",
	"token_savings_ratio": 2.5,
	"original_size": 100,
	"compressed_size": 40,
	"marker_size": 40
}
```

`compressed_size` and `marker_size` equal the bare hash length - the endpoint
contract is the hash, not a rendered marker.

### Errors

| Status | Body                                                                     | When                                    |
| ------ | ------------------------------------------------------------------------ | --------------------------------------- |
| 400    | `{"error": "invalid JSON: ..."}` or `{"error": "invalid UTF-8 in body"}` | Malformed body                          |
| 503    | `{"error": "CCR not enabled"}`                                           | No CCR backend (e.g. `--no-ccr-marker`) |
| 500    | `{"error": "failed to store content in CCR backend"}`                    | Store write failed                      |

### Notification

If `notify_url` is configured, fires an async POST with a `CcrNotification`:

```json
{
	"event": "ccr_created",
	"hash": "...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1"]
}
```

Auth: `Authorization: Bearer <notify_key>` when `notify_key` is set. Timeout:
5s. Success/failure counted into `notify_success`/`notify_failure`.

## handle_ccr_list

Reports the CCR entry count and backend kind (no listing of actual entries).

### Endpoint

```
GET /ccr/list
```

### Response

```json
{
	"entries": 42,
	"backend": "sqlite",
	"mode": "token"
}
```

With no CCR backend: `{"entries": 0, "message": "CCR not enabled"}`.

## handle_ccr_delete

Deletes a CCR entry by hash.

### Endpoint

```
DELETE /ccr/{hash}
```

### Response (200 OK)

```json
{ "deleted": true, "hash": "abc123" }
```

### Response (404)

```json
{ "deleted": false, "hash": "abc123", "error": "not found" }
```

### Response (503)

```json
{ "error": "CCR not enabled" }
```

## handle_ccr_reload

Hot-reloads `aphrodite.toml` and applies the `[compression]` thresholds
(cache, token, inline, code multiplier) plus the preview cap to the live
`AppState`.

### Endpoint

```
POST /reload
```

### Response

```json
{
	"reloaded": true,
	"applied": true,
	"config": "aphrodite.toml",
	"compression": {
		"tool_threshold_cache": 8192,
		"tool_threshold_token": 1024,
		"inline_threshold": 256,
		"code_multiplier": 3.0
	},
	"parsed_only": { ... }
}
```

`parsed_only` echoes `[compression]` keys that have no consumer in the proxy
path (auto-expand, terminal threshold, engine threshold, catalog mode) - they
are reported for visibility but not applied. A parse failure returns 500 with
the error message.

## health_check

Liveness endpoint. Always returns 200 - capability state is conveyed in the
body, since CCR is optional.

### Endpoint

```
GET /health
```

Public (no loopback enforcement, no management token) so external
load-balancer probes work. Does not call the upstream - see
`/health/upstream` for that.

### Response

```json
{
	"status": "healthy",
	"ccr": true,
	"mode": "token",
	"version": "1.4.6",
	"fill_pct": 90.0
}
```

`version` reports the compiled `CARGO_PKG_VERSION`; `fill_pct` is the current
headroom fill percentage (0.0-99.0).

## handle_retrieve

Resolves CCR markers to original content.

### Endpoint

```
POST /retrieve
```

### Request

```json
{
	"hash": "abc123...",
	"query": "optional filter string",
	"offset": 0,
	"limit": 0
}
```

### Response

```json
{
	"found": true,
	"content": "original content...",
	"source": "ccr",
	"error": null,
	"truncated": false
}
```

### Retrieve Flow

1. Validate hash (required - 400 if missing); the hash is normalized (a
   `|type|size` marker-body suffix and surrounding whitespace are stripped).
2. Check inline_ccr (LRU cache, lock dropped before any await).
3. Fall back to the CCR backend (SQLite/In-memory via a blocking thread).
4. Apply the query filter (case-insensitive substring, capped at 512 chars).
5. Apply pagination (offset + limit, clamped to a 10,000-line cap); set
   `truncated: true` when the returned window does not cover the whole
   document, and prepend a `[lines a-b/total]` header to the content.
6. Return with source tracking.

Full request/response schemas, the pagination contract, and the `truncated`
flag semantics: [Retrieve Endpoint](https://github.com/PlayForm/Aphrodite/tree/Current/docs/api/retrieve.md).
