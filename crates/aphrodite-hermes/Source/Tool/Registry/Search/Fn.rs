//! `aphrodite_search` - filter recorded markers by keyword and/or type.

use crate::Tool::{Arg, Dispatch::Fn::ToolHandler};

/// The `aphrodite_search` handler.
///
/// Matches the preview line and the CCR type only - never the stored bodies - and
/// caps at 20 results. Both limits are stated in the schema, because a caller that
/// reads a miss as "the content is gone" will not fall back to the catalogue.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_search", |Args| {
		let Query = Arg::Fn::Fn(Args, "query").to_lowercase();
		let TypeFilter = Args.get("type").and_then(|v| v.as_str());

		crate::Session::With::Fn::with_shared(|State| {
			let Results:Vec<serde_json::Value> = State
				.RecentMarkers
				.iter()
				.rev()
				.filter(|Marker| {
					let MatchesQuery = Query.is_empty()
						|| Marker.Preview.to_lowercase().contains(&Query)
						|| Marker.CCRType.to_lowercase().contains(&Query);
					let MatchesType = TypeFilter.is_none_or(|Kind| Marker.CCRType == Kind);

					MatchesQuery && MatchesType
				})
				.take(20)
				.map(|Marker| {
					serde_json::json!({
						"hash": &Marker.hash,
						"type": Marker.CCRType, "size": Marker.Size, "preview": Marker.Preview,
					})
				})
				.collect();

			serde_json::json!({"query": Query, "total": Results.len(), "results": Results})
		})
	})
}
