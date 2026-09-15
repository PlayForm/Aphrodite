# bench - Aphrodite benchmark suite

Benchmark and evaluation tooling for the Aphrodite CCR compression proxy.
Everything here is self-contained, stdlib-first, and never modifies product
code. Working-tree artifacts only on the Development branch; nothing here is
wired into CI.

```
bench/
  corpus/         14 content-type fixtures + gen_corpus.py + metadata.json
                   (+ verify_labels.py, README.md)
  compression/    standalone cargo crate: criterion per-stage micro-benches
                   + crash-regression tests on the pathological fixtures
  conversational/ agent-conversation harness (5 conversations) + results/
  proxy/          end-to-end proxy bench: /ccr/create ratios + latencies +
                   /retrieve round-trip + /metrics deltas (override ports)
  agents/         mock LLM upstream + wire-compat probes (no API key needed)
```

## Migration note

Migrated + adapted from the archived `.bench/` suite (2026-07-13/2026-07-20
runs, previously hidden in the repo) and the archived conversational bench
run `2026-07-20_045201`. All content anonymized (no usernames, emails, keys,
machine paths), version strings updated to the Development reality
(aphrodite v1.4.3 / plugin v2.1.3), and fixtures regenerated deterministically
(`gen_corpus.py`, seed `0xCC12`). The archive originals are preserved
read-only under `~/Developer/.playform/Temporary/Aphrodite/`.

## How to run

| Area | Command |
| --- | --- |
| Corpus verification | `python3.13 bench/corpus/verify_labels.py` |
| Proxy bench (needs built binary) | `cargo build --release -p aphrodite && bench/proxy/bench_proxy.sh` |
| Wire compat | `python3.13 bench/agents/wire_compat.py` (starts its own mock upstream) |
| Conversational harness | `bench/conversational/run_all.sh` |
| Compression benches | `cd bench/compression && cargo test && cargo bench --bench stages -- --quick` |