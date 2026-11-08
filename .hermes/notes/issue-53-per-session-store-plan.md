# Issue #53 follow-up - per-session CCR store isolation (task spec)

Status: **planned, not started.** Scope of the `Aphrodite/v1.8.0` release:
bounded store retention (`store_max_entries` / `store_max_bytes`, LRU
eviction, index-driven sweep, truthful telemetry). This document is the
specification for the _second_ half of
[#53](https://github.com/PlayForm/Aphrodite/issues/53) plus the leftovers the
retention work deliberately left open.

## 1. What is already done (do not re-do)

- Space bound: `[compression] store_max_entries` (default `100000`) /
  `store_max_bytes` (default `536870912`, payload bytes), LRU eviction by
  `last_accessed`, trim-at-open, `0` = unlimited. Env:
  `APHRODITE_STORE_MAX_ENTRIES` / `APHRODITE_STORE_MAX_BYTES`.
- Retrieval cost decoupled from store size: the expiry sweep is debounced
  (60 s) and index-driven; measured 37.40 ms → 0.22 ms on a 300k-row store.
- Telemetry that lies is gone: `/health` carries `fill_pct_kind` +
  `store_entries`, `/stats` `ccr_store` reads the live backend, `/stats/db`
  reports caps, `evicted_total`, `freelist_bytes`.
- **The in-process store is already per-session.** `crates/aphrodite-hermes`
  keys state as `session_id -> Arc<Mutex<AphroditeState>>`, so the inline CCR
  store is per session today. The shared surface is _only_ the token proxy's
  SQLite store.

## 2. The blocker (read before designing)

The token proxy is one HTTP listener shared by every session. It sees OpenAI
request bodies and nothing else:

- Hermes' provider config points all sessions at the same `base_url`, so a
  per-session port cannot be selected client-side.
- The proxy has no session identifier in the payload: no header, no `user`
  field, no conversation id.
- `POST /retrieve` is hash-only, and hashes are content-addressed (BLAKE3
  prefix), so a `(session, hash)` key would break hash-only retrieval unless
  every retrieval call also carried the session.

**Therefore the first task is a feasibility probe, not a refactor**: establish
whether a session identifier can reach the proxy at all. Do not start
partitioning before that probe answers.

## 3. Phase 1 - session-signal probe (research, no production code)

Questions, each answered with a real probe against this machine's Hermes:

1. Does Hermes' provider/model config support a custom header map, and is it
   per-session or global? (`~/.hermes/hermes-agent` source, provider client.)
2. Does Hermes send any stable per-session field the proxy could key on
   (`user`, `metadata`, `X-*`)?
3. Is there a Hermes plugin hook that can mutate outbound LLM request headers
   per session (`pre_llm_call` sees the payload - can it rewrite headers)?
4. Can the plugin register a per-session provider override (base_url/port) so
   a per-session proxy instance is addressable?

**Acceptance:** a written answer per question with the source file/line or
probe output, and one of:

- (A) a session signal is available → Phase 2A (partitioned store), or
- (B) no signal is available → Phase 2B (documented limitation + the cheapest
  real mitigation), and close #53's second ask as "not implementable without a
  Hermes-side change", with the evidence.

**Stop if** the probe turns into "add a header nobody sends" - that is the
failure mode this phase exists to prevent.

## 4. Phase 2A - partitioned store (only if Phase 1 finds a signal)

1. Store: `ccr_entries` gains `session TEXT NOT NULL DEFAULT ''`; primary key
   becomes `(session, hash)` (schema `user_version` 4, migration copies
   existing rows to `session = ''`). `CcrStore` grows session-aware
   `put_for` / `get_for`; the existing trait methods delegate with `''` so the
   in-memory and Redis backends keep compiling.
2. Proxy: read the session from the agreed source (header), default `''`.
   `/retrieve` accepts the same source and falls back to a global lookup so a
   marker produced before the change still resolves.
3. Retention stays global: caps apply to the whole store, eviction order is
   `last_accessed` across sessions. Optional follow-up: a per-session floor so
   one session cannot evict another's hot entries.
4. Config: `[compression] store_isolation = "shared" | "session"`
   (`shared` = today's behaviour, the default until the probe proves the
   signal is always present).

**Acceptance**

- Two concurrent sessions writing the same payload get distinct rows and each
  retrieves only its own.
- A session's eviction never removes a row another session can still reach
  (probe with two stores and one shared cap).
- `shared` mode is byte-identical to `1.8.0` behaviour (same rows, same caps).
- Migration from a `1.8.0` DB preserves every retrievable hash.

## 5. Phase 2B - mitigation when no signal exists (likely path)

- Document in `docs/proxy/` and `Maintain/SECURITY.md` that the token store is
  shared by construction and what an operator can do instead: one Hermes
  profile → one `APHRODITE_DB` path (per-profile isolation, already possible),
  and `store_max_*` to bound the blast radius.
- Surface `evicted_total` / `freelist_bytes` in `aphrodite_stats` so a session
  can see its markers being evicted rather than silently missing them
  (retrieval miss vs. eviction miss are indistinguishable today).
- Close #53's second ask with the probe evidence, explicitly _not_ as "done".

## 6. Leftover from the retention work (independent, small)

- **Compaction pass**: eviction frees pages into the freelist; the file
  plateaus near the cap but never shrinks. Add a debounced
  `PRAGMA incremental_vacuum` / `VACUUM` (background connection, off the
  connection mutex, at most once per hour and only when
  `freelist_bytes / database_size_bytes` exceeds a threshold). Acceptance:
  after evicting 90 % of a store, `database_size_bytes` drops; no write latency
  regression in the store bench.
- **Retrieval-vs-eviction distinction**: `/retrieve` returns the same "not
  found" for both. A `X-Aphrodite-Miss: expired|evicted|never-stored` response
  header (best-effort, from a tombstone counter) would make operator reports
  like #53 diagnosable without guessing.

## 7. Gates for any phase

- `cargo test -p aphrodite --lib` and `-p aphrodite-headroom-core --lib` green.
- `cargo clippy -D warnings`, `cargo fmt --check`, `ruff check plugins/aphrodite/`.
- `pnpm run Check:FFI` (the FFI surface changes if the trait grows).
- A before/after store probe on a 300k-row fixture (retention caps, retrieval
  latency, `evicted_total`), recorded in the release note's Infrastructure
  section.
- Headroom fork: crate version + parent pin move together; the crate must be
  published before any parent tag that depends on it.

## 8. Out of scope

- At-rest encryption of `ccr.db` (separate security backlog item).
- A management-auth hardening pass (separate backlog item).
- Any change to what may be compressed (`aphrodite-compression-safety`).
