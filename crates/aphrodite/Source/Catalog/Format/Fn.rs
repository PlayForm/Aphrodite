//! Render the human-readable catalog table.

use crate::{
	Catalog::Size::Fn::Fn as FmtSize,
	State::AphroditeState,
	StructExtract::Format::Floor::Fn as FloorBoundary,
};

/// Format catalog as markdown table (for LLM consumption).
pub fn Fn(State:&AphroditeState) -> String {
	let Items = &State.RecentMarkers;
	if Items.is_empty() {
		return "No compressed items yet.".to_string();
	}

	let mut Lines = vec![
		format!(
			"Catalog: {} items {} saved {} turns {} files",
			Items.len(),
			FmtSize(Items.iter().map(|m| m.Size).sum()),
			State.ConvIndex.len(),
			State.ReferencedFiles.len(),
		),
		String::new(),
		"| Hash | Type | Size | Preview |".to_string(),
		"|------|------|------|---------|".to_string(),
	];

	// Newest 20 by turn, newest first. A plain `iter().rev()` assumes
	// insertion order is chronological; sort defensively by turn so a
	// marker recorded out of order (e.g. a re-scan with an older turn)
	// can't displace genuinely newer entries from the visible window.
	// Stable sort keeps equal-turn markers in insertion order.
	let mut Newest:Vec<&crate::State::MarkerEntry> = Items.iter().collect();
	Newest.sort_by_key(|m| std::cmp::Reverse(m.Turn));

	for m in Newest.iter().take(20) {
		let hash = &m.hash[..10.min(m.hash.len())];
		let Size = FmtSize(m.Size);
		let Preview = m.Preview.replace('|', "\\|");
		let Preview = FloorBoundary(&Preview, 80);
		Lines.push(format!("| {} | {} | {} | {} |", hash, m.CCRType, Size, Preview));
	}

	// The header reports the full item count, but the table body caps at
	// 20 rows - say how many entries are not shown and where to list them.
	if Items.len() > 20 {
		Lines.push(format!(
			"{} more entries - use the aphrodite_catalog tool to list them all.",
			Items.len() - 20
		));
	}

	Lines.join("\n")
}
