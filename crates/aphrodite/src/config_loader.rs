//! TOML config loader - port of plugins/aphrodite/_core/config.py
//!
//! Priority: env var > aphrodite.toml > hardcoded default
//! Search paths: cwd, ~/.hermes/aphrodite/, relative to binary

use std::{collections::HashMap, path::PathBuf};

/// Config value resolution: env var → TOML → default
pub struct Config {
	raw:toml::Table,
	overrides:HashMap<String, String>,
}

impl Default for Config {
	fn default() -> Self { Self { raw:toml::Table::new(), overrides:HashMap::new() } }
}

impl Config {
	/// Load from TOML file. Returns defaults on any failure.
	pub fn load() -> Self {
		let search_paths = vec![
			PathBuf::from("aphrodite.toml"),
			dirs::home_dir()
				.unwrap_or_default()
				.join(".hermes")
				.join("aphrodite")
				.join("aphrodite.toml"),
		];

		for path in &search_paths {
			if let Ok(content) = std::fs::read_to_string(path) {
				if let Ok(table) = content.parse::<toml::Table>() {
					return Self { raw:table, overrides:HashMap::new() };
				}
			}
		}

		Self::default()
	}

	/// Reload from disk
	pub fn reload(&mut self) { *self = Self::load(); }

	/// Load from an explicit TOML file path, bypassing `load()`'s search
	/// paths - used by `aphrodite_init` (01-F4/F9) so the handle-based C ABI
	/// init path shares this type's parsing/section/env-override logic
	/// instead of hand-parsing four `[compression]` keys directly (which had
	/// silently drifted from `apply_compression`'s own key names and never
	/// honored env var overrides at all).
	pub fn load_from(path:&str) -> Self {
		if let Ok(content) = std::fs::read_to_string(path) {
			if let Ok(table) = content.parse::<toml::Table>() {
				return Self { raw:table, overrides:HashMap::new() };
			}
		}
		Self::default()
	}

	/// Set a runtime override (equivalent to Python's _settings store)
	pub fn set_override(&mut self, key:&str, value:&str) { self.overrides.insert(key.to_string(), value.to_string()); }

	/// Get a TOML section as a table, or empty if missing
	fn section(&self, name:&str) -> Option<&toml::Table> { self.raw.get(name).and_then(|v| v.as_table()) }

	/// Resolve bool: override → env → toml[section][key] → default
	pub fn get_bool(&self, env_key:&str, section:&str, key:&str, default:bool) -> bool {
		if let Some(v) = self.overrides.get(env_key) {
			return v == "true" || v == "1";
		}
		if let Ok(v) = std::env::var(env_key) {
			return v == "true" || v == "1";
		}
		self.section(section)
			.and_then(|s| s.get(key))
			.and_then(|v| v.as_bool())
			.unwrap_or(default)
	}

	/// Resolve u64: override → env → toml[section][key] → default
	pub fn get_u64(&self, env_key:&str, section:&str, key:&str, default:u64) -> u64 {
		if let Some(v) = self.overrides.get(env_key) {
			return v.parse().unwrap_or(default);
		}
		if let Ok(v) = std::env::var(env_key) {
			return v.parse().unwrap_or(default);
		}
		self.section(section)
			.and_then(|s| s.get(key))
			.and_then(|v| v.as_integer())
			.map(|v| v as u64)
			.unwrap_or(default)
	}

	/// Resolve usize: same as u64 but for sizes
	pub fn get_usize(&self, env_key:&str, section:&str, key:&str, default:usize) -> usize {
		self.get_u64(env_key, section, key, default as u64) as usize
	}

	/// Resolve String
	pub fn get_string(&self, env_key:&str, section:&str, key:&str, default:&str) -> String {
		if let Some(v) = self.overrides.get(env_key) {
			return v.clone();
		}
		if let Ok(v) = std::env::var(env_key) {
			return v;
		}
		self.section(section)
			.and_then(|s| s.get(key))
			.and_then(|v| v.as_str())
			.map(|v| v.to_string())
			.unwrap_or_else(|| default.to_string())
	}

	/// Resolve a TOML array of strings.
	pub fn get_string_list(&self, section:&str, key:&str) -> Vec<String> {
		self.section(section)
			.and_then(|s| s.get(key))
			.and_then(|v| v.as_array())
			.map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default()
	}

	/// Load compression settings into an AphroditeState
	pub fn apply_compression(&self, state:&mut crate::state::AphroditeState) {
		state.context_engine_enabled = self.get_bool("APHRODITE_CONTEXT_ENGINE", "compression", "context_engine", true);
		state.engine_threshold_pct =
			self.get_u64("APHRODITE_ENGINE_THRESHOLD_PCT", "compression", "engine_threshold_pct", 45);
		state.engine_min_msgs = self.get_usize("APHRODITE_ENGINE_MIN_MSGS", "compression", "engine_min_msgs", 8);
		state.engine_protect_first =
			self.get_usize("APHRODITE_ENGINE_PROTECT_FIRST", "compression", "engine_protect_first", 2);
		state.engine_protect_last =
			self.get_usize("APHRODITE_ENGINE_PROTECT_LAST", "compression", "engine_protect_last", 5);
		// F3: was keyed "tool_threshold" and env var APHRODITE_TOOL_THRESHOLD -
		// neither exists in any shipped TOML/doc (they use
		// tool_threshold_token/tool_threshold_cache); wiring this up as-is
		// would always fall through to the default. The Hermes dylib path
		// is the tool-injecting, aggressive-compression environment (no
		// cache-vs-token split like the proxy's dual listeners), so it maps
		// to `tool_threshold_token`.
		state.tool_threshold =
			self.get_usize("APHRODITE_TOOL_THRESHOLD_TOKEN", "compression", "tool_threshold_token", 4096);
		state.terminal_threshold =
			self.get_usize("APHRODITE_TERMINAL_THRESHOLD", "compression", "terminal_threshold", 1024);
		state.model = self.get_string("APHRODITE_MODEL", "defaults", "model", "gpt-4o");
		state.api_url = self.get_string("APHRODITE_API_URL", "defaults", "api_url", "");

		// ── Directives ──
		// 01-F4: load whenever a directives/ dir exists, not gated on `active`
		// being non-empty - the shipped template default is `active = []`, so
		// gating on it meant `state.directives` stayed empty forever on a cold
		// start, making runtime discovery-then-activate
		// (`aphrodite_directive("add"|"swap", name)`) impossible unless the
		// user pre-activates at least one directive in TOML first. `active`
		// now only seeds which loaded directives start active.
		let dirs = vec![
			std::path::PathBuf::from("directives"),
			dirs::home_dir().unwrap_or_default().join(".hermes").join("directives"),
		];
		for dir in &dirs {
			if dir.is_dir() {
				state.directives = crate::directives::load_directives(dir);
				break;
			}
		}
		let active = self.get_string_list("directives", "active");
		state.active_directives = active.into_iter().filter(|name| state.directives.contains_key(name)).collect();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_defaults() {
		let cfg = Config::default();
		assert_eq!(cfg.get_u64("NONEXISTENT", "compression", "threshold", 42), 42);
		assert_eq!(cfg.get_bool("NONEXISTENT", "compression", "enabled", true), true);
		assert_eq!(cfg.get_string("NONEXISTENT", "defaults", "model", "gpt-4o"), "gpt-4o");
	}

	#[test]
	fn test_override() {
		let mut cfg = Config::default();
		cfg.set_override("APHRODITE_ENGINE_THRESHOLD_PCT", "90");
		assert_eq!(
			cfg.get_u64("APHRODITE_ENGINE_THRESHOLD_PCT", "compression", "engine_threshold_pct", 45),
			90
		);
	}

	// ── T16 (F3): `apply_compression` must read the TOML key/env var that
	// actually ships (`tool_threshold_token`/`APHRODITE_TOOL_THRESHOLD_TOKEN`)
	// - the old `tool_threshold`/`APHRODITE_TOOL_THRESHOLD` names exist in no
	// shipped TOML or doc, so wiring this up without the rename would have
	// silently resolved to the default forever. ──
	#[test]
	fn test_apply_compression_reads_tool_threshold_token_key() {
		let mut cfg = Config::default();
		cfg.set_override("APHRODITE_TOOL_THRESHOLD_TOKEN", "777");
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.tool_threshold, 777);
	}

	#[test]
	fn test_apply_compression_from_toml_table() {
		let cfg = Config {
			raw:"[compression]\ntool_threshold_token = 321\n".parse().unwrap(),
			overrides:HashMap::new(),
		};
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.tool_threshold, 321);
	}
}
