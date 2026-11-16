//! One piece, rendered against the stats document.

use serde_json::Value;

use crate::Boot::Route::Metrics::{Entry, Histogram, Piece};

/// Render one piece: a counter line when the document holds its member, and
/// nothing at all when it does not.
pub fn Fn(Piece:Piece::Enum::Enum, Stats:&Value) -> String {
	match Piece {
		Piece::Enum::Enum::Entry(Counter) => Entry::Line::Fn::Fn(&Counter, Stats).unwrap_or_default(),
		Piece::Enum::Enum::Histogram => Histogram::Fn::Fn(Stats),
	}
}
