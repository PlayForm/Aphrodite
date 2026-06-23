# Health Endpoint

Origin: External load-balancers and monitoring systems need a fast, public
health check that doesn't require loopback access. The `/health` endpoint is
intentionally exempt from loopback enforcement.

Source of truth: `crates/aphrodite/src/proxy.rs:health_check()` (line 1796),
`crates/aphrodite/src/main.rs:357`

## Endpoint

```
GET /health
```

## Access

**Public** - no loopback enforcement. The only endpoint exempt from the
`loopback_only` middleware (main.rs:357):

```rust
// Public route (no loopback enforcement) merged with restricted routes
let app = Router::new()
    .route("/health", get(health_check))
    .merge(restricted)  // includes all loopback-only routes
```

## Response

```json
{
	"status": "healthy",
	"ccr": true,
	"mode": "token",
	"version": "0.5.69",
	"fill_pct": 90.0
}
```

Always returns HTTP 200 - capability state conveyed via JSON body (CCR is
optional/opt-in).

## Fields

| Field      | Type   | Description                         |
| ---------- | ------ | ----------------------------------- |
| `status`   | string | Always `"healthy"`                  |
| `ccr`      | bool   | Whether CCR store is enabled        |
| `mode`     | string | `"cache"` or `"token"`              |
| `version`  | string | `CARGO_PKG_VERSION`                 |
| `fill_pct` | float  | Context fill percentage (0.0-100.0) |

## Note

`/health` does NOT probe the upstream API. For upstream health, use
`/health/upstream` (loopback-only).
