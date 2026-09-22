# Component Architecture

Top-level components and the two boundaries that matter: the **C-ABI** (Python shim ↔ Rust dylib) and the **HTTP** boundary (LLM clients / Hermes ↔ the two loopback proxies ↔ upstream API). Runtime artifacts (binary, dylib, config, directives, logs) live in the canonical runtime home `~/.hermes/aphrodite` - the plugin dir holds only the loader + downloader, and layout self-heal (`layout_check.py` + `layout_schema.json`) repairs any misplaced runtime state back to that home at startup.

```mermaid
graph TB
	subgraph host["Hermes host process"]
		HERMES["Hermes runtime<br/>(registers hooks/tools; skills are dev-side,<br/>never shipped)"]
		subgraph plugin["Python plugin (thin loader)"]
			SHIM["__init__.py ctypes shim<br/>_load_dylib · _probe_dylib · _call_json · register<br/>layout_check · download.sh"]
		end
	end

	subgraph runtimehome["$HERMES_HOME/aphrodite (canonical runtime home)"]
		BIN["binaries/<br/>aphrodite + libaphrodite_hermes.{dylib,so,dll}<br/>(populated by explicit download.sh / aphrodite setup)"]
		DIRS2["directives/<br/>(materialized builtins)"]
		CFG["aphrodite.toml · ccr.db · proxy-stderr.log"]
	end

	subgraph dylib["libaphrodite_hermes.{dylib,so,dll}"]
		BR["aphrodite-hermes (bridge crate)<br/>process-global STATE (OnceLock)<br/>dispatch_tool · call_hook · get_schemas/get_hooks<br/>version · proxy_health · materialize_directives"]
	end

	subgraph corelib["aphrodite core crate (rlib, linked into dylib + binary)"]
		HOOKS["hooks (transform_*, pre/post_llm_call)"]
		FLOW["flow::build_turn_context (assembler)"]
		DIRS["directives · session · catalog<br/>builtin_directives/*.md (embedded)"]
		CCRMOD["marker · resolve · preview · stage2 · struct_extract"]
		COREABI["core C-ABI: aphrodite_init/dispatch (HANDLES)"]
	end

	subgraph vendored["vendor/headroom-core (aphrodite-headroom-core)"]
		TRAIT["CcrStore trait · compute_key (BLAKE3)"]
		BSQL["SqliteCcrStore"]
		BMEM["InMemoryCcrStore (LRU+TTL)"]
		BRED["RedisCcrStore (compiled, no call site)"]
	end

	subgraph binproc["aphrodite proxy binary (separate process)"]
		MAIN["main.rs · dual axum listeners"]
		subgraph proxies["proxies"]
			CACHEP[":9797 cache proxy (InMemory CCR)"]
			TOKENP[":9798 token proxy (Sqlite CCR)"]
		end
		PH["proxy_handler · compress_chat_completion"]
		INLINE["inline_ccr LRU (1024) · response_cache LRU"]
	end

	LLMCLIENT["OpenAI/Anthropic-compatible client"]
	UPSTREAM["Upstream LLM API (DeepSeek/OpenAI/…)"]

	HERMES -->|register/invoke| SHIM
	SHIM -.->|"C-ABI ctypes"| BR
	SHIM -->|"fetch via download.sh (BINARY_VERSION-pinned)"| BIN
	SHIM -->|"materialize builtins"| DIRS2
	SHIM -->|"unique-path copies"| HOT
	SHIM -->|"read/write"| CFG
	BIN -->|"loads"| BR
	BR --> HOOKS
	BR --> FLOW
	HOOKS --> DIRS
	HOOKS --> CCRMOD
	BR --> COREABI

	SHIM -.->|"subprocess: aphrodite (auto-launch)"| MAIN
	SHIM -->|HTTP /health| CACHEP
	LLMCLIENT -->|HTTP /v1/chat/completions| CACHEP
	LLMCLIENT -->|HTTP| TOKENP
	CACHEP --> PH
	TOKENP --> PH
	PH --> INLINE
	PH -->|HTTP forward| UPSTREAM

	PH --> TRAIT
	HOOKS -.->|inline_store| corelib
	TRAIT --> BSQL
	TRAIT --> BMEM
	TRAIT --> BRED
	TOKENP --> BSQL
	CACHEP --> BMEM
	CCRMOD --> TRAIT

	classDef boundary stroke-dasharray: 5 5;
```

Boundary notes:

- **C-ABI (dashed ctypes edges):** `aphrodite-hermes` exposes seven `aphrodite_hermes_*` process-global functions (dispatch, hooks, schemas, version, health, directives materialize - no skill export anymore); the core crate additionally exposes handle-based `aphrodite_*` functions (`aphrodite_init` + `aphrodite_dispatch`). Both guard panics so no unwind crosses FFI. Declarations come from the generated `_bindings.py` (cbindgen → ctypesgen), replayed onto the plugin's CDLL handle by `bind_to()` with a manual restype fallback and a `_REQUIRED_VOID_P` completeness assertion.
- **HTTP boundary:** clients and Hermes talk to the two loopback listeners; `loopback_only` + `require_mgmt_token` middleware gate the management routes, and a catch-all route forwards everything else to upstream.
- **Store backends:** token proxy → SQLite, cache proxy → in-memory LRU; Redis is compiled in the vendored crate but wired by no `build_state` arm.
- **Runtime home vs plugin tree:** the plugin is a pure loader. Every runtime artifact resolves from `~/.hermes/aphrodite`, with the env overrides `APHRODITE_BINARY_PATH` / `APHRODITE_HERMES_DYLIB_PATH` / `APHRODITE_HOME` taking precedence; plugin-dir `binaries/` copies survive only as legacy fallbacks, and `layout_check.py` migrates misplaced files back to the canonical home at `register()`.
- The **core crate is linked into both** the dylib (Hermes path) and the proxy binary - the marker/resolve/preview logic is shared, but the two run in separate processes with separate CCR state.
