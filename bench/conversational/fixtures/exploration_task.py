"""EXPLORATION_TASK"""

from .types import Conversation, ToolCall, Turn

# ═══════════════════════════════════════════════════════════════════════════════
EXPLORATION_TASK = Conversation(
    name="exploration_task",
    description="Explore an unfamiliar codebase - search, read many files, understand architecture",
    system_prompt="""You are a software exploration assistant. Help the user understand an unfamiliar codebase by searching for relevant code, reading files, and explaining the architecture. Use search and read tools systematically.""",
    turns=[
        Turn(
            role="user",
            content="""I just joined a project and need to understand its HTTP proxy implementation. The codebase is at `/tmp/proxy_project/`. It seems to handle API requests, compression, and has multiple proxy modes. Can you explore it and give me an architectural overview? Start by finding the main entry points.""",
        ),
        Turn(
            role="assistant",
            content="Let me explore the project structure and find the key files.",
            tool_calls=[
                ToolCall(
                    id="call_e1",
                    name="search_files",
                    arguments='{"path": "/tmp/proxy_project", "pattern": "*.rs", "target": "files"}',
                ),
            ],
        ),
        Turn(
            role="tool",
            tool_call_id="call_e1",
            content="""Found 47 Rust source files:

./src/main.rs (2.1 KB)
./src/lib.rs (4.8 KB)
./src/proxy.rs (24.3 KB)
./src/handlers/mod.rs (1.2 KB)
./src/handlers/openai.rs (8.7 KB)
./src/handlers/anthropic.rs (6.2 KB)
./src/handlers/gemini.rs (4.1 KB)
./src/handlers/bedrock.rs (5.3 KB)
./src/handlers/batch.rs (3.9 KB)
./src/handlers/streaming.rs (7.4 KB)
./src/compression/mod.rs (2.1 KB)
./src/compression/engine.rs (12.4 KB)
./src/compression/classifier.rs (5.8 KB)
./src/compression/marker.rs (3.2 KB)
./src/compression/store.rs (8.9 KB)
./src/compression/retrieve.rs (4.5 KB)
./src/compression/catalog.rs (6.7 KB)
./src/compression/inline.rs (2.3 KB)
./src/config.rs (9.1 KB)
./src/config_loader.rs (5.4 KB)
./src/session.rs (7.8 KB)
./src/state.rs (4.2 KB)
./src/flow.rs (6.3 KB)
./src/hooks.rs (3.1 KB)
./src/setup.rs (2.8 KB)
./src/preview.rs (5.9 KB)
./src/directives.rs (4.7 KB)
./src/resolve.rs (3.3 KB)
./src/prefetch.rs (6.1 KB)
./src/marker.rs (2.2 KB)
./src/poll_worker.rs (3.8 KB)
./src/struct_extract.rs (5.5 KB)
./src/stage2.rs (4.9 KB)
./src/catalog.rs (3.6 KB)
./src/tools/mod.rs (1.8 KB)
./src/tools/aphrodite_catalog.rs (2.4 KB)
./src/tools/aphrodite_compress.rs (3.7 KB)
./src/tools/aphrodite_retrieve.rs (2.9 KB)
./src/tools/aphrodite_search.rs (2.2 KB)
./src/tools/aphrodite_stats.rs (1.8 KB)
./src/tools/aphrodite_test.rs (3.1 KB)
./src/tools/aphrodite_diff.rs (1.5 KB)
./src/tools/aphrodite_files.rs (1.4 KB)
./src/tools/aphrodite_prefetch.rs (2.6 KB)
./src/tools/aphrodite_rebuild.rs (1.7 KB)
./src/tools/aphrodite_directive.rs (2.0 KB)
./src/tools/aphrodite_reclassify.rs (1.9 KB)

Total: 47 files, ~240 KB of Rust source""",
        ),
        Turn(
            role="assistant",
            content="Good, I can see the project structure. Let me read the main entry point, the proxy module, and the config to understand the core architecture.",
            tool_calls=[
                ToolCall(
                    id="call_e2",
                    name="read_file",
                    arguments='{"path": "/tmp/proxy_project/src/main.rs"}',
                ),
                ToolCall(
                    id="call_e3",
                    name="read_file",
                    arguments='{"path": "/tmp/proxy_project/src/proxy.rs", "limit": 200}',
                ),
                ToolCall(
                    id="call_e4",
                    name="read_file",
                    arguments='{"path": "/tmp/proxy_project/src/config.rs"}',
                ),
            ],
        ),
        Turn(
            role="tool",
            tool_call_id="call_e2",
            content="""// src/main.rs - Binary entry point for the Aphrodite proxy
//
// Two modes: `cache` (compress tool results → CCR) and `token`
// (compress chat completions → CCR offload). Both accept the
// standard /v1/chat/completions endpoint and relay to upstream.
//
// Usage:
//   aphrodite --mode cache --listen 127.0.0.1:9797  (tool relay + cache proxy)
//   aphrodite --mode token --listen 127.0.0.1:9798  (context window compression)

use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};
use aphrodite::{config, proxy, session, state};
use aphrodite::config_loader::ConfigSource;

/// CLI arguments parsed by clap - env vars like APHRODITE_MODE /
/// APHRODITE_LISTEN / APHRODITE_API_URL / APHRODITE_API_KEY take
/// precedence over flags; the config file is the fallback.
#[derive(Parser, Debug)]
#[command(name = "aphrodite", version, about)]
struct Cli {
    /// Proxy mode: "cache" (tool-relay + tool-output CCR) or "token" (chat CCR offload)
    #[arg(long, env = "APHRODITE_MODE", default_value = "cache")]
    mode: String,

    /// Listen address (e.g. "127.0.0.1:9797")
    #[arg(long, env = "APHRODITE_LISTEN", default_value = "127.0.0.1:9797")]
    listen: SocketAddr,

    /// Upstream API base URL
    #[arg(long, env = "APHRODITE_API_URL")]
    api_url: Option<String>,

    /// Upstream API key
    #[arg(long, env = "APHRODITE_API_KEY")]
    api_key: Option<String>,

    /// Path to the config TOML file
    #[arg(long, env = "APHRODITE_CONFIG_PATH")]
    config_path: Option<String>,

    /// CCR database path (token mode)
    #[arg(long, env = "APHRODITE_CCR_DB_PATH")]
    ccr_db_path: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured JSON logging - RUST_LOG controls filter level.
    // In production, this feeds into the operator's log aggregator.
    fmt().json().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();

    // Config resolution order (highest wins):
    //   1. CLI flags / env vars
    //   2. aphrodite.toml in the working directory
    //   3. Hard-coded defaults
    let source = config_loader::resolve_source(
        cli.config_path.as_deref(),
        cli.mode.clone(),
    );
    let cfg = config::load(source, &cli).await?;

    // Shared application state - one instance per process.
    // In `cache` mode this is just a HashMap; in `token` mode it
    // wraps a SQLite-backed CCR store with TTL-based expiry.
    let state = state::AppState::new(&cfg).await?;

    // Session manager - handles per-connection session state,
    // including the catalog of compressed content available to
    // each conversation.
    let session_mgr = session::SessionManager::new();

    // Build the Axum router and start the server.
    let app = proxy::build_router(Arc::new(state), Arc::new(session_mgr), &cfg);

    tracing::info!(
        mode = %cfg.mode,
        listen = %cfg.listen,
        api_url = %cfg.api_url,
        "Aphrodite proxy starting"
    );

    let listener = tokio::net::TcpListener::bind(cfg.listen).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_e3",
            content="""// src/proxy.rs - HTTP proxy: routes, middleware, compression pipeline
//
// Architecture:
// ┌──────────────┐     POST /v1/chat/completions      ┌──────────────────┐
// │   Client     │ ──────────────────────────────────> │  Aphrodite Proxy │
// │  (Hermes)    │ <────────────────────────────────── │                  │
// └──────────────┘     SSE stream / JSON response      └───────┬──────────┘
//                                                              │
//                                          ┌───────────────────┘
//                                          │ POST /v1/chat/completions
//                                          ▼
//                              ┌───────────────────────┐
//                              │  Upstream API          │
//                              │  (DeepSeek / OpenAI)    │
//                              └───────────────────────┘
//
// Cache mode (port 9797): Intercepts tool outputs > threshold,
//   compresses them to CCR, replaces with markers. The LLM
//   sees <<<CCR:hash|type|size>>> instead of large outputs.
//
// Token mode (port 9798): Monitors context window size.
//   When approaching the LLM's context limit, offloads older
//   messages to CCR. Injects aphrodite_retrieve guidance.
//
// Endpoints:
//   POST /v1/chat/completions  - Main chat endpoint (both modes)
//   POST /ccr/create            - Store content in CCR
//   POST /retrieve              - Retrieve CCR entry by hash
//   GET  /stats                 - Proxy health + compression stats
//   GET  /health                - Liveness check
//   POST /reload                - Hot-reload config

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response, sse::{Event, Sse}},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};

use crate::compression::{self, CompressionEngine};
use crate::config::Config;
use crate::session::SessionManager;
use crate::state::AppState;

// ── Router construction ────────────────────────────────────────

pub fn build_router(
    state: Arc<AppState>,
    sessions: Arc<SessionManager>,
    cfg: &Config,
) -> Router {
    let shared = Arc::new(ProxyState {
        app: state,
        sessions,
        config: cfg.clone(),
        engine: CompressionEngine::new(cfg),
    });

    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/ccr/create", post(ccr_create))
        .route("/retrieve", post(retrieve))
        .route("/ccr/{hash}", delete(ccr_delete))
        .route("/stats", get(stats))
        .route("/health", get(health))
        .route("/reload", post(reload_config))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(shared)
}

// ── Shared proxy state ────────────────────────────────────────

pub struct ProxyState {
    pub app: Arc<AppState>,
    pub sessions: Arc<SessionManager>,
    pub config: Config,
    pub engine: CompressionEngine,
}

// ── Chat completions handler ──────────────────────────────────
//
// This is the core compression entry point. In token mode:
//   1. Check if context window is near capacity
//   2. If yes, select messages to offload
//   3. Compress them into CCR
//   4. Replace with an offload notice + retrieve guidance
//   5. Relays the modified request to upstream
//   6. Streams the response back
//
// In cache mode:
//   Passes through to upstream without message compression.
//   Tool output compression happens in the Hermes process
//   (which calls /ccr/create directly).

pub async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(mut body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // ... (200 more lines of handler implementation)
}
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_e4",
            content="""// src/config.rs - Configuration types and loading
//
// Loads from aphrodite.toml, CLI flags, and environment variables.
// Supports hot-reload via POST /reload and filesystem watcher.

use std::net::SocketAddr;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Top-level configuration root (maps to aphrodite.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AphroditeConfig {
    /// Multiple proxy definitions (for multi-mode operation)
    #[serde(default)]
    pub proxies: Vec<ProxyConfig>,

    /// Shared defaults applied to all proxies unless overridden
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// Compression thresholds and engine settings
    #[serde(default)]
    pub compression: CompressionConfig,

    /// Preview template configuration per model family
    #[serde(default)]
    pub previews: PreviewsConfig,

    /// Prompt guidance templates
    #[serde(default)]
    pub prompts: PromptsConfig,

    /// Template strings for CCR markers and previews
    #[serde(default)]
    pub templates: TemplatesConfig,

    /// Behavioral directives
    #[serde(default)]
    pub directives: DirectivesConfig,

    /// Per-turn flow budget
    #[serde(default)]
    pub flow: FlowConfig,
}

/// A single proxy instance definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy name: "cache" or "token"
    pub name: String,

    /// Listen address (e.g., "0.0.0.0:9797")
    pub listen: SocketAddr,

    /// Operating mode: "cache" or "token"
    pub mode: String,

    /// Enable tool relay (forward tool calls to upstream)
    #[serde(default = "default_true")]
    pub tool_relay: bool,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Optional per-proxy API URL override
    pub api_url: Option<String>,

    /// Optional per-proxy API key override
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_api_url")]
    pub api_url: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default = "default_ccr_ttl")]
    pub ccr_ttl_seconds: u64,
}

/// Compression thresholds - these are the knobs that determine
/// when and how content gets compressed. Hot-reloadable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Context engine activation threshold (percentage of max context)
    #[serde(default = "default_engine_threshold_pct")]
    pub engine_threshold_pct: u8,

    /// Messages to protect at the start (system prompt + early turns)
    #[serde(default = "default_protect_first")]
    pub engine_protect_first: usize,

    /// Messages to protect at the end (recent conversation)
    #[serde(default = "default_protect_last")]
    pub engine_protect_last: usize,

    /// Minimum messages before compression activates
    #[serde(default = "default_min_msgs")]
    pub engine_min_msgs: usize,

    /// Token proxy threshold in bytes
    #[serde(default = "default_token_threshold")]
    pub tool_threshold_token: usize,

    /// Cache proxy threshold in bytes
    #[serde(default = "default_cache_threshold")]
    pub tool_threshold_cache: usize,

    /// Terminal output threshold
    #[serde(default = "default_terminal_threshold")]
    pub terminal_threshold: usize,

    /// Inline zlib fallback threshold
    #[serde(default = "default_inline_threshold")]
    pub inline_threshold: usize,

    /// Auto-expand markers in responses
    #[serde(default)]
    pub auto_expand: bool,

    /// Number of markers to auto-expand (0 = disabled)
    #[serde(default)]
    pub auto_expand_limit: usize,

    /// Catalog output mode: "compact", "full", or "tool"
    #[serde(default = "default_catalog_mode")]
    pub catalog_mode: String,

    /// Skip CCR for clean outputs (classifier pre-filter)
    #[serde(default = "default_true")]
    pub classifier_poll: bool,

    /// Code type multiplier
    #[serde(default = "default_code_multiplier")]
    pub code_multiplier: f64,

    /// Context engine status flag
    #[serde(default = "default_true")]
    pub context_engine: bool,

    /// Enable prefetch tool
    #[serde(default = "default_true")]
    pub prefetch: bool,

    /// Enable poll worker
    #[serde(default = "default_true")]
    pub poll_worker: bool,
}

// ... (default value functions and remaining config types)
fn default_true() -> bool { true }
fn default_timeout() -> u64 { 120 }
fn default_api_url() -> String { "https://api.deepseek.com".into() }
fn default_model() -> String { "deepseek-v4-pro".into() }
fn default_ccr_ttl() -> u64 { 3600 }
fn default_engine_threshold_pct() -> u8 { 45 }
fn default_protect_first() -> usize { 2 }
fn default_protect_last() -> usize { 5 }
fn default_min_msgs() -> usize { 8 }
fn default_token_threshold() -> usize { 512 }
fn default_cache_threshold() -> usize { 4096 }
fn default_terminal_threshold() -> usize { 1024 }
fn default_inline_threshold() -> usize { 2048 }
fn default_catalog_mode() -> String { "tool".into() }
fn default_code_multiplier() -> f64 { 3.0 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreviewsConfig {
    pub model_family: Option<String>,
    pub code_structure_map: Option<bool>,
    pub preview_max_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptsConfig {
    pub retrieve_guidance: Option<String>,
    pub ccr_marker_hint: Option<bool>,
    pub catalog_intent_hints: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplatesConfig {
    pub preview: Option<serde_json::Value>,
    pub marker: Option<serde_json::Value>,
    pub prompts: Option<serde_json::Value>,
    pub reverse: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectivesConfig {
    pub active: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    #[serde(default = "default_flow_budget")]
    pub budget_chars: usize,
}

fn default_flow_budget() -> usize { 4000 }

impl Default for FlowConfig {
    fn default() -> Self { FlowConfig { budget_chars: default_flow_budget() } }
}

/// Runtime config (resolved for a single proxy instance)
#[derive(Debug, Clone)]
pub struct Config {
    pub mode: String,
    pub listen: SocketAddr,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub ccr_ttl_seconds: u64,
    pub ccr_db_path: Option<String>,
    pub tool_relay: bool,
    pub timeout: u64,
    pub compression: CompressionConfig,
    pub previews: PreviewsConfig,
    pub prompts: PromptsConfig,
    pub templates: TemplatesConfig,
    pub directives: DirectivesConfig,
    pub flow: FlowConfig,
}

impl Config {
    pub async fn load_from_file(path: &str) -> anyhow::Result<Self> { todo!() }
    pub fn hot_reload(&mut self, new: AphroditeConfig) { todo!() }
}
""",
        ),
        # Architectural summary
        Turn(
            role="assistant",
            content="""Here's the architectural overview of this proxy project:

## Aphrodite Proxy Architecture

### Two Operating Modes

1. **Cache Proxy** (port 9797): Compresses tool outputs (file reads, terminal results, search output) into CCR markers. The LLM sees `<<<CCR:hash|type|size>>>` instead of large tool outputs. Includes tool relay - forwards tool calls to the upstream API.

2. **Token Proxy** (port 9798): Monitors context window utilization. When approaching capacity (45% by default), offloads older messages to CCR and injects retrieval guidance. Only the most recent messages stay in context.

### Request Flow (both modes)
```
Client (Hermes) → POST /v1/chat/completions → Aphrodite Proxy
    → [token mode: compress context if needed]
    → POST /v1/chat/completions → Upstream API (DeepSeek)
    → [stream response back to client]
```

### Key Modules

| Module | Role |
|--------|------|
| `proxy.rs` | HTTP server, routing, main handlers |
| `handlers/` | Provider-specific request/response adapters (OpenAI, Anthropic, Gemini, Bedrock) |
| `compression/engine.rs` | Core compression pipeline - classifier + marker generation + retrieval |
| `compression/store.rs` | CCR storage backend (in-memory for cache, SQLite for token) |
| `config.rs` | TOML config loading, hot-reload support |
| `session.rs` | Per-connection session tracking, catalog management |
| `flow.rs` | Per-turn injected context budget management |

### Compression Pipeline
1. **Classifier** (`classifier.rs`): Determines content type (code, diff, build_output, error, JSON, etc.)
2. **Threshold check**: Compares size against per-type thresholds
3. **Marker generation** (`marker.rs`): Creates `<<<CCR:hash|type|size>>>` markers with preview templates
4. **Storage** (`store.rs`): Stores original content, keyed by content hash (BLAKE3)
5. **Retrieval** (`retrieve.rs`): Resolves markers back to original content

The architecture is clean and modular - new API providers can be added by implementing a handler in the `handlers/` directory, and new compression strategies go in `compression/`.""",
        ),
    ],
)
