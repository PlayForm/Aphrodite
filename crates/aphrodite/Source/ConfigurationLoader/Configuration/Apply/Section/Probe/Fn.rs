//! One candidate directives directory.

use std::path::PathBuf;

use crate::State::Struct::AphroditeState;

/// Probe one candidate directory, adopting it when it exists.
///
/// An existing directory wins even when it yields zero readable `.md` files:
/// `Directive::Load` returns an intentionally-empty map for that case (an
/// *existing* directory means an intentional directive set), so the baked-in
/// defaults must NOT be substituted here. A directory that does not exist is
/// recorded for the fallback's warning, which is what makes the fallback
/// explainable from the outside.
pub(crate) fn Fn(Directory:&PathBuf, Unusable:&mut Vec<String>, State:&mut AphroditeState) -> Option<PathBuf> {
	let Exists = Directory.is_dir();

	tracing::info!(
		DirectiveSource = "none",
		path = %Directory.display(),
		Exists = Exists,
		"probing directives candidate"
	);

	Unusable.extend((!Exists).then(|| Directory.display().to_string()));

	Exists.then(|| Adopt(Directory, State))
}

/// Load the directory into the state and answer the path that was selected.
fn Adopt(Directory:&PathBuf, State:&mut AphroditeState) -> PathBuf {
	let Loaded = crate::Directive::Load::Fn(Directory);

	tracing::info!(
		DirectiveSource = "disk",
		path = %Directory.display(),
		Count = Loaded.len(),
		"selected directives source: {} ({} directive(s) loaded)",
		Directory.display(),
		Loaded.len()
	);

	State.Directives = Loaded;

	Directory.clone()
}
