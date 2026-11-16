//! Parsing the config file (or the CLI) and registering what it configures.

use aphrodite::{
	Configuration::{
		CLI::Struct::Struct as CLI,
		Environment::Bool::Fn::Fn as Bool,
		Proxy::Multi::Struct::Struct as Multi,
	},
	ConfigurationLoader::Configuration::Apply::Global::{Publish, Rendering},
};
use clap::Parser;

use super::{
	super::{Path, Source},
	Loaded,
};

/// Load `aphrodite.toml` when one was found, otherwise parse the CLI.
///
/// The registrations in the file arm are process-global on purpose: the preview
/// builder, the marker builder and the template renderer have no
/// `AphroditeState` to read from, so the values are pushed into them once at
/// startup. A key declared in the config structs but never registered here is a
/// no-op knob - which is exactly what `previews.preview_max_chars` was before it
/// was wired.
///
/// The two arms are the two values `Path::Found::Enum` can hold, so "a file was
/// found" and "the CLI is the configuration" cannot both be true and neither can
/// be forgotten.
pub(crate) fn Fn(Resolved:&Path::Resolved::Struct::Struct) -> anyhow::Result<Loaded::Struct::Struct> {
	match Resolved.Found {
		Path::Found::Enum::Enum::Present => {
			let Configuration = Multi::Load(&Resolved.Path)?;

			// Publish every process-global rendering setting in one call: the
			// preview builder, the marker builder and the template renderer have
			// no `AphroditeState` to read from, so what the document says about
			// them is pushed into them once, here. A key declared in the config
			// structs but never published is a no-op knob - which is exactly what
			// `previews.preview_max_chars` was before it was wired.
			//
			// One call rather than four blocks: the preview cap, the marker
			// grammar and the template pair are one document's opinion, and
			// publishing them piecemeal is how the family and the templates could
			// be observed from two different files.
			Publish::Fn::Fn(&Rendering::Struct::Struct::from(&Configuration));

			Ok(Loaded::Struct::Struct {
				Source:Source::Enum::Enum::Multi(Configuration),
				LogCompact:Bool("APHRODITE_LOG_COMPACT"),
			})
		},
		Path::Found::Enum::Enum::Absent => {
			let CLI = CLI::parse();
			// Resolved before the source takes ownership of `CLI`: the flag has two
			// spellings on this path, `--log-compact` on the command line and the
			// environment variable the file path also honours, and either one turns it on.
			let LogCompact = CLI.LogCompact || Bool("APHRODITE_LOG_COMPACT");

			Ok(Loaded::Struct::Struct { Source:Source::Enum::Enum::Standalone(CLI), LogCompact })
		},
	}
}
