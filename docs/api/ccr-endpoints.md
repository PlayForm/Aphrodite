# CCR Management Endpoints

These endpoints let the Python plugin, Hermes agent, and external tools
create, list, and delete compressed content entries directly.

## Authentication

All three endpoints are loopback only, and require
`Authorization: Bearer <token>` when `APHRODITE_MGMT_TOKEN` is set (unset =
any loopback caller, back-compat with a one-time startup warning). This is
the management-route auth introduced in v1.3.2 - a hostile local page could
previously issue a CORS "simple request" that lands as a write (seed CCR
entries) even though it can't read the reply. Note this is a different token
from `notify_key` below, which authenticates the *outbound* notification
callback.

## POST /ccr/create

Creates a new CCR entry. Supports both JSON and raw octet-stream bodies.

### Access

Loopback only + mgmt token (see [Authentication](#authentication)).

### JSON Mode

**Content-Type:** `application/json`

**Request:**

```json
{
	"content": "string content to store",
	"key": "optional_custom_hash",
	"ttl_seconds": 3600,
	"tags": ["tag1", "tag2"]
}
```

**Response:**

```json
{
	"hash": "abc123def456...",
	"token_savings_ratio": 2.5,
	"original_size": 100,
	"compressed_size": 40,
	"marker_size": 40
}
```

### Octet-Stream Mode

**Content-Type:** `application/octet-stream`

**Request:** Raw UTF-8 bytes as body.

**Response:** Same JSON schema as JSON mode.

### Notification

If `notify_url` configured: async POST with:

```json
{
	"event": "ccr_created",
	"hash": "...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1"]
}
```

| Property | Value                                        |
| -------- | -------------------------------------------- |
| Auth     | Bearer token via `notify_key`                |
| Timeout  | 5s                                           |
| Tracking | `notify_success` / `notify_failure` counters |

### Errors

| Status | Condition                          |
| ------ | ---------------------------------- |
| 400    | Invalid JSON body                  |
| 400    | Invalid UTF-8 in octet-stream body |

### Types

```rust
// Request
pub struct CcrCreateRequest {
    pub content: String,
    pub key: Option<String>,         // custom hash override
    pub ttl_seconds: Option<u64>,
    pub tags: Option<Vec<String>>,
}

// Response
pub struct CcrCreateResponse {
    pub hash: String,
    pub token_savings_ratio: f64,
    pub original_size: usize,
    pub compressed_size: usize,
    pub marker_size: usize,
}

// Notification
pub struct CcrNotification {
    pub event: String,               // "ccr_created"
    pub hash: String,
    pub created_at: u64,             // unix seconds
    pub ttl: u64,
    pub tags: Vec<String>,
}
```

## GET /ccr/list

Returns CCR entry count and backend info.

### Access

Loopback only + mgmt token (see [Authentication](#authentication)).

### Response (CCR enabled)

```json
{
	"entries": 42,
	"backend": "sqlite",
	"mode": "token"
}
```

### Response (CCR disabled)

```json
{
	"entries": 0,
	"message": "CCR not enabled"
}
```

### Fields

| Field     | Description                                           |
| --------- | ----------------------------------------------------- |
| `entries` | Number of live entries in CCR store                   |
| `backend` | `"sqlite"` (token mode) or `"in_memory"` (cache mode) |
| `mode`    | `"token"` or `"cache"`                                |

## DELETE /ccr/{hash}

Deletes a specific CCR entry by hash.

### Access

Loopback only + mgmt token (see [Authentication](#authentication)).

### Response (200 OK)

```json
{
	"deleted": true,
	"hash": "abc123..."
}
```

### Response (404 Not Found)

```json
{
	"deleted": false,
	"hash": "abc123...",
	"error": "not found"
}
```

### Response (503 Service Unavailable)

```json
{
	"error": "CCR not enabled"
}
```

### Implementation

```rust
pub async fn handle_ccr_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> impl IntoResponse {
    match &state.ccr {
        Some(ccr) => {
            let existed = ccr_del(ccr, &hash).await;
            if existed {
                (StatusCode::OK, Json({"deleted": true, "hash": hash}))
            } else {
                (StatusCode::NOT_FOUND, Json({"deleted": false, "hash": hash, "error": "not found"}))
            }
        },
        None => (StatusCode::SERVICE_UNAVAILABLE, Json({"error": "CCR not enabled"})),
    }
}
```

## Python Plugin Usage

The Python plugin uses `POST /ccr/create` extensively:

| Caller                     | Usage                                                   |
| -------------------------- | ------------------------------------------------------- |
| `_compress_via_proxy`      | Sends `Content-Type: application/octet-stream`          |
| `_compress_handler`        | Gets a hash back and mirrors it in the inline store     |
| `_store_conversation_turn` | Sends turn data and stores the result in `_conv_index`  |
| Context engine             | Sends packed messages, gets a hash, and builds a marker |
