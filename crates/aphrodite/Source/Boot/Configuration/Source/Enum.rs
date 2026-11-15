//! Where the configuration came from.

use aphrodite::Configuration::{CLI::Struct::Struct as CLI, Proxy::Multi::Struct::Struct as MultiConfiguration};

/// The one configuration the process runs on.
///
/// Before this existed the loader answered `(Option<MultiConfiguration>,
/// Option<CLI>, bool)` and assembly recovered the missing half with
/// `.expect("cli_fallback set when use_multi_config is false")` - an `expect`
/// that was sound only because a separate boolean happened to agree with the two
/// options. Three of the four combinations that type admits are meaningless
/// (`Some` and `Some`, `None` and `None`, and either one paired with the wrong
/// boolean), and every one of them compiled.
///
/// As a sum type the meaningless combinations cannot be written down: there is
/// exactly one value, it is either the file or the command line, and the
/// assembly step matches it instead of trusting a flag.
// Both payloads are configuration structs of comparable size, so neither variant
// is the outlier a `Box` would pay for; the difference clippy measures is the TOML
// table's fixed inline arrays.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Enum {
	/// A parsed `aphrodite.toml` with one `[[proxies]]` entry per listener.
	Multi(MultiConfiguration),
	/// No file was found, so the command line is the configuration.
	Standalone(CLI),
}
