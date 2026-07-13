# Aphrodite - Atomic Test Examples

**These are frozen bug narratives (executable documentation), not regression
tests of the shipped Rust code.** Each `.py` file re-implements both the buggy
and the fixed logic in Python and asserts on that replica - it always passes
regardless of what the current Rust code does, so it can never catch a Rust
regression. The live regression tests (which exercise the real Rust code) are
the `regression_*`-named tests in `crates/aphrodite/src/proxy.rs` and
`crates/aphrodite/src/resolve.rs` - run them with `cargo test -p aphrodite
regression_`. See `.plans/09-testing-quality.md` F2 for the full rationale.

Each file in this directory is **self-contained** and exercises exactly one bug
or improvement from the audit. Run any single file with:

```bash
python examples/ < file > .py
```

No Rust binary, no live proxy, no API key required unless noted.

| File                           | Covers                                        | Pass condition                 |
| ------------------------------ | --------------------------------------------- | ------------------------------ |
| `01_env_var_typo.py`           | `APHRODITEINLINE_THRESHOLD` typo              | env var is read correctly      |
| `02_duplicate_declarations.py` | shadowed `_inline_store` / `INLINE_THRESHOLD` | configured value survives      |
| `03_alive_json_parse.py`       | health-check JSON space mismatch              | healthy status detected        |
| `04_hardcoded_path.py`         | absolute `/Users/username` path               | path resolves from `__file__`  |
| `05_platform_binary.py`        | unused `_detect_platform` in download         | URL includes platform tag      |
| `06_marker_glyph.py`           | `⭷` vs `⫷` Unicode glyph mismatch             | description matches format     |
| `07_tokens_saved.py`           | `AtomicU64` never incremented                 | counter reflects savings       |
| `08_health_upstream.py`        | upstream ping on every `/health`              | upstream checked at most 1/min |
| `09_alive_retry_loop.py`       | fixed 0.5 s sleep, no retry                   | retry loop succeeds on 3rd try |
| `10_alive_double_call.py`      | redundant `_alive()` per turn                 | TTL cache dedups calls         |
| `11_should_compress.py`        | `threshold_percent` never used                | compression skipped below pct  |
| `12_resolve_port_fallback.py`  | retrieval only on token port                  | falls back to cache port       |
| `13_engine_truncation.py`      | `[:2000]` breaks CCR markers                  | full marker preserved          |
| `14_hash_extraction.py`        | pipe-suffix not stripped                      | hash cleaned before lookup     |
| `15_binary_launch_warn.py`     | silent failure when binary missing            | user-visible `RuntimeError`    |
| `16_integration_smoke.py`      | end-to-end proxy + hermes mock                | full round-trip marker→resolve |
