//! The `/reload` handler: re-read configuration without a restart.
//!
//! Re-resolves the thresholds, re-applies the port/model overrides and swaps
//! them into the live `AppState` atomics, then reports what changed. This is
//! the only path that mutates write-once-adjacent configuration in place, so it
//! is also the one that must never leave the atomics half-applied.

use std::sync::{Arc, atomic::Ordering};

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::Proxy::{State::Struct::Struct as AppState, Threshold::Resolve::Fn::Fn as ResolveThresholds};

/// Hot-reload aphrodite.toml and apply compression config changes.
/// POST /reload - returns the newly loaded compression settings.
/// `POST /reload` - re-parse `aphrodite.toml` and apply its `[compression]`
/// thresholds to THIS listener's live `AppState` (report 07 F2/F4/T15) -
/// previously this endpoint parsed the file, echoed the values back, and
/// discarded them; a 200 response with `"reloaded": true` asserted a state
/// change that never happened. Other `[compression]` keys
/// (`EngineThresholdPct`, `CatalogMode`, `TerminalThreshold`) have no
/// consumer in this crate's proxy path (they're echoed for visibility, not
/// applied - see report 07 F8/F9 for their fate). `AutoExpand` /
/// `AutoExpandLimit` ARE applied here, mirroring the thresholds.
pub async fn Fn(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	let ConfigurationPath = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
	match crate::Configuration::Proxy::Multi::Struct::Struct::Load(&ConfigurationPath) {
		Ok(Configuration) => {
			// Issue #11 WS4: keep the preview cap in sync on hot-reload
			// (`[previews] preview_max_chars`).
			crate::Preview::State::Set::Fn(Configuration.Previews.as_ref().and_then(|p| p.PreviewMaximumChars));
			// ── TOML-driven marker block templates (pre-Rust port): keep the
			// process-global marker format/hint + `CCRMarkerHint` knob in
			// sync on hot-reload, same as the preview cap above.
			crate::Marker::Format::Const::SetMarkerTemplates(
				Configuration
					.Templates
					.as_ref()
					.and_then(|t| t.Marker.get("format"))
					.map(String::as_str),
				Configuration
					.Templates
					.as_ref()
					.and_then(|t| t.Marker.get("hint"))
					.map(String::as_str),
				Configuration.Prompts.as_ref().and_then(|p| p.CCRMarkerHint).unwrap_or(false),
			);
			// Pre-Rust port: keep `[templates]` preview families + the
			// `[previews] model_family` selector in sync on hot-reload, the
			// same way the cap is re-applied here.
			crate::Template::Family::Set::Fn::Fn(Configuration.Previews.as_ref().and_then(|p| p.ModelFamily.clone()));
			crate::Template::Set::Fn::Fn(Configuration.Templates.clone());
			let Comp = Configuration.Compression.as_ref();
			let Thresholds = ResolveThresholds(Comp);
			state.CacheCompressThreshold.store(Thresholds.Cache, Ordering::Relaxed);
			state.TokenCompressThreshold.store(Thresholds.Token, Ordering::Relaxed);
			state.InlineCCRThreshold.store(Thresholds.inline, Ordering::Relaxed);
			state
				.CodeMultiplierX100
				.store((Thresholds.CodeMultiplier * 100.0) as u64, Ordering::Relaxed);
			// Pre-Rust port: auto-expand knobs are live-tunable exactly like
			// the thresholds above - a `/reload` with `auto_expand = true`
			// switches the compress paths to inline resolution immediately,
			// and a `false` (or absent) value switches them back to markers.
			state.AutoExpand.store(Thresholds.AutoExpand, Ordering::Relaxed);
			state.AutoExpandLimit.store(Thresholds.AutoExpandLimit, Ordering::Relaxed);
			let body = serde_json::json!({
				"reloaded": true,
				"applied": true,
				"config": ConfigurationPath,
				"compression": {
					"tool_threshold_cache": Thresholds.Cache,
					"tool_threshold_token": Thresholds.Token,
					"inline_threshold": Thresholds.inline,
					"code_multiplier": Thresholds.CodeMultiplier,
					"auto_expand": Thresholds.AutoExpand,
					"auto_expand_limit": Thresholds.AutoExpandLimit,
					// Echoed for visibility; the live cap is write-once
					// (resolved at startup) - a reload cannot change it.
					"max_request_body_size": Thresholds.MaximumRequestBodySize,
				},
				// Parsed and visible, but not applied by this proxy (see doc
				// comment above).
				"parsed_only": {
					"terminal_threshold": Comp.and_then(|c| c.TerminalThreshold),
					"engine_threshold_pct": Comp.and_then(|c| c.EngineThresholdPct),
					"catalog_mode": Comp.and_then(|c| c.CatalogMode.clone()),
				}
			});
			tracing::info!(
				%ConfigurationPath,
				CacheThreshold = Thresholds.Cache,
				TokenThreshold = Thresholds.Token,
				InlineThreshold = Thresholds.inline,
				CodeMultiplier = Thresholds.CodeMultiplier,
				AutoExpand = Thresholds.AutoExpand,
				AutoExpandLimit = Thresholds.AutoExpandLimit,
				MaximumRequestBodySize = ?Thresholds.MaximumRequestBodySize,
				"config reloaded - compression thresholds applied"
			);
			(StatusCode::OK, Json(body)).into_response()
		},
		Err(e) => {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(serde_json::json!({"error": format!("failed to reload: {e}")})),
			)
				.into_response()
		},
	}
}

// ── Health check ────────────────────────────────────────────────────
