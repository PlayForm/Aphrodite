# 01 - Process Startup & Dual-Proxy Launch

Traces `aphrodite` process startup: `main()` → runtime build → config resolution
(env > TOML > default) → bind-before-spawn of every listener → per-listener
`AppState` + CCR store init → config-file hot-reload watcher → graceful shutdown.
The two listeners are the `:9797` cache proxy and `:9798` token proxy, both bound
to loopback.

## Startup sequence

```mermaid
sequenceDiagram
    autonumber
    actor OS as OS / shell
    participant main as main() (main.rs:38)
    participant rt as tokio runtime
    participant run as run() (main.rs:107)
    participant cfg as MultiConfig (config.rs)
    participant bind as bind loop (main.rs:206)
    participant bs as proxy::build_state (proxy.rs:643)
    participant ccr as CcrStore backend
    participant watch as config watcher task (main.rs:251)
    participant single as run_single (main.rs:371)

    OS->>main: exec `aphrodite [args]`
    main->>main: --version / --help / `setup` early-exit checks
    main->>main: worker_threads = env APHRODITE_WORKER_THREADS<br/>or (cpus*4).max(32)
    main->>rt: Builder::new_multi_thread().worker_threads(n).enable_all()
    rt->>run: block_on(run())

    run->>run: resolve config_path:<br/>APHRODITE_CONFIG_PATH → ./aphrodite.toml → ~/.hermes/aphrodite/aphrodite.toml
    alt config file exists (multi-proxy)
        run->>cfg: MultiConfig::load(path)
        cfg-->>run: proxies[], [compression], [defaults]
        run->>cfg: for each proxy: config.resolve(p) → Cli
        Note over cfg: env > TOML(proxy>defaults) > default<br/>API-key chain, per-mode port override,<br/>timeout clamp ≤600s, max_output<max_context
    else no config (CLI fallback)
        run->>cfg: Cli::parse() (requires --api-key)
    end
    run->>run: init tracing subscriber (log_compact?)

    loop each (name, cli) in proxies
        bind->>bind: resolve relative ccr_db_path vs exe dir; mkdir -p parent
        bind->>bind: TcpListener::bind(cli.listen)  ← FAILS LOUD, aborts startup (F9)
        bind->>bs: build_state(&cli, compression)
        bs->>bs: resolve_thresholds(compression) (proxy.rs:121)
        alt mode == Token && !no_ccr_marker
            bs->>ccr: SqliteCcrStore::open(db_path, ccr_ttl_seconds)
        else mode == Cache
            bs->>ccr: InMemoryCcrStore::with_capacity_and_ttl(10_000, ttl)
        else Token && no_ccr_marker
            bs->>ccr: None (no CCR backend)
        end
        bs-->>bind: Arc<AppState> (atomics seeded: ema=200, fill=9000, thresholds)
        bind->>bind: bound.push((name, cli, listener, state))
    end

    run->>watch: spawn notify watcher on aphrodite.toml dir
    Note over watch: on Modify(aphrodite.toml): debounce 500ms →<br/>MultiConfig::load → resolve_thresholds →<br/>store into every live AppState's 4 atomics

    loop each bound listener
        run->>single: spawn run_single(name, cli, listener, state, shutdown_rx)
        single->>single: warn if APHRODITE_MGMT_TOKEN unset
        single->>single: build restricted Router (/stats,/retrieve,/ccr/*,/reload,...)<br/>+ catch_all /{*path} → proxy_handler<br/>+ /health (public)
        single->>single: axum::serve(...).with_graceful_shutdown(shutdown_rx.changed())
    end

    OS-->>main: SIGINT / SIGTERM (shutdown_signal, main.rs:766)
    main->>single: shutdown_tx.send(true) → all listeners drain
    main->>main: select: drain done | 5s timeout | 2nd Ctrl+C → abort remaining
```

## Port-override path (env pierces TOML per-mode)

```mermaid
flowchart TD
    A["ProxyConfig (from aphrodite.toml)"] --> B["MultiConfig::resolve (config.rs:297)"]
    B --> C{"cfg.listen set?"}
    C -->|Some s| D["s.parse::&lt;SocketAddr&gt; (fail→error)"]
    C -->|None| E["default 127.0.0.1:9797"]
    D --> F{"mode / name"}
    E --> F
    F -->|cache| G["apply_port_override(listen, APHRODITE_CACHE_PORT)"]
    F -->|token| H["apply_port_override(listen, APHRODITE_TOKEN_PORT)"]
    F -->|other| I["listen unchanged"]
    G --> J{"env var parses as u16?"}
    H --> J
    J -->|Ok port| K["addr.set_port(port) · info! log"]
    J -->|present, malformed| L["warn! · keep listen (F10/F15)"]
    J -->|absent| I
    K --> M["Cli.listen"]
    L --> M
    I --> M
```

## Key call sites
- `main()` runtime + subcommand dispatch - `crates/aphrodite/src/main.rs:38`
- `run()` config path resolution + bind-before-spawn - `crates/aphrodite/src/main.rs:107,206`
- config hot-reload watcher - `crates/aphrodite/src/main.rs:251`
- `run_single()` router + serve - `crates/aphrodite/src/main.rs:371`
- `MultiConfig::resolve` / `apply_port_override` - `crates/aphrodite/src/config.rs:297,406`
- `proxy::build_state` (CCR backend selection) - `crates/aphrodite/src/proxy.rs:643`
- `resolve_thresholds` - `crates/aphrodite/src/proxy.rs:121`
- `SqliteCcrStore::open` / `InMemoryCcrStore::with_capacity_and_ttl` - `vendor/headroom/crates/headroom-core/src/ccr/backends/{sqlite.rs:228,in_memory.rs:84}`
