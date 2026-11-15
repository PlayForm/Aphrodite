//! `aphrodite_search` - filter recorded markers by keyword and/or type.

use crate::Tool::{Arg, Dispatch::ToolHandler};

/// The `aphrodite_search` handler.
///
/// Matches the preview line and the CCR type only - never the stored bodies - and
/// caps at 20 results. Both limits are stated in the schema, because a caller that
/// reads a miss as "the content is gone" will not fall back to the catalogue.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_search", |args| {
		let query = Arg::Fn(args, "query").to_lowercase();
		let type_filter = args.get("type").and_then(|v| v.as_str());
		crate::with_shared(|state| {
			let results:Vec<serde_json::Value> = state
				.recent_markers
				.iter()
				.rev()
				.filter(|mk| {
					let q_ok = query.is_empty()
						|| mk.preview.to_lowercase().contains(&query)
						|| mk.ccr_type.to_lowercase().contains(&query);
					let t_ok = type_filter.is_none_or(|t| mk.ccr_type == t);
					q_ok && t_ok
				})
				.take(20)
				.map(|mk| {
					serde_json::json!({
						"hash": &mk.hash,
						"type": mk.ccr_type, "size": mk.size, "preview": mk.preview,
					})
				})
				.collect();
			serde_json::json!({"query": query, "total": results.len(), "results": results})
		})
	})
}
