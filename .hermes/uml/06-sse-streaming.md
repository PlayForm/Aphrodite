# 06 - SSE Streaming Passthrough

Server-sent-event (`text/event-stream`) responses bypass compression and the
response cache entirely, stream chunk-by-chunk with a timeout-free client, and
account bytes/errors mid-stream. Detection happens twice: on the request
(`"stream":true` selects the timeout-free client) and on the response
(`Content-Type` selects the passthrough branch).

## SSE detection & passthrough

```mermaid
sequenceDiagram
    autonumber
    participant C as client
    participant PH as proxy_handler (proxy.rs:913)
    participant BW as body_wants_stream (proxy.rs:768)
    participant SC as stream_client (NO total timeout, proxy.rs:655)
    participant Up as upstream
    participant IS as is_sse (proxy.rs:905)
    participant STR as bytes_stream().inspect

    C->>PH: POST /v1/chat/completions {"stream":true}
    PH->>BW: body_wants_stream(body)
    alt stream:true
        PH->>SC: select stream_client (connect_timeout only, no .timeout())
    else
        PH->>PH: select bounded client (cli.timeout)
    end
    PH->>Up: forward request
    Up-->>PH: response headers (Content-Type)
    PH->>IS: is_sse(content_type)  (prefix match text/event-stream)
    alt is SSE
        PH->>STR: response.bytes_stream().inspect(count)
        loop each chunk (no buffering, no compression, no cache write)
            Up-->>STR: chunk
            alt Ok(bytes)
                STR->>STR: response_body_bytes += len
            else Err
                STR->>STR: sse_stream_errors += 1  (02-F9)
            end
            STR-->>C: forward chunk (Body::from_stream)
        end
        PH-->>C: 200 X-Aphrodite-Streamed: true
    else non-SSE
        PH->>PH: accumulate_body → compress path (see 02)
    end
```

## Why compression is skipped for SSE

```mermaid
flowchart TD
    A["SSE response detected"] --> B["compression SKIPPED"]
    A --> C["response_cache SKIPPED"]
    B --> D["chunks are partial JSON deltas - not whole message.content;<br/>can't classify/hash/marker a fragment"]
    C --> E["cache_key_from_body already returns None when request stream:true"]
    A --> F["timeout-free stream_client - a slow-but-valid stream<br/>must not be killed mid-answer"]
    A --> G["mid-stream errors counted separately from<br/>upstream_connect_errors/upstream_timeouts (pre-header only)"]
```

Byte accounting: only `response_body_bytes` accumulates during the stream (via
the `.inspect` closure); `tokens_saved` is not touched because nothing was
compressed. `sse_stream_errors` surfaces in `/stats` and `/metrics`
(`aphrodite_sse_stream_errors_total`).

## Key call sites
- `body_wants_stream` (request-side detection) - `crates/aphrodite/src/proxy.rs:768`
- `stream_client` construction (no total timeout) - `crates/aphrodite/src/proxy.rs:655`
- client selection in handler - `crates/aphrodite/src/proxy.rs:1019`
- `is_sse` (response-side detection) - `crates/aphrodite/src/proxy.rs:905`
- SSE stream branch (`bytes_stream().inspect`, byte/error counting) - `crates/aphrodite/src/proxy.rs:1102`
- `sse_stream_errors` in `/metrics` - `crates/aphrodite/src/main.rs:550`
