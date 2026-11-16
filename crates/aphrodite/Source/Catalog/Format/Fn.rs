//! Render the human-readable catalog table.

use super::Table;
use crate::State::Struct::AphroditeState;

/// Format the catalog as a markdown table (for LLM consumption).
///
/// The empty catalog is a different BODY, not a table with no rows: it says so and
/// returns, which is why emptiness is a binding rather than a test beside the
/// renderer. `Table::Fn::Fn` then owns the whole non-empty shape - header, rows and the
/// "N more entries" footer - so the two cases cannot drift into one another.
pub fn Fn(State:&AphroditeState) -> String {
	let Items = &State.Marker.Recent;

	let Some(Items) = (!Items.is_empty()).then_some(Items) else {
		return "No compressed items yet.".to_string();
	};

	Table::Fn::Fn(State, Items)
}
