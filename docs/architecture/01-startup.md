# Process Startup & Dual-Proxy Launch

`aphrodite` startup splits into two halves. The binary half resolves configuration (env > TOML > default), binds every listener before spawning any server, builds one `AppState` per listener with a CCR store backend, and arms a config-file hot-reload watcher. The Hermes plugin half runs once per Hermes home inside `register()`: it loads and smoke-tests the dylib, self-heals the runtime layout, materializes the built-in directives, registers hooks and tools, and launches the proxy when it is not already healthy.

## Startup sequence

```mermaid
sequenceDiagram
    autonumber
    actor OS as OS / shell
    participant main as main()
    participant rt as tokio runtime
    participant run as run()
    participant cfg as MultiConfig
    participant bind as bind loop
    participant bs as proxy::build_state
    participant ccr as CcrStore backend
    participant watch as config watcher task
    participant single as run_single

    OS->>main: exec `aphrodite [args]`
    main->>main: --version / --help early-exit checks
    main->>main: `aphrodite setup` subcommand handled before the runtime
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
    run->>run: init tracing subscriber

    loop each (name, cli) in proxies
        bind->>bind: resolve relative ccr_db_path vs exe dir · mkdir -p parent
        bind->>bind: TcpListener::bind(cli.listen) - a bind failure aborts startup
        bind->>bs: build_state(&cli, compression)
        bs->>bs: resolve_thresholds(compression)
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

    OS-->>main: SIGINT / SIGTERM
    main->>single: shutdown_tx.send(true) → all listeners drain
    main->>main: select: drain done | 5s timeout | 2nd Ctrl+C → abort remaining
```

## Port override (env pierces TOML per mode)

```mermaid
flowchart TD
    A["ProxyConfig (from aphrodite.toml)"] --> B["MultiConfig::resolve"]
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
    J -->|present, malformed| L["warn! · keep listen"]
    J -->|absent| I
    K --> M["Cli.listen"]
    L --> M
    I --> M
```

## Plugin startup (register() - the Hermes-side half)

`register()` runs once per Hermes home (one `PluginManager` per home) and is a **pure loader**: all logic lives in the dylib. Runtime artifacts (binary, dylib, `aphrodite.toml`, `ccr.db`, hot-reload cache) live under the canonical runtime home `~/.hermes/aphrodite`, never inside the plugin tree.

```mermaid
sequenceDiagram
    autonumber
    participant H as Hermes host (PluginManager)
    participant R as register()
    participant DL as _load_dylib
    participant PR as _probe_dylib (subprocess)
    participant LH as layout_check.check_and_heal
    participant MD as aphrodite_hermes_materialize_directives
    participant GH as aphrodite_hermes_get_hooks / get_schemas
    participant SP as _start_proxy

    H->>R: register(ctx)
    R->>DL: _load_dylib()
    DL->>DL: _ensure_binaries(): download.sh → ~/.hermes/aphrodite/binaries/<br/>if binary/dylib missing (version-pinned, SHA-256 verified)
    DL->>PR: subprocess smoke-test (once per unique path)
    PR-->>DL: rc==0 → load fresh unique-path copy → CDLL
    DL->>DL: _configure_ffi: _bindings.py bind_to → manual fallback →<br/>_REQUIRED_VOID_P assertion
    R->>LH: check_and_heal() (best-effort, never raises)
    Note over LH: canonical layout per layout_schema.json:<br/>relocate misplaced config/binaries out of the plugin dir,<br/>quarantine stale plugin-source copies, warn on ambiguity
    R->>MD: materialize_directives(home_dir=b"")
    MD-->>R: {"status":"ok","written":[...],"skipped":[...],"warnings":[...]}
    Note over MD: embeds the built-in directives from the binary<br/>(include_str!) → writes ~/.hermes/aphrodite/directives/<br/>idempotent, never overwrites user-modified files
    R->>GH: hooks[] + schemas[]
    loop each hook (per-hook try/except - one failure never aborts)
        R->>R: ctx.register_hook(name, _hook_dispatch) (fresh dylib per call)
    end
    loop each tool (per-tool try/except)
        R->>R: ctx.register_tool(name, "aphrodite", schema, _make_handler(name))
    end
    Note over R: skills are not shipped - they live dev-side<br/>context engine is opt-in (APHRODITE_CONTEXT_ENGINE=1)
    R->>SP: _start_proxy()
    SP->>SP: pre-launch probe: GET /health on :9797/:9798<br/>(direct-only opener, confirmed "healthy" body) → reuse running instance?
    alt both healthy
        SP->>SP: skip launch
    else launch needed
        SP->>SP: Popen(aphrodite binary) · stderr → ~/.hermes/aphrodite/proxy-stderr.log
        SP->>SP: immediate-death check (stderr tail + API-key hint) ·<br/>poll both health endpoints ≤5s
    end
```

The layout self-heal and the directives materialize are best-effort by design: either failing degrades to a warning and the plugin still registers (never a raise). The dylib load itself is the one hard gate - if the smoke test or the FFI assertion fails, `register()` logs "plugin disabled" and returns.

## Call sites

| Concern                                                               | Module                                                                        |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Runtime + subcommand dispatch                                         | `crates/aphrodite/src/main.rs`                                                |
| Config path resolution + bind-before-spawn                            | `crates/aphrodite/src/main.rs`                                                |
| Config hot-reload watcher                                             | `crates/aphrodite/src/main.rs`                                                |
| Per-listener router + serve                                           | `crates/aphrodite/src/main.rs`                                                |
| `MultiConfig::resolve` / `apply_port_override`                        | `crates/aphrodite/src/config/`                                                |
| `proxy::build_state` (CCR backend selection)                          | `crates/aphrodite/src/proxy.rs`                                               |
| `resolve_thresholds`                                                  | `crates/aphrodite/src/proxy.rs`                                               |
| `SqliteCcrStore` / `InMemoryCcrStore`                                 | `vendor/headroom/crates/headroom-core/src/ccr/backends/{sqlite,in_memory}.rs` |
| Plugin `register()` / `_load_dylib` / `_probe_dylib` / `_start_proxy` | `plugins/aphrodite/__init__.py`                                               |
| Layout self-heal                                                      | `plugins/aphrodite/layout_check.py` (+ `layout_schema.json`)                  |
| Directives materialize FFI                                            | `crates/aphrodite-hermes/src/lib.rs`                                          |
