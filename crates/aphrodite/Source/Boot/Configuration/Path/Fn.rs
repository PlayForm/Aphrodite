//! Deciding which configuration file to read.

use aphrodite::Home;

use super::{Found, Resolved};

/// Resolve the config path and whether a config file was found there.
///
/// Three candidates, highest priority first:
///
/// 1. an explicit `APHRODITE_CONFIG_PATH`,
/// 2. `./aphrodite.toml` in the current directory,
/// 3. the runtime home's `aphrodite.toml`.
///
/// The third exists because `aphrodite setup` writes its generated config into
/// the runtime home, and a setup run from a directory without its own
/// `aphrodite.toml` used to fall straight through to CLI-parse mode, where the
/// templated ports, model and api_url were never read.
///
/// The chain is lazy: each candidate - including the environment read and the
/// runtime-home precedence chain behind candidate 3 - is evaluated only when
/// every earlier candidate has declined. A set `APHRODITE_CONFIG_PATH` therefore
/// costs one environment read and one stat, not four filesystem probes and a
/// `$HOME` walk whose answer is thrown away.
///
/// An explicit path is never redirected: an override that does not exist is
/// reported as `Absent` against that path - so the caller can surface the wrong
/// path - and never becomes a silent switch to a different file, because an
/// operator who named a path needs to hear that the path is wrong.
pub(crate) fn Fn() -> Resolved::Struct::Struct {
	// A candidate that only counts when the file is really there.
	let Existing = |Path:std::path::PathBuf| {
		std::fs::metadata(&Path).ok().map(|_| {
			Resolved::Struct::Struct { Path:Path.to_string_lossy().into_owned(), Found:Found::Enum::Enum::Present }
		})
	};

	let Explicit = std::env::var("APHRODITE_CONFIG_PATH").ok().map(|Path| {
		Resolved::Struct::Struct {
			Found:std::fs::metadata(&Path)
				.map(|_| Found::Enum::Enum::Present)
				.unwrap_or(Found::Enum::Enum::Absent),
			Path,
		}
	});

	Explicit
		.or_else(|| Existing(std::path::PathBuf::from("aphrodite.toml")))
		.or_else(|| Existing(Home::Configuration::Fn::Fn()))
		.unwrap_or_else(|| {
			Resolved::Struct::Struct { Path:String::from("aphrodite.toml"), Found:Found::Enum::Enum::Absent }
		})
}
