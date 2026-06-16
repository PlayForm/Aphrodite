//! # aphrodite — LLM proxy with CCR + tool relay
//!
//! Generic proxy for any OpenAI-compatible API. Two modes:
//! - **Cache** (:9797): in-memory CCR, >8KB threshold, preview preserved
//! - **Token** (:9798): SQLite CCR, >1KB threshold, tool injection + relay
//!
//! ## Quick Start
//!
//! ```bash
//! # Multi-proxy from config
//! aphrodite  # reads aphrodite.toml → starts :9797 + :9798
//!
//! # Single mode
//! aphrodite --mode cache --listen :9797 --api-key $APHRODITE_UPSTREAM_API_KEY
//! aphrodite --mode token --listen :9798 --api-key $APHRODITE_UPSTREAM_API_KEY --tool-relay
//!
//! # Dev mode with verbose logging
//! APHRODITE_UPSTREAM_API_KEY=sk-... cargo watch -x 'run -p aphrodite'
//! ```
//!
//! ## Architecture
//!
//! ```text
//! Hermes → aphrodite (:9797/:9798) → any LLM API
//!              ↓ CCR store (in-memory / SQLite)
//!              ↓ Tool relay (bidirectional)
//!         POST /tool/relay ← aphrodite_retrieve / aphrodite_compress
//! ```
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/health` | Upstream probe + version |
//! | GET | `/stats` | Latency histogram, CCR stats |
//! | GET | `/history` | Ring buffer of last 50 requests |
//! | POST | `/retrieve` | Resolve CCR markers |
//! | POST | `/tool/relay` | aphrodite_retrieve/aphrodite_compress |
//! | POST | `/ccr/create` | Programmatic CCR entry |
//! | GET | `/ccr/list` | Entry count + backend info |
//! | ANY | `/*path` | LLM API pass-through |
//!
//! ## Benchmarks
//!
//! | File | Size | Compressed | Ratio | Latency |
//! |------|------|------------|-------|---------|
//! | LICENSE | 7.0KB | 24B | 290x | 40ms |
//! | README | 2.5KB | 24B | 103x | 64ms |
//! | 20KB text | 20KB | 24B | 833x | — |
//! | **Retrieve** | 20KB | — | — | **27ms avg** |
//!
//! ## Config (aphrodite.toml)
//!
//! ```toml
//! [defaults]
//! api_url = "https://upstream-api.example.com"
//! model = "default-model"
//!
//! [[proxies]]
//! name = "cache"
//! listen = "127.0.0.1:9797"
//! mode = "cache"
//!
//! [[proxies]]
//! name = "token"
//! listen = "127.0.0.1:9798"
//! mode = "token"
//! tool_relay = true
//! ```
//!
//! ## Hermes Integration
//!
//! ```yaml
//! providers:
//!   aphrodite-cache:
//!     api_key_env: APHRODITE_UPSTREAM_API_KEY
//!     provider: deepseek
//!     base_url: http://127.0.0.1:9797
//!   aphrodite-token:
//!     api_key_env: APHRODITE_UPSTREAM_API_KEY
//!     provider: deepseek
//!     base_url: http://127.0.0.1:9798
//! fallback_providers:
//!   - deepseek-direct
//! ```

pub mod config;
pub mod proxy;
pub mod retrieve;
pub mod scripting;

/// Check if Rhai scripting is enabled via env var or CLI flag.
/// Feature-gated: returns false when compiled without `scripting` feature.
pub fn scripting_enabled() -> bool {
	#[cfg(feature = "scripting")]
	{
		std::env::var("APHRODITE_SCRIPTING").map(|v| v == "1").unwrap_or(false)
	}
	#[cfg(not(feature = "scripting"))]
	{
		false
	}
}
