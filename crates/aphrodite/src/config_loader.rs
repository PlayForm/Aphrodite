//! TOML config loader - port of plugins/aphrodite/_core/config.py
//!
//! Priority: env var > aphrodite.toml > hardcoded default
//! Search paths: cwd, then ~/.hermes/aphrodite/ - the config is NEVER
//! resolved relative to the binary (or the plugin dir).

use std::{collections::HashMap, path::PathBuf};

/// Config value resolution: env var → TOML → default
pub struct Config {
	raw:toml::Table,
	overrides:HashMap<String, String>,
	/// Set when a found `aphrodite.toml` failed to parse and the loader
	/// fell back to defaults - carried into `AphroditeState::config_error`
	/// so `aphrodite_stats` can surface "defaults in effect (parse failed)"
	/// and the failure is never indistinguishable from "config not set".
	pub parse_failure:Option<String>,
}

impl Default for Config {
	fn default() -> Self { Self { raw:toml::Table::new(), overrides:HashMap::new(), parse_failure:None } }
}

/// Emit a parse-failure warning that actually reaches a log surface.
///
/// `tracing::warn!` is a silent no-op while no subscriber is installed -
/// exactly the Hermes dylib path (the host is a Python process that never
/// installs a Rust tracing subscriber) and the engine binary before
/// `main()` initializes one (config loads first). Fall back to stderr so a
/// found-but-broken config is never invisible. The caller additionally
/// records the failure on the returned `Config` (`parse_failure`) for the
/// `aphrodite_stats` self-diagnosis surface.
fn warn_parse_failure(path:&std::path::Path, error:&toml::de::Error, action:&str) {
	if tracing::dispatcher::has_been_set() {
		tracing::warn!(
			path = %path.display(),
			error = %error,
			"aphrodite.toml found but failed to parse; {}",
			action
		);
	} else {
		eprintln!(
			"[aphrodite] aphrodite.toml found but failed to parse; {} ({})",
			action,
			path.display()
		);
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

		let mut parse_failure:Option<String> = None;
		for path in &search_paths {
			if let Ok(content) = std::fs::read_to_string(path) {
				match content.parse::<toml::Table>() {
					Ok(table) => return Self { raw:table, overrides:HashMap::new(), parse_failure:None },
					Err(err) => {
						// Fix 22 (inspection): a found-but-broken TOML file
						// used to fall through silently to the next search
						// path (or defaults) - warn so a broken local
						// aphrodite.toml is not ignored without indication.
						// `parse_failure` is only recorded when the search
						// ends on defaults (a later path may still yield a
						// valid config).
						warn_parse_failure(path, &err, "skipping");
						parse_failure = Some(format!("{}: {err}", path.display()));
					},
				}
			}
			// File not found/unreadable is the normal search miss - keep
			// looking.
		}

		Self { raw:toml::Table::new(), overrides:HashMap::new(), parse_failure }
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
			match content.parse::<toml::Table>() {
				Ok(table) => return Self { raw:table, overrides:HashMap::new(), parse_failure:None },
				Err(err) => {
					// Fix 22 (inspection): same warn-on-parse-error treatment
					// as `load()` - the explicit-path init used to silently
					// fall back to defaults on a broken file. `parse_failure`
					// is recorded so `aphrodite_stats` can self-diagnose.
					warn_parse_failure(std::path::Path::new(path), &err, "using defaults");
					return Self {
						raw:toml::Table::new(),
						overrides:HashMap::new(),
						parse_failure:Some(format!("{path}: {err}")),
					};
				},
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
		// Fix 21 (inspection): values are matched case-insensitively -
		// `TRUE`/`True`/`tRuE` previously resolved to false because the
		// exact lowercase comparison missed them. Only `true`/`1` (after
		// ASCII-lowercasing) still count as true; `YES`/`on` stay false.
		if let Some(v) = self.overrides.get(env_key) {
			let v = v.to_ascii_lowercase();
			return v == "true" || v == "1";
		}
		if let Ok(v) = std::env::var(env_key) {
			let v = v.to_ascii_lowercase();
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
		// Parse-failure self-diagnosis: a found-but-broken aphrodite.toml
		// falls back to defaults - surface it in `aphrodite_stats` so
		// "defaults in effect" is never indistinguishable from "config not
		// set" (the tracing warn alone is a no-op in the Hermes dylib).
		state.config_error = self.parse_failure.clone();
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

		// ── Fine-grained chain splitting ──
		// Default OFF for release: the segment markers (`echo __APHRODITE_SEG__`)
		// pollute captured stdout when commands are redirected to files, and
		// every split also adds marker lines to live tool output. Opt in per
		// session with APHRODITE_CHAIN_SPLIT=1 (or TOML chain_split = true).
		state.chain_split_enabled = self.get_bool("APHRODITE_CHAIN_SPLIT", "compression", "chain_split", false);

		// ── Tier 1 teaching loop: adaptive split threshold ──
		// `chain_split_min_segments` is the initial threshold (floor). The
		// threshold adapts within [floor, max] from the retrieval ratio of
		// split segments; see `adapt_chain_split_threshold`.
		state.chain_split_min_segments = self
			.get_usize(
				"APHRODITE_CHAIN_SPLIT_MIN_SEGMENTS",
				"compression",
				"chain_split_min_segments",
				2,
			)
			.max(2);
		state.chain_split_floor = state.chain_split_min_segments;
		state.chain_split_max_segments = self
			.get_usize(
				"APHRODITE_CHAIN_SPLIT_MAX_SEGMENTS",
				"compression",
				"chain_split_max_segments",
				6,
			)
			.max(state.chain_split_min_segments);

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
		let home_aphrodite = dirs::home_dir().unwrap_or_default().join(".hermes").join("aphrodite");
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
		let mut dirs:Vec<std::path::PathBuf> = Vec::new();
		if let Ok(env_dir) = std::env::var("APHRODITE_DIRECTIVES_DIR")
			&& !env_dir.trim().is_empty()
		{
			dirs.push(std::path::PathBuf::from(env_dir));
		}
		dirs.push(std::path::PathBuf::from("directives"));
		dirs.push(home_aphrodite.join("directives"));
		if let Some(bin_dir) = bin_relative {
			dirs.push(bin_dir);
		}
		// Candidates that were probed but unusable, kept for the builtin
		// fallback's warn log - the old path failed silently.
		let mut probed_unusable:Vec<String> = Vec::new();
		let mut selected:Option<std::path::PathBuf> = None;
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
		// to those that actually loaded. If the TOML list resolves empty while
		// directives ARE loaded (from builtins or disk), default to the
		// focus + foresight + lazy subset that exists in the loaded set (lazy
		// keeps the session from over-eagerly stacking directives until a later
		// turn proves it needs focus/explore/foresight/cleanup).
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

	/// Load preview settings into the process-global preview builder.
	/// `[previews] preview_max_chars` (env override:
	/// `APHRODITE_PREVIEW_MAX_CHARS`) caps the rendered preview string in
	/// chars; absent/0 = unlimited (legacy behavior). Issue #11 WS4: the key
	/// existed in the config structs but was never read anywhere - the
	/// preview builder now enforces it on every path (proxy, hooks, Hermes
	/// dylib), and this is the dylib-side wiring (the engine binary reads
	/// `MultiConfig.previews` directly in `main.rs`).
	pub fn apply_previews(&self) {
		let max = self.get_u64("APHRODITE_PREVIEW_MAX_CHARS", "previews", "preview_max_chars", 0);
		crate::preview::set_preview_max_chars(if max == 0 { None } else { Some(max.min(u32::MAX as u64) as u32) });
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Tests that mutate the process cwd / env serialize on this guard -
	// `apply_compression`'s directives discovery reads cwd + env vars, so two
	// such tests running in parallel could observe each other's state.
	static CWD_GUARD:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

	#[test]
	fn test_defaults() {
		let cfg = Config::default();
		assert_eq!(cfg.get_u64("NONEXISTENT", "compression", "threshold", 42), 42);
		assert!(cfg.get_bool("NONEXISTENT", "compression", "enabled", true));
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
			parse_failure:None,
		};
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.tool_threshold, 321);
	}

	// ── 05-T5 (P1): `[flow] budget_chars` (+ env override) resolves into
	// `state.flow_budget_chars`; default is 4000. ──
	#[test]
	fn test_flow_budget_from_toml() {
		let cfg = Config { raw:"[flow]\nbudget_chars = 1234\n".parse().unwrap(), overrides:HashMap::new(), parse_failure:None };
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
		let cfg = Config { raw:"[directives]\nactive = []\n".parse().unwrap(), overrides:HashMap::new(), parse_failure:None };
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);

		std::env::set_current_dir(&original).unwrap();
		let _ = std::fs::remove_dir_all(&tmp);

		assert!(
			state.directives.contains_key("focus"),
			"directives must load even when [directives] active is empty"
		);
		// With empty TOML `active` and directives loaded from disk, the
		// fallback seeds whatever of [focus, foresight, lazy] exists in the
		// loaded set - this temp dir only has focus.md, so focus is seeded.
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
		unsafe { std::env::set_var("APHRODITE_DIRECTIVES_DIR", &env_dir) };

		let mut state = crate::state::AphroditeState::default();
		Config::default().apply_compression(&mut state);

		unsafe { std::env::remove_var("APHRODITE_DIRECTIVES_DIR") };
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
			raw:"[compression]\npoll_worker = false\n".parse().unwrap(),
			overrides:HashMap::new(),
			parse_failure:None,
		};
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert!(!state.poll_worker_enabled);
	}

	// ── Issue #11 WS4: `[previews] preview_max_chars` must reach the
	// preview builder (the key was declared-but-unread dead config). ──
	#[test]
	fn test_apply_previews_wires_preview_max_chars() {
		let _g = crate::preview::preview_cap_test_guard();
		// Hermetic: a stray APHRODITE_PREVIEW_MAX_CHARS in the session env
		// (e.g. left by a manual cap probe) would win the precedence chain
		// and break every assert - and the leaked cap would poison every
		// later preview test. Remove it for the duration of this test.
		let env_backup = std::env::var("APHRODITE_PREVIEW_MAX_CHARS").ok();
		unsafe { std::env::remove_var("APHRODITE_PREVIEW_MAX_CHARS") };
		// TOML wins over the default (unlimited).
		let cfg = Config {
			raw:"[previews]\npreview_max_chars = 77\n".parse().unwrap(),
			overrides:HashMap::new(),
			parse_failure:None,
		};
		cfg.apply_previews();
		assert_eq!(crate::preview::preview_max_chars(), 77);

		// Env override wins over TOML.
		let mut cfg2 = Config::default();
		cfg2.set_override("APHRODITE_PREVIEW_MAX_CHARS", "123");
		cfg2.apply_previews();
		assert_eq!(crate::preview::preview_max_chars(), 123);

		// Absent/0 -> unlimited (legacy behavior), and restore the global.
		Config::default().apply_previews();
		assert_eq!(crate::preview::preview_max_chars(), 0);
		if let Some(v) = env_backup {
			unsafe { std::env::set_var("APHRODITE_PREVIEW_MAX_CHARS", v) };
		}
	}

	// ── Fix 21 (inspection): `get_bool` must accept uppercase/mixed-case
	// `TRUE`/`True`/`tRuE` and `1` through BOTH the overrides map and the
	// environment path - the old exact lowercase comparison silently
	// resolved them to false. Only `true`/`1` (after ASCII-lowercasing)
	// count as true; `YES`/`on`/`0`/`FALSE` stay false. ──
	#[test]
	fn test_get_bool_override_case_insensitive() {
		let mut cfg = Config::default();
		for (value, expected) in [
			("TRUE", true),
			("True", true),
			("tRuE", true),
			("1", true),
			("YES", false),
			("on", false),
			("0", false),
			("FALSE", false),
		] {
			cfg.set_override("APHRODITE_TEST_BOOL_CASE", value);
			assert_eq!(
				cfg.get_bool("APHRODITE_TEST_BOOL_CASE", "compression", "enabled", true),
				expected,
				"override value {value:?} must resolve to {expected}"
			);
		}
	}

	#[test]
	fn test_get_bool_env_case_insensitive() {
		// Env-mutating tests serialize on CWD_GUARD (same rationale as the
		// directives tests below).
		let _g = CWD_GUARD.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap();
		const KEY:&str = "APHRODITE_TEST_BOOL_CASE_ENV";
		// Hermetic (Rule 5): back up any prior value, remove it, set per
		// case, then remove and restore - a stray value must never leak
		// into this suite or later runs.
		let backup = std::env::var(KEY).ok();
		unsafe { std::env::remove_var(KEY) };

		let cfg = Config::default();
		for (value, expected) in [
			("TRUE", true),
			("True", true),
			("tRuE", true),
			("1", true),
			("YES", false),
			("on", false),
			("0", false),
			("FALSE", false),
		] {
			unsafe { std::env::set_var(KEY, value) };
			assert_eq!(
				cfg.get_bool(KEY, "compression", "enabled", true),
				expected,
				"env value {value:?} must resolve to {expected}"
			);
		}

		unsafe { std::env::remove_var(KEY) };
		if let Some(v) = backup {
			unsafe { std::env::set_var(KEY, v) };
		}
	}

	// ── Fix 22 (inspection): a found-but-broken TOML file must emit a
	// tracing warn (path + parse error) instead of silently falling through
	// to the next search path / defaults; resolution behavior itself is
	// unchanged (broken => defaults, valid => parsed, missing => defaults). ──

	/// Minimal tracing subscriber that records WARN event text, so a test
	/// can assert a `tracing::warn!` actually fired.
	struct WarnCapture {
		tx:std::sync::mpsc::Sender<String>,
	}

	impl tracing::Subscriber for WarnCapture {
		fn enabled(&self, _m:&tracing::Metadata<'_>) -> bool { true }
		fn new_span(&self, _s:&tracing::span::Attributes<'_>) -> tracing::span::Id { tracing::span::Id::from_u64(1) }
		fn record(&self, _span:&tracing::span::Id, _values:&tracing::span::Record<'_>) {}
		fn record_follows_from(&self, _span:&tracing::span::Id, _follows:&tracing::span::Id) {}
		fn enter(&self, _span:&tracing::span::Id) {}
		fn exit(&self, _span:&tracing::span::Id) {}
		fn event(&self, e:&tracing::Event<'_>) {
			if *e.metadata().level() != tracing::Level::WARN {
				return;
			}
			let mut s = String::new();
			struct Rec<'a>(&'a mut String);
			impl tracing::field::Visit for Rec<'_> {
				fn record_debug(&mut self, _f:&tracing::field::Field, v:&dyn std::fmt::Debug) {
					self.0.push_str(&format!("{v:?} "));
				}
			}
			e.record(&mut Rec(&mut s));
			let _ = self.tx.send(s);
		}
	}

	#[test]
	fn test_load_from_broken_toml_warns_and_returns_defaults() {
		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let broken = std::env::temp_dir().join(format!("aphrodite-cfg-broken-{stamp}.toml"));
		std::fs::write(&broken, "[compression\nenabled = false\n").unwrap();

		let (tx, rx) = std::sync::mpsc::channel();
		let cfg = tracing::subscriber::with_default(WarnCapture { tx }, || Config::load_from(broken.to_str().unwrap()));

		let _ = std::fs::remove_file(&broken);
		// Resolution behavior unchanged: a broken file yields defaults.
		assert!(cfg.get_bool("NONEXISTENT", "compression", "enabled", true));
		// ...and the warn fired, naming the path and the parse failure.
		let msg = rx
			.recv_timeout(std::time::Duration::from_secs(5))
			.expect("warn must be emitted for a broken TOML file");
		assert!(msg.contains("failed to parse"), "warn must mention the parse failure: {msg}");
		assert!(
			msg.contains(broken.to_str().unwrap()),
			"warn must include the offending path: {msg}"
		);
	}

	#[test]
	fn test_load_from_valid_and_missing() {
		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let valid = std::env::temp_dir().join(format!("aphrodite-cfg-valid-{stamp}.toml"));
		std::fs::write(&valid, "[compression]\nenabled = false\n").unwrap();
		let cfg = Config::load_from(valid.to_str().unwrap());
		let _ = std::fs::remove_file(&valid);
		assert!(
			!cfg.get_bool("NONEXISTENT", "compression", "enabled", true),
			"a valid file must parse and win over the default"
		);

		// Missing path: defaults, no panic.
		let cfg = Config::load_from(&format!("/nonexistent/aphrodite-{stamp}.toml"));
		assert!(cfg.get_bool("NONEXISTENT", "compression", "enabled", true));
	}

	// ── Parse-failure self-diagnosis: a broken file must record itself on
	// the Config (and via `apply_compression` on the state) so
	// `aphrodite_stats` can surface "defaults in effect (parse failed)" -
	// the tracing warn is a no-op in the Hermes dylib (no subscriber), so
	// the recorded field is the only guaranteed-visible diagnostic. ──
	#[test]
	fn test_parse_failure_recorded_on_config_and_state() {
		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let broken = std::env::temp_dir().join(format!("aphrodite-cfg-broken-recorded-{stamp}.toml"));
		std::fs::write(&broken, "[compression\nenabled = false\n").unwrap();

		let cfg = Config::load_from(broken.to_str().unwrap());
		let _ = std::fs::remove_file(&broken);

		let pf = cfg.parse_failure.as_ref().expect("parse failure must be recorded");
		assert!(
			pf.contains(broken.to_str().unwrap()),
			"recorded failure must name the broken path: {pf}"
		);
		assert!(pf.contains("TOML parse error"), "recorded failure must carry the parse error: {pf}");

		// `aphrodite_stats` self-diagnosis surface.
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		let ce = state.config_error.as_deref().expect("config_error must be set on the state");
		assert!(
			ce.contains(broken.to_str().unwrap()),
			"config_error must name the broken path: {ce}"
		);
	}

	// ── Multibyte content in comments is valid TOML: an em dash inside a
	// comment must parse and resolve - the fallback-to-defaults trigger is
	// a *broken* file, never non-ASCII bytes (report claim check). ──
	#[test]
	fn test_load_from_multibyte_comment_parses() {
		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let path = std::env::temp_dir().join(format!("aphrodite-cfg-multibyte-{stamp}.toml"));
		std::fs::write(
			&path,
			"[compression]\nterminal_threshold = 16384   # bytes; was default 1024 - raised\n",
		)
		.unwrap();
		let cfg = Config::load_from(path.to_str().unwrap());
		let _ = std::fs::remove_file(&path);

		assert!(
			cfg.parse_failure.is_none(),
			"a multibyte comment must not fail the parse: {:?}",
			cfg.parse_failure
		);
		let mut state = crate::state::AphroditeState::default();
		cfg.apply_compression(&mut state);
		assert_eq!(state.terminal_threshold, 16384);
		assert!(state.config_error.is_none());
	}
}
