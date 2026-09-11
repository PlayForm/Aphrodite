//! TOML config loader - port of plugins/aphrodite/_core/config.py
//!
//! Priority: env var > aphrodite.toml > hardcoded default
//! Search paths: cwd, ~/.hermes/aphrodite/, relative to binary

use std::{collections::HashMap, path::PathBuf};

/// Config value resolution: env var → TOML → default
pub struct Config {
	raw: toml::Table,
	overrides: HashMap<String, String>,
}

impl Default for Config {
	fn default() -> Self {
		Self { raw: toml::Table::new(), overrides: HashMap::new() }
	}
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
					return Self { raw: table, overrides: HashMap::new() };
				}
			}
		}

		Self::default()
	}

	/// Reload from disk
	pub fn reload(&mut self) {
		*self = Self::load();
	}

	/// Load from an explicit TOML file path, bypassing `load()`'s search
	/// paths - used by `aphrodite_init` (01-F4/F9) so the handle-based C ABI
	/// init path shares this type's parsing/section/env-override logic
	/// instead of hand-parsing four `[compression]` keys directly (which had
	/// silently drifted from `apply_compression`'s own key names and never
	/// honored env var overrides at all).
	pub fn load_from(path: &str) -> Self {
		if let Ok(content) = std::fs::read_to_string(path) {
			if let Ok(table) = content.parse::<toml::Table>() {
				return Self { raw: table, overrides: HashMap::new() };
			}
		}
		Self::default()
	}

	/// Set a runtime override (equivalent to Python's _settings store)
	pub fn set_override(&mut self, key: &str, value: &str) {
		self.overrides.insert(key.to_string(), value.to_string());
	}

	/// Get a TOML section as a table, or empty if missing
	fn section(&self, name: &str) -> Option<&toml::Table> {
		self.raw.get(name).and_then(|v| v.as_table())
	}

	/// Resolve bool: override → env → toml[section][key] → default
	pub fn get_bool(&self, env_key: &str, section: &str, key: &str, default: bool) -> bool {
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
	pub fn get_u64(&self, env_key: &str, section: &str, key: &str, default: u64) -> u64 {
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
	pub fn get_usize(&self, env_key: &str, section: &str, key: &str, default: usize) -> usize {
		self.get_u64(env_key, section, key, default as u64) as usize
	}

	/// Resolve String
	pub fn get_string(&self, env_key: &str, section: &str, key: &str, default: &str) -> String {
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
	pub fn get_string_list(&self, section: &str, key: &str) -> Vec<String> {
		self.section(section)
			.and_then(|s| s.get(key))
			.and_then(|v| v.as_array())
			.map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default()
	}

	/// Load compression settings into an AphroditeState
	pub fn apply_compression(&self, state: &mut crate::state::AphroditeState) {
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

		// ── Flow-context assembler (05-P1/T5) ──
		// Hard cap for ALL per-turn injected context assembled by
		// `flow::build_turn_context`; kept well below Hermes's 10k-char spill
		// threshold so the catalog never gets spill-mangled.
		state.flow_budget_chars = self.get_usize("APHRODITE_FLOW_BUDGET_CHARS", "flow", "budget_chars", 4000);

		// ── Poll-worker auto-backgrounding ──
		state.poll_worker_enabled = self.get_bool("APHRODITE_POLL_WORKER", "compression", "poll_worker", true);

		// ── Directives ──
		// 01-F4: load whenever a directives/ dir exists, not gated on `active`
		// being non-empty - the shipped template default is `active = []`, so
		// gating on it meant `state.directives` stayed empty forever on a cold
		// start, making runtime discovery-then-activate
		// (`aphrodite_directive("add"|"swap", name)`) impossible unless the
		// user pre-activates at least one directive in TOML first. `active`
		// now only seeds which loaded directives start active.
		//
		// Namespace: every Aphrodite artifact under `~/.hermes/` lives under
		// `~/.hermes/aphrodite/` (never a bare `~/.hermes/<thing>` that could
		// collide with other tools). Directives therefore resolve from
		// `~/.hermes/aphrodite/directives`, not `~/.hermes/directives`.
		//
		// Built-in directives (baked into the binary via include_str!) are
		// used as fallbacks when no `directives/` directory exists on disk -
		// or when the on-disk directory is missing/unreadable - so a fresh
		// install (or a missing `~/.hermes/aphrodite/directives`) gets
		// shipped defaults without any filesystem setup and never errors.
		let home_aphrodite = dirs::home_dir()
			.unwrap_or_default()
			.join(".hermes")
			.join("aphrodite");
		// 2. binary-relative (portable install: shipped directives/ next to
		//    the executable, e.g. the Hermes plugin dir).
		let bin_relative = std::env::current_exe()
			.ok()
			.and_then(|p| p.parent().map(|d| d.join("directives")));
		// Candidate discovery order (first existing directory wins):
		//   0. $APHRODITE_DIRECTIVES_DIR - explicit env override, checked FIRST
		//      (issue #6): plugin/host setups export this so the plugin's own
		//      directives/ dir is found even though a ctypes-loaded dylib's
		//      current_exe points at the host process exe and cwd is rarely the
		//      plugin checkout.
		//   1. cwd (explicit, local override for dev/testing).
		//   2. home namespace (user-customizable): ~/.hermes/aphrodite/directives.
		//   3. binary-relative (portable install: shipped directives/ next to
		//      the executable, e.g. the Hermes plugin dir).
		let mut dirs: Vec<std::path::PathBuf> = Vec::new();
		if let Ok(env_dir) = std::env::var("APHRODITE_DIRECTIVES_DIR") {
			if !env_dir.trim().is_empty() {
				dirs.push(std::path::PathBuf::from(env_dir));
			}
		}
		dirs.push(std::path::PathBuf::from("directives"));
		dirs.push(home_aphrodite.join("directives"));
		if let Some(bin_dir) = bin_relative {
			dirs.push(bin_dir);
		}
		// Candidates that were probed but unusable, kept for the builtin
		// fallback's warn log - the old path failed silently.
		let mut probed_unusable: Vec<String> = Vec::new();
		let mut selected: Option<std::path::PathBuf> = None;
		for dir in dirs {
			let exists = dir.is_dir();
			tracing::info!(
				directive_source = "none",
				path = %dir.display(),
				exists = exists,
				"probing directives candidate"
			);
			if exists {
				// An existing directory wins even when it yields zero readable .md
				// files: `load_directives` returns an intentionally-empty map for that
				// case (an *existing* directory means an intentional directive set), so
				// the baked-in defaults must NOT be substituted here.
				let loaded = crate::directives::load_directives(&dir);
				tracing::info!(
					directive_source = "disk",
					path = %dir.display(),
					count = loaded.len(),
					"selected directives source: {} ({} directive(s) loaded)",
					dir.display(),
					loaded.len()
				);
				state.directives = loaded;
				selected = Some(dir);
				break;
			}
			probed_unusable.push(dir.display().to_string());
		}
		if selected.is_none() {
			// No usable directives/ directory found on disk - use baked-in defaults.
			// This is the defensive path: a missing `~/.hermes/aphrodite/directives`
			// (or an unreadable one) never breaks startup. Warns instead of failing
			// silently so a setup that expected a custom set learns it got defaults.
			tracing::warn!(
				directive_source = "builtins",
				probed = ?probed_unusable,
				"no usable directives directory found (probed: {}); falling back to built-in directives",
				probed_unusable.join(", ")
			);
			state.directives = crate::directives::loaded_builtins();
		}
		// Seed active directives: from TOML [directives] active list, filtered
		// to those that actually loaded. If the TOML list is empty AND we fell
		// back to builtins, default to focus + foresight + lazy (lazy keeps the
		// session from over-eagerly stacking directives until a later turn
		// proves it needs focus/explore/foresight/cleanup).
		let active = self.get_string_list("directives", "active");
		state.active_directives = active.into_iter().filter(|name| state.directives.contains_key(name)).collect();
		if state.active_directives.is_empty() && !state.directives.is_empty() {
			for name in ["focus", "foresight", "lazy"] {
				if state.directives.contains_key(name) {
					state.active_directives.push(name.to_string());
				}
			}
		}

		// ── First-turn session injection (templates.prompts.session_inject) ──
		// Loaded from the TOML's [prompts] section; defaults to the compiled-in
		// SHIPPED_SESSION_INJECT constant when the key is absent so a bare/minimal
		// config still gets a first-turn orientation. Empty string disables it.
		state.session_inject = self.get_string(
			"APHRODITE_SESSION_INJECT",
			"prompts",
			"session_inject",
			crate::flow::SHIPPED_SESSION_INJECT,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Tests that mutate the process cwd / env serialize on this guard -
	// `apply_compression`'s directives discovery reads cwd + env vars, so two
	// such tests running in parallel could observe each other's state.
	static CWD_GUARD: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

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
			raw: "[compression]\ntool_threshold_token = 321\n".parse().unwrap(),
			overrides: HashMap::new(),
		};
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.tool_threshold, 321);
	}

	// ── 05-T5 (P1): `[flow] budget_chars` (+ env override) resolves into
	// `state.flow_budget_chars`; default is 4000. ──
	#[test]
	fn test_flow_budget_from_toml() {
		let cfg = Config { raw: "[flow]\nbudget_chars = 1234\n".parse().unwrap(), overrides: HashMap::new() };
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.flow_budget_chars, 1234);

		// Env override wins over TOML.
		let mut cfg2 = Config::default();
		cfg2.set_override("APHRODITE_FLOW_BUDGET_CHARS", "555");
		let mut state2 = crate::state::AphroditeState::default();
		cfg2.apply_compression(&mut state2);
		assert_eq!(state2.flow_budget_chars, 555);

		// Absent everywhere => default 4000.
		let mut state3 = crate::state::AphroditeState::default();
		Config::default().apply_compression(&mut state3);
		assert_eq!(state3.flow_budget_chars, 4000);
	}

	// ── 05-T2 (P1/G3): directives load whenever a `directives/` dir exists,
	// independent of whether `[directives] active` is empty - the pre-05
	// gating on a non-empty `active` left `state.directives` empty on cold
	// start, making runtime discover-then-activate impossible. This test runs
	// from a temp cwd containing a `directives/` dir so it is hermetic. ──
	#[test]
	fn test_directives_loaded_even_when_active_empty() {
		let _g = CWD_GUARD.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();

		let tmp = std::env::temp_dir().join(format!(
			"aphrodite-cfg-directives-{}",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		std::fs::create_dir_all(tmp.join("directives")).unwrap();
		std::fs::write(tmp.join("directives").join("focus.md"), "# focus\nstay targeted").unwrap();

		let original = std::env::current_dir().unwrap();
		std::env::set_current_dir(&tmp).unwrap();

		// `active` is empty - directives must still load.
		let cfg = Config { raw: "[directives]\nactive = []\n".parse().unwrap(), overrides: HashMap::new() };
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);

		std::env::set_current_dir(&original).unwrap();
		let _ = std::fs::remove_dir_all(&tmp);

		assert!(
			state.directives.contains_key("focus"),
			"directives must load even when [directives] active is empty"
		);
		// With empty TOML `active` and directives loaded from disk, the
		// fallback seeds focus + foresight as defaults.
		assert!(
			!state.active_directives.is_empty(),
			"empty active list should seed focus + foresight defaults"
		);
		assert!(state.active_directives.contains(&"focus".to_string()));
	}

	// ── Issue #6: $APHRODITE_DIRECTIVES_DIR is the FIRST candidate and
	// must win over a cwd `directives/` dir. Shares CWD_GUARD with the other
	// cwd-mutating test since both set_current_dir. ──
	#[test]
	fn test_directives_env_override_wins_over_cwd() {
		let _g = CWD_GUARD.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();

		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let env_dir = std::env::temp_dir().join(format!("aphrodite-cfg-envdir-{stamp}"));
		let cwd_dir = std::env::temp_dir().join(format!("aphrodite-cfg-env-cwd-{stamp}"));
		std::fs::create_dir_all(&env_dir).unwrap();
		std::fs::create_dir_all(cwd_dir.join("directives")).unwrap();
		std::fs::write(env_dir.join("envwin.md"), "# envwin\nfrom env override").unwrap();
		std::fs::write(cwd_dir.join("directives").join("cwdwin.md"), "# cwdwin\nfrom cwd").unwrap();

		let original = std::env::current_dir().unwrap();
		std::env::set_current_dir(&cwd_dir).unwrap();
		std::env::set_var("APHRODITE_DIRECTIVES_DIR", &env_dir);

		let mut state = crate::state::AphroditeState::default();
		Config::default().apply_compression(&mut state);

		std::env::remove_var("APHRODITE_DIRECTIVES_DIR");
		std::env::set_current_dir(&original).unwrap();
		let _ = std::fs::remove_dir_all(&env_dir);
		let _ = std::fs::remove_dir_all(&cwd_dir);

		assert!(
			state.directives.contains_key("envwin"),
			"$APHRODITE_DIRECTIVES_DIR must win as the first candidate"
		);
		assert!(
			!state.directives.contains_key("cwdwin"),
			"the cwd directives/ candidate must not beat the env override"
		);
	}

	// ── Poll-worker config flag ──────────────────────────────

	#[test]
	fn test_poll_worker_enabled_default_true() {
		let cfg = Config::default();
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert!(state.poll_worker_enabled, "poll_worker must default to true");
	}

	#[test]
	fn test_poll_worker_disabled_via_env_override() {
		let mut cfg = Config::default();
		cfg.set_override("APHRODITE_POLL_WORKER", "false");
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert!(!state.poll_worker_enabled);
	}

	#[test]
	fn test_poll_worker_disabled_via_toml() {
		let cfg = Config {
			raw: "[compression]\npoll_worker = false\n".parse().unwrap(),
			overrides: HashMap::new(),
		};
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert!(!state.poll_worker_enabled);
	}
}
