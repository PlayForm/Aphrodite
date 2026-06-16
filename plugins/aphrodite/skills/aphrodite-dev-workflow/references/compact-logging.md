# Compact Proxy Logging

## Production Command

```bash
APHRODITE_API_KEY=sk-... APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=info cargo watch -x 'run -p aphrodite'
```

## What This Does

- `APHRODITE_LOG_COMPACT=1`: Enables compact log format in main.rs — no timestamps, no target prefix, lean output
- `RUST_LOG=aphrodite=info`: Shows only aphrodite info-level messages; suppresses all hyper_util, rustls, reqwest, tokio, tower noise

## Output Example

**Before (noisy):**
```
2026-06-15T11:22:02.344691Z TRACE hyper_util::client::legacy::pool: checkout waiting for idle connection
2026-06-15T11:22:02.344726Z DEBUG reqwest::connect: starting new connection
2026-06-15T11:22:02.353243Z TRACE rustls::client::hs: We got ServerHello...
```

**After (clean):**
```
 INFO aphrodite: starting 2 proxy listener(s)
 INFO aphrodite: proxy starting name=token listen=127.0.0.1:9798
 INFO aphrodite: listening addr=127.0.0.1:9798
```

## Implementation

In `crates/aphrodite/src/main.rs`:

```rust
let compact = std::env::var("APHRODITE_LOG_COMPACT").is_ok();
let subscriber = tracing_subscriber::registry().with(filter);
if compact {
    subscriber
        .with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
        .try_init()?;
} else {
    subscriber
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;
}
```

## Debugging

For trace-level aphrodite logs with compact format:
```bash
APHRODITE_LOG_COMPACT=1 RUST_LOG=aphrodite=trace cargo watch -x 'run -p aphrodite'
```

For verbose (full timestamps + targets) without compact:
```bash
RUST_LOG=aphrodite=trace cargo watch -x 'run -p aphrodite'
```
