//! The engine type and its constructor.

use std::{path::PathBuf, sync::Mutex};

use crate::Scripting::Script::Struct::Struct as Script;

/// Rhai micro-script engine: loads `.rhai` files from the script dirs,
/// evaluates their hook functions against a hook payload, and degrades
/// (warn + pass-through) on any script error - never panics.
///
/// The script list sits behind a mutex because `Run` takes `&self` and the
/// reload path rewrites the list from another call site; the dirs are fixed at
/// construction. Both fields are crate-visible so the reload, check and test
/// leaves can operate on the same engine without a second copy of the list.
pub struct Struct {
	/// The loaded scripts, replaced wholesale by `Reload`.
	pub(crate) Scripts:Mutex<Vec<Script>>,
	/// Directories scanned by `Reload`, in precedence order.
	pub(crate) Directories:Vec<PathBuf>,
}

impl Struct {
	/// Create the engine and load every `.rhai` file from the default script
	/// dirs (`~/.hermes/aphrodite/scripts` and `scripts/aphrodite` relative to
	/// the cwd). Missing dirs are skipped silently.
	///
	/// The home dir is the platform user home rather than `Home::Runtime::Fn::Fn`:
	/// this is a pre-Rust compatibility surface whose directory is part of the
	/// documented layout, and moving it would orphan every script a user has
	/// already installed.
	pub fn new() -> Self {
		let Home = dirs::home_dir().unwrap_or_default();
		let Engine = Struct {
			Scripts:Mutex::new(Vec::new()),
			Directories:vec![
				Home.join(".hermes").join("aphrodite").join("scripts"),
				PathBuf::from("scripts/aphrodite"),
			],
		};
		Engine.Reload();
		Engine
	}
}

impl Default for Struct {
	/// `Default` is the same as [`Struct::new`]: an engine with the default
	/// script dirs and every loadable script already read. It exists because
	/// clippy requires it beside a zero-argument `new`, and it must not mean
	/// "empty engine" - a caller reaching for `Default` wants the loaded one.
	fn default() -> Self { Self::new() }
}
