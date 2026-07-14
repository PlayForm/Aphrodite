# Proxy Handlers

Aphrodite exposes HTTP endpoints for proxying LLM API requests, managing CCR
entries, executing tool relay calls, and health checks.

Every management handler below (everything except `proxy_handler` and
`health_check`) additionally requires `Authorization: Bearer <token>` once
`APHRODITE_MGMT_TOKEN` is set - see
[Architecture: Management-Route Authentication](architecture.md#management-route-authentication).

## proxy_handler

Catch-all handler. Forwards any request to the upstream LLM API.

### Endpoint

```
ANY /{*path}  (e.g., POST /v1/chat/completions)
```

Registered as the fallback route.

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
1. Increment requests_total, request_body_bytes
2. Generate UUID request ID (short: first 8 chars)
3. If dev mode: log incoming headers (authorization redacted)
4. Build upstream URL: {api_url}/{path}
5. Determine if Chat Completions request; pick the HTTP client -
   "stream": true requests go out on stream_client (no total timeout)
6. Compute cache key (FNV-1a(api_key:model:messages)) for Chat Completions
   (skipped for streaming requests - they are never cached)
7. Check LLM response cache:
   a. HIT → return cached response with X-Aphrodite-Cache: HIT
   b. MISS → continue
8. Retry loop (3 attempts):
   a. Build reqwest request (strip host, auth, content-length, x-aphrodite-* headers)
   b. Forward to upstream
   c. On transport error: exponential backoff + jitter
9. On success: extract response
   a. Track upstream_errors_4xx/5xx
   b. If Content-Type is text/event-stream: forward chunk-by-chunk
      (Body::from_stream), propagate upstream headers, add
      X-Aphrodite-Streamed: true, count bytes into response_body_bytes,
      count mid-stream chunk errors into sse_stream_errors - skip
      compression and caching entirely, done
   c. Read response body
   d. Track upstream_latency_micros, response_body_bytes
   e. If Chat Completions + CCR enabled:
      - Extract x-headroom-budget from inbound headers
      - compress_chat_completion() (message.content only -
        tool_calls[].function.arguments is never compressed)
      - If compressed: set X-Aphrodite-Compressed: true, store in response_cache
   f. Otherwise: return raw, store in response_cache
10. On failure: track upstream_timeouts, return 502 BAD_GATEWAY with a
    generic {"error": "upstream request failed"} body - the transport
    error's detail (which can embed the upstream URL/host) is recorded
    server-side in last_errors, never leaked to the client
```

### Response Headers

| Header                   | Value                           | When                                 |
| ------------------------ | ------------------------------- | ------------------------------------ |
| `Content-Type`           | application/json; charset=utf-8 | Always                               |
| `X-Aphrodite-Cache`      | HIT or MISS                     | Chat Completions                     |
| `X-Aphrodite-Compressed` | true                            | When compression occurred            |
| `X-Aphrodite-Streamed`   | true                            | SSE (text/event-stream) responses    |
| `X-Aphrodite-Fill-Pct`   | float (0.0-99.0)                | Chat Completions (from fill_pct/100) |

### Forwarded Headers

Stripped before forwarding:

| Header           | Reason                           |
| ---------------- | -------------------------------- |
| `host`           |                                  |
| `authorization`  | Replaced with configured API key |
| `content-length` | Recalculated from body           |
| `x-aphrodite-*`  | Internal                         |

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

### Response

```json
{
    "success": true,
    "result": { ... },
    "error": null,
    "async_call": false
}
```

### Tools Handled

| Tool                 | Behavior                    |
| -------------------- | --------------------------- |
| `aphrodite_retrieve` | inline_ccr → CCR store      |
| `aphrodite_compress` | inline (<256B) or CCR store |
| `aphrodite_list`     | ccr.len()                   |

### Validation

- `aphrodite_retrieve` requires `hash` param (400 if missing, even with query)

### Callback (async mode)

When `callback_url` is provided:

1. SSRF protection: only `https://` URLs accepted
2. Tool execution spawns on `task_tracker`
3. Result POSTed to callback_url with 5s timeout
4. Response: `async_call: true`, stateless

### Auth

Callback uses Bearer token via `notify_key` (from config).

## handle_ccr_create

Programmatic CCR entry creation. Accepts JSON or raw octet-stream.

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

Raw bytes treated as content. Hash computed via `compute_key()` (BLAKE3).

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

### Notification

If `notify_url` configured: fires async POST with `CcrNotification`:

```json
{
	"event": "ccr_created",
	"hash": "...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1"]
}
```

Auth: Bearer token via `notify_key`. Timeout: 5s.

## handle_ccr_list

List CCR entry count.

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

## handle_ccr_delete

Delete a CCR entry by hash.

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

## health_check

Health check endpoint. Always returns 200 - capability state conveyed in body.

### Endpoint

```
GET /health
```

Public (no loopback enforcement).

### Response

```json
{
	"status": "healthy",
	"ccr": true,
	"mode": "token",
	"version": "1.3.3",
	"fill_pct": 90.0
}
```

## handle_retrieve

Resolve CCR markers to original content.

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

1. Validate hash (required, 400 if missing)
2. Check inline_ccr (LruCache, lock dropped before await)
3. Fallback to CCR backend (SQLite/InMemory via blocking thread)
4. Apply query filter (case-insensitive, max 512 chars)
5. Apply pagination (offset + limit, clamped to a 10,000-line cap); set
   `truncated: true` when the returned window doesn't cover the whole
   document
6. Return with source tracking

Full request/response schemas, the pagination contract, and the `truncated`
flag semantics: [Retrieve Endpoint](../api/retrieve.md).
