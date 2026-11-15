//! `Default` for the configuration struct - the "nothing is configured"
//! value.

use std::collections::HashMap;

/// An empty table with no overrides and no recorded failure.
///
/// This is the value every resolution chain terminates at, so it must stay
/// genuinely empty: seeding it with defaults would silently change what
/// "absent everywhere" resolves to.
impl Default for super::Struct::Struct {
	fn default() -> Self { Self { raw:toml::Table::new(), Overrides:HashMap::new(), ParseFailure:None } }
}
