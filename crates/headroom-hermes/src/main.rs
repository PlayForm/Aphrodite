//! headroom-hermes — Hermes Agent wrapper with transparent compression.
//!
//! Architecture (identical to headroom + Claude Code):
//!   Hermes → localhost:9797 → compress → DeepSeek
//!
//! Completely standalone — no headroom_core dependency.
//! Own CCR store (SQLite), own compression pipeline, own proxy.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use rusqlite::Connection;
use serde_json::Value;

// ── CCR Store ────────────────────────────────────────────

struct CcrStore {
    db: Mutex<Connection>,
}

impl CcrStore {
    fn open(path: &str) -> Self {
        let db = Connection::open(path).expect("open CCR db");
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS ccr_entries (
                hash TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s','now'))
            );
            CREATE TABLE IF NOT EXISTS stats (
                key TEXT PRIMARY KEY,
                value INTEGER DEFAULT 0
            );
            INSERT OR IGNORE INTO stats(key,value) VALUES('requests',0);
            INSERT OR IGNORE INTO stats(key,value) VALUES('stored',0);
            INSERT OR IGNORE INTO stats(key,value) VALUES('retrieved',0);"
        ).expect("init CCR schema");
        db.execute("PRAGMA journal_mode=WAL", []).ok();
        CcrStore { db: Mutex::new(db) }
    }

    fn put(&self, hash: &str, content: &str) {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO ccr_entries(hash,content) VALUES(?1,?2)",
            rusqlite::params![hash, content],
        ).ok();
        db.execute("UPDATE stats SET value=value+1 WHERE key='stored'", []).ok();
        db.execute("UPDATE stats SET value=value+1 WHERE key='requests'", []).ok();
    }

    fn get(&self, hash: &str) -> Option<String> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT content FROM ccr_entries WHERE hash=?1"
        ).ok()?;
        let content: String = stmt.query_row(rusqlite::params![hash], |r| r.get(0)).ok()?;
        db.execute("UPDATE stats SET value=value+1 WHERE key='retrieved'", []).ok();
        Some(content)
    }

    fn len(&self) -> usize {
        self.db.lock().unwrap()
            .query_row("SELECT COUNT(*) FROM ccr_entries", [], |r| r.get(0))
            .unwrap_or(0)
    }
}

fn compute_key(content: &[u8]) -> String {
    blake3::hash(content).to_hex()[..24].to_string()
}

fn marker_for(hash: &str) -> String {
    format!("<<ccr:{hash}>>")
}

// ── Compression ───────────────────────────────────────────

enum CompressionMode { Cache, Token }

struct State {
    upstream_url: String,
    api_key: String,
    ccr: CcrStore,
    mode: CompressionMode,
}

fn compress(messages: &mut Vec<Value>, store: &CcrStore, mode: CompressionMode) {
    match mode {
        CompressionMode::Token => {
            for msg in messages.iter_mut() {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
                if role != "tool" { continue; }
                let content = match msg.get("content").and_then(|c| c.as_str()) {
                    Some(s) if s.len() > 500 => s.to_string(),
                    _ => continue,
                };
                let hash = compute_key(content.as_bytes());
                store.put(&hash, &content);
                msg["content"] = Value::String(marker_for(&hash));
            }
        }
        CompressionMode::Cache => {} // prefix-freeze — pass through
    }
}

// ── Main ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let upstream_url = std::env::var("DEEPSEEK_API_URL")
        .unwrap_or_else(|_| "https://api.deepseek.com/v1".into());
    let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
    let port: u16 = std::env::var("HEADROOM_HERMES_PORT")
        .unwrap_or_else(|_| "9797".into()).parse().unwrap_or(9797);
    let mode = match std::env::var("HEADROOM_MODE").as_deref() {
        Ok("token") => CompressionMode::Token,
        _ => CompressionMode::Cache,
    };
    let db_path = shellexpand::tilde("~/.hermes/headroom_cache.db").to_string();
    let ccr = CcrStore::open(&db_path);

    let state = Arc::new(State {
        upstream_url: upstream_url.clone(),
        api_key: api_key.clone(),
        ccr,
        mode,
    });

    tracing::info!("headroom-hermes :{port} → {upstream_url}  mode={mode:?}");

    let app = Router::new()
        .route("/v1/chat/completions", post(handler))
        .route("/health", get(|| async { serde_json::json!({"status":"healthy"}) }))
        .route("/stats", get({
            let s = state.clone();
            move || async move {
                serde_json::json!({"mode": format!("{:?}", s.mode), "ccr_entries": s.ccr.len()}).to_string()
            }
        }))
        .with_state(state.clone());

    // Launch Hermes
    let hermes_bin = std::env::args().nth(1).unwrap_or_else(|| "hermes".into());
    let hermes_args: Vec<String> = std::env::args().skip(2).collect();

    let mut hermes = Command::new(&hermes_bin)
        .args(&hermes_args)
        .env("OPENAI_BASE_URL", format!("http://127.0.0.1:{port}/v1"))
        .env("OPENAI_API_KEY", &api_key)
        .stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
        .spawn().expect("launch Hermes");

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();

    tokio::select! {
        _ = axum::serve(listener, app) => {},
        s = hermes.wait() => tracing::info!("Hermes: {s:?}"),
    }
}

// ── Handler ───────────────────────────────────────────────

async fn handler(State(s): State<Arc<State>>, req: axum::http::Request<Body>) -> impl IntoResponse {
    let body = match axum::body::to_bytes(req.into_body(), 50*1024*1024).await {
        Ok(b) => b, Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let mut payload: Value = match serde_json::from_slice(&body) {
        Ok(p) => p, Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if let Some(Value::Array(msgs)) = payload.get_mut("messages") {
        compress(msgs, &s.ccr, s.mode);
    }
    let client = reqwest::Client::new();
    match client.post(format!("{}/chat/completions", s.upstream_url))
        .header("Authorization", format!("Bearer {}", s.api_key))
        .json(&payload).send().await
    {
        Ok(r) => { let status = r.status(); let body = r.text().await.unwrap_or_default(); (status, body).into_response() }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}
