# Health Endpoint

`GET /health` is the public liveness check for load balancers and monitoring
systems. It always returns HTTP 200 - capability state (CCR enabled, mode,
context fill) is conveyed in the JSON body instead of the status code, because
CCR is optional/opt-in.

## Endpoint

| Method | Path      | Access | Auth |
| ------ | --------- | ------ | ---- |
| GET    | `/health` | Public | None |

## Access

`/health` is the only route exempt from loopback enforcement and from
management-token auth. It is registered on its own router without either
middleware layer, so external probes reach it without credentials. Everything
else - including `/metrics` (loopback-only, token-exempt) - sits behind the
loopback gate.

## Response

```json
{
	"status": "healthy",
	"ccr": true,
	"mode": "token",
	"version": "1.4.6",
	"fill_pct": 90.0
}
```

## Fields

| Field      | Type   | Description                                                        |
| ---------- | ------ | ------------------------------------------------------------------ |
| `status`   | string | Always `"healthy"`                                                 |
| `ccr`      | bool   | Whether the CCR store is enabled (disabled with `--no-ccr-marker`) |
| `mode`     | string | `"cache"` or `"token"`                                             |
| `version`  | string | Binary version (`CARGO_PKG_VERSION`)                               |
| `fill_pct` | float  | Context fill percentage, clamped to the range 1.0-99.0             |

## fill_pct

`fill_pct` is derived from the compression-ratio EMA: `100 - (ratio_ema / 20)`,
clamped to 1-99 and stored at x100 precision (the JSON body divides by 100).
It initializes at 90.0 and recomputes after every compression. A higher
compression ratio lowers the fill percentage, signaling more context headroom.

## Upstream Health

`/health` does not probe the upstream API - a 200 here does not mean the model
provider is reachable. For upstream health use `GET /health/upstream`
(loopback only, gated by `APHRODITE_MGMT_TOKEN` when set): it probes
`GET {api_url}/models` with the configured API key, caches the result for 60
seconds, and returns `{"upstream": bool, "cached": bool}`.
