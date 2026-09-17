# 01 - Process Startup & Dual-Proxy Launch

Traces `aphrodite` process startup: `main()` → runtime build → config
resolution (env > TOML > default) → bind-before-spawn of every listener →
per-listener `AppState` + CCR store init → config-file hot-reload watcher →
graceful shutdown. The two listeners are the `:9797` cache proxy and `:9798`
token proxy, both bound to loopback. The **plugin's** startup (the Hermes-side
half: dylib probe/load, layout self-heal, directives materialize, proxy launch)
is traced in the second section - `register()` is where the runtime home gets
healed and provisioned before anything consumes it.

## Startup sequence

```mermaid
sequenceDiagram
    autonumber
    actor OS as OS / shell
    participant main as main() (main.rs:31)
    participant rt as tokio runtime
    participant run as run() (main.rs:98)
    participant cfg as MultiConfig (config.rs)
    participant bind as bind loop (main.rs:229)
    participant bs as proxy::build_state (proxy.rs:656)
    participant ccr as CcrStore backend
    participant watch as config watcher task (main.rs:251)
    participant single as run_single (main.rs:371)

    OS->>main: exec `aphrodite [args]`
    main->>main: --version / --help early-exit checks
    main->>main: `aphrodite setup` subcommand handled before runtime (main.rs:47)
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
        bind->>bind: resolve relative ccr_db_path vs exe dir · mkdir -p parent
        bind->>bind: TcpListener::bind(cli.listen)  ← FAILS LOUD, aborts startup (F9)
        bind->>bs: build_state(&cli, compression)
        bs->>bs: resolve_thresholds(compression) (proxy.rs:130)
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

    OS-->>main: SIGINT / SIGTERM (shutdown_signal, main.rs:768)
    main->>single: shutdown_tx.send(true) → all listeners drain
    main->>main: select: drain done | 5s timeout | 2nd Ctrl+C → abort remaining
```

## Port-override path (env pierces TOML per-mode)

```mermaid
flowchart TD
    A["ProxyConfig (from aphrodite.toml)"] --> B["MultiConfig::resolve (config.rs:301)"]
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

## Plugin startup (register() - the Hermes-side half)

`plugins/aphrodite/__init__.py:register` runs once per Hermes home (one
PluginManager per home). It is a **pure loader**: all logic lives in
`libaphrodite_hermes.dylib`. The runtime artifacts (binary, dylib,
`aphrodite.toml`, `ccr.db`, hotreload cache) live under the canonical runtime
home `~/.hermes/aphrodite`, never inside the plugin tree.

```mermaid
sequenceDiagram
    autonumber
    participant H as Hermes host (PluginManager)
    participant R as register() (__init__.py:1106)
    participant DL as _load_dylib (__init__.py:492)
    participant PR as _probe_dylib (subprocess, :373)
    participant LH as layout_check.check_and_heal (layout_check.py)
    participant MD as aphrodite_hermes_materialize_directives (hermes/lib.rs:541)
    participant GH as aphrodite_hermes_get_hooks / get_schemas (lib.rs:498,489)
    participant SP as _start_proxy (__init__.py:905)

    H->>R: register(ctx)
    R->>DL: _load_dylib()
    DL->>DL: _ensure_binaries(): download.sh → ~/.hermes/aphrodite/binaries/<br/>if binary/dylib missing (BINARY_VERSION-pinned, SHA-256 verified)
    DL->>PR: subprocess smoke-test (once per unique path)
    PR-->>DL: rc==0 → load fresh unique-path copy → CDLL
    DL->>DL: _configure_ffi: _bindings.py bind_to → manual fallback →<br/>_REQUIRED_VOID_P assertion
    R->>LH: check_and_heal() (best-effort, never raises)
    Note over LH: canonical layout per layout_schema.json:<br/>relocate misplaced config/binaries out of the plugin dir,<br/>quarantine stale plugin-source copies, warn on ambiguity
    R->>MD: materialize_directives(home_dir=b"")
    MD-->>R: {"status":"ok","written":[...],"skipped":[...],"warnings":[...]}
    Note over MD: embeds crates/aphrodite/src/builtin_directives/*.md<br/>(include_str!) → writes ~/.hermes/aphrodite/directives/<br/>idempotent, NEVER overwrites user-modified files
    R->>GH: hooks[] + schemas[]
    loop each hook (per-hook try/except - one failure never aborts)
        R->>R: ctx.register_hook(name, _hook_dispatch) (fresh dylib per call)
    end
    loop each tool (per-tool try/except)
        R->>R: ctx.register_tool(name, "aphrodite", schema, _make_handler(name))
    end
    Note over R: skills are NOT shipped - they live dev-side in .hermes/skills/<br/>context engine opt-in via APHRODITE_CONTEXT_ENGINE=1
    R->>SP: _start_proxy()
    SP->>SP: pre-launch probe: GET /health on :9797/:9798<br/>(direct-only opener, confirmed "healthy" body) → reuse running instance?
    alt both healthy
        SP->>SP: skip launch (INFO)
    else launch needed
        SP->>SP: Popen(aphrodite binary) · stderr → ~/.hermes/aphrodite/proxy-stderr.log
        SP->>SP: immediate-death check (stderr tail + API-key hint) ·<br/>poll both health endpoints ≤5s
    end
```

The layout self-heal and directives materialize are best-effort by design:
either failing degrades to a `_log.warning` and the plugin still registers
(never a raise). The dylib load itself is the one hard gate - if the smoke
test or FFI assertion fails, `register()` logs "plugin disabled" and returns.

## Key call sites

- `main()` runtime + subcommand dispatch - `crates/aphrodite/src/main.rs:31`
- `run()` config path resolution + bind-before-spawn - `crates/aphrodite/src/main.rs:98,229`
- config hot-reload watcher - `crates/aphrodite/src/main.rs:251`
- `run_single()` router + serve - `crates/aphrodite/src/main.rs:371`
- `MultiConfig::resolve` / `apply_port_override` - `crates/aphrodite/src/config.rs:301,410`
- `proxy::build_state` (CCR backend selection) - `crates/aphrodite/src/proxy.rs:656`
- `resolve_thresholds` - `crates/aphrodite/src/proxy.rs:130`
- `SqliteCcrStore::open` / `InMemoryCcrStore::with_capacity_and_ttl` - `vendor/headroom/crates/headroom-core/src/ccr/backends/{sqlite.rs:113,in_memory.rs:100}`
- plugin `register()` / `_load_dylib` / `_probe_dylib` / `_start_proxy` - `plugins/aphrodite/__init__.py:1106,492,373,905` (mirror: `crates/aphrodite/templates/__init__.py`)
- layout self-heal `check_and_heal` - `plugins/aphrodite/layout_check.py` (+ `layout_schema.json`)
- directives materialize FFI - `crates/aphrodite-hermes/src/lib.rs:541` (impl `crates/aphrodite-hermes/src/directives.rs:153`)
