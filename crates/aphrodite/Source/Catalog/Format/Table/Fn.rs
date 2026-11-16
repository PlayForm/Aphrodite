//! The markdown table body.

use std::collections::VecDeque;

use crate::{
	Catalog::Size::Fn::Fn as FmtSize,
	State::{Marker::Struct::Struct as MarkerEntry, Struct::AphroditeState},
	StructExtract::Format::Floor::Fn as FloorBoundary,
};

/// The header, the newest 20 rows and the "not shown" footer.
///
/// Newest 20 by turn, newest first. A plain `iter().rev()` assumes insertion order is
/// chronological; sorting by turn defensively means a marker recorded out of order (a
/// re-scan with an older turn) cannot displace genuinely newer entries from the visible
/// window. The sort is stable, so equal-turn markers keep insertion order.
///
/// The header reports the FULL item count while the body caps at 20 rows, so the
/// footer says how many entries are not shown and where to list them - an agent
/// reading a table that silently stopped at 20 would under-count the session.
pub(crate) fn Fn(State:&AphroditeState, Items:&VecDeque<MarkerEntry>) -> String {
	let mut Lines = vec![
		format!(
			"Catalog: {} items {} saved {} turns {} files",
			Items.len(),
			FmtSize(Items.iter().map(|Marker| Marker.Size).sum()),
			State.ConvIndex.len(),
			State.ReferencedFiles.len(),
		),
		String::new(),
		"| Hash | Type | Size | Preview |".to_string(),
		"|------|------|------|---------|".to_string(),
	];

	let mut Newest:Vec<&MarkerEntry> = Items.iter().collect();
	Newest.sort_by_key(|Marker| std::cmp::Reverse(Marker.Turn));

	Newest.iter().take(20).for_each(|Marker| {
		let hash = &Marker.hash[..10.min(Marker.hash.len())];
		let Size = FmtSize(Marker.Size);
		// A `|` in a preview would end the cell and shift every column after it.
		let Preview = Marker.Preview.replace('|', "\\|");
		let Preview = FloorBoundary(&Preview, 80);

		Lines.push(format!("| {} | {} | {} | {} |", hash, Marker.CCRType, Size, Preview));
	});

	Lines.extend((Items.len() > 20).then(|| {
		format!(
			"{} more entries - use the aphrodite_catalog tool to list them all.",
			Items.len() - 20
		)
	}));

	Lines.join("\n")
}
