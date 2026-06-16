# examples/

Four atomic benchmarks that test compression aggressiveness and retrieval
correctness against a live binary. No mocking. No stubs. Each spawns its
own proxy pair on non-conflicting ports and tears them down on exit.

## Prerequisites

```sh
cargo build --release   # must be done before running any bench
curl --version          # used for all HTTP calls (no extra Rust deps)
```

## bench_01_corpus

Full corpus run. Fires one sample per content-type through both `cache`
(`:49797`) and `token` (`:49798`) proxies via `/ccr/create`, then retrieves
each compressed entry via `POST /retrieve`. Prints a per-label table.

```sh
cargo run --example bench_01_corpus 2>&1 | tee /tmp/bench_01.log
```

| column | meaning |
|---|---|
| `ratio` | original / marker bytes |
| `retrieve` | OK = `found: true`, MISS = bug R-10/R-11 |

Exits non-zero on any retrieve miss.

## bench_02_threshold

Boundary sweep. Generates payloads at `threshold-1`, `threshold`, and
`threshold+1` for every multiplier:

| type | multiplier | token threshold |
|---|---|---|
| `text` | 1× | 1 KB |
| `code_rust` | 4× | 4 KB |
| `linter` | 0.5× | 512 B |
| `build_output` | 0.5× | 512 B |
| `error` | 8× | 8 KB |
| cache `text` | 1× | 8 KB |

```sh
cargo run --example bench_02_threshold 2>&1 | tee /tmp/bench_02.log
```

Exits non-zero on any boundary violation.

## bench_03_retrieve

Ten labeled correctness checks:

1. Same-port store + retrieve (cache)
2. Same-port store + retrieve (token)
3. Cross-port isolation: token hash must miss on cache port
4. Cross-port isolation: cache hash must miss on token port
5. Inline_ccr zone (257 B): retrievable via `POST /retrieve` (R-10 fix)
6. UTF-8 content: `found=true` (R-5 fix: no byte-boundary panic)
7. UTF-8 content: byte-exact round-trip
8. Bulk storm: 50 inserts → 50 retrieves, 0 misses
9. `DELETE /ccr/{hash}`: entry removed, subsequent retrieve = miss
10. Double-store idempotency: same hash, still retrievable

```sh
cargo run --example bench_03_retrieve 2>&1 | tee /tmp/bench_03.log
```

## bench_04_ema

EMA auto-tune stability and the R-9 guard.

- Phase A: 10 high-ratio inserts → EMA climbs above 5×
- Phase B: 10 incompressible inserts → EMA decays
- Phase C: 10 more high-ratio → EMA recovers
- R-9 guard: when EMA > 20×, measures the live linter and build_output
  thresholds and asserts they stay ≤ 600 B (not doubled to 1 KB by the
  auto-tune 2× branch).

```sh
cargo run --example bench_04_ema 2>&1 | tee /tmp/bench_04.log
```

## Run all

```sh
for e in bench_01_corpus bench_02_threshold bench_03_retrieve bench_04_ema; do
    echo "=== $e ==="
    cargo run --example $e 2>&1 || exit 1
done
```

## Port allocation

| example | cache port | token port |
|---|---|---|
| bench_01_corpus | 49797 | 49798 |
| bench_02_threshold | 59797 | 59798 |
| bench_03_retrieve | 69797 | 69798 |
| bench_04_ema | — | 79798 |

No two examples share a port so they can run concurrently.
