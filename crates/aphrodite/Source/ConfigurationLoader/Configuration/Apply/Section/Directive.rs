//! `[directives]` - which directive set is loaded, and which of it starts
//! active.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Discover a directives directory, load it, and seed the active set.
///
/// An existing directory wins even when it yields zero readable `.md` files:
/// an existing directory is an intentional directive set, so the baked-in
/// defaults must NOT be substituted behind the operator's back. Only when no
/// candidate directory exists at all are the builtins used - and that path
/// warns with every probed location, because a setup that expected a custom
/// set and silently got defaults is unfixable from the outside.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── Directives ──
	// 01-F4: load whenever a directives/ dir exists, not gated on `Active`
	// being non-empty - the shipped template default is `active = []`, so
	// gating on it meant `state.directives` stayed empty forever on a cold
	// start, making runtime discovery-then-activate
	// (`aphrodite_directive("add"|"swap", name)`) impossible unless the
	// user pre-activates at least one directive in TOML first. `Active`
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
	let HomeAphrodite = crate::Home::Runtime::Fn::Fn();
	// 2. binary-relative (portable install: shipped directives/ next to
	//    the executable, e.g. the Hermes plugin dir).
	let BinRelative = std::env::current_exe()
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
	let _ = std::env::var("APHRODITE_DIRECTIVES_DIR")
		.ok()
		.filter(|EnvironmentDirectory| !EnvironmentDirectory.trim().is_empty())
		.map(|EnvironmentDirectory| dirs.push(std::path::PathBuf::from(EnvironmentDirectory)));

	dirs.push(std::path::PathBuf::from("directives"));
	dirs.push(HomeAphrodite.join("directives"));
	let _ = BinRelative.map(|BinDirectory| dirs.push(BinDirectory));
	// Candidates that were probed but unusable, kept for the builtin
	// fallback's warn log - the old path failed silently.
	let mut ProbedUnusable:Vec<String> = Vec::new();
	let mut Selected:Option<std::path::PathBuf> = None;
	for Directory in dirs {
		let Exists = Directory.is_dir();
		tracing::info!(
			DirectiveSource = "none",
			path = %Directory.display(),
			Exists = Exists,
			"probing directives candidate"
		);
		match Exists {
			true => {
				// An existing directory wins even when it yields zero readable .md
				// files: `LoadDirectives` returns an intentionally-empty map for that
				// case (an *existing* directory means an intentional directive set), so
				// the baked-in defaults must NOT be substituted here.
				let Loaded = crate::Directive::Load::Fn(&Directory);
				tracing::info!(
					DirectiveSource = "disk",
					path = %Directory.display(),
					Count = Loaded.len(),
					"selected directives source: {} ({} directive(s) loaded)",
					Directory.display(),
					Loaded.len()
				);
				State.Directives = Loaded;
				Selected = Some(Directory);
				break;
			},
			false => ProbedUnusable.push(Directory.display().to_string()),
		}
	}
	let _ = Selected.is_none().then(|| {
		// No usable directives/ directory found on disk - use baked-in defaults.
		// This is the defensive path: a missing `~/.hermes/aphrodite/directives`
		// (or an unreadable one) never breaks startup. Warns instead of failing
		// silently so a setup that expected a custom set learns it got defaults.
		tracing::warn!(
			DirectiveSource = "builtins",
			Probed = ?ProbedUnusable,
			"no usable directives directory found (probed: {}); falling back to built-in directives",
			ProbedUnusable.join(", ")
		);
		State.Directives = crate::Directive::Builtin::Fn();
	});
	// Seed active directives: from TOML [directives] active list, filtered
	// to those that actually loaded. If the TOML list resolves empty while
	// directives ARE loaded (from builtins or disk), default to the
	// focus + foresight + lazy subset that exists in the loaded set (lazy
	// keeps the session from over-eagerly stacking directives until a later
	// turn proves it needs focus/explore/foresight/cleanup).
	let Active = Configuration.GetStringList("directives", "active");
	State.ActiveDirectives = Active.into_iter().filter(|name| State.Directives.contains_key(name)).collect();
	let _ = (State.ActiveDirectives.is_empty() && !State.Directives.is_empty()).then(|| {
		State.ActiveDirectives.extend(
			["focus", "foresight", "lazy"]
				.into_iter()
				.filter(|name| State.Directives.contains_key(*name))
				.map(str::to_string),
		);
	});
}
