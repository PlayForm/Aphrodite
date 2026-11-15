//! Query narrowing of resolved content.
//!
//! A retrieve may carry a query, and the answer the model gets back should be the
//! lines of the resolved content that mention it - not the whole blob. This
//! module owns that narrowing and nothing else. It is not a search engine and it
//! does not rank: it keeps the lines containing the query, in their original
//! order, case-insensitively.
//!
//! The failure that matters is a filter that hides content: a query with no match
//! must say so and hand the content back, never return an empty string that reads
//! as "the stored entry was empty".

/// Narrow content to the lines containing a query.
///
/// Case-insensitive substring match, one line per match, order preserved. An
/// empty query returns the content untouched; a query that matches nothing
/// returns the full content behind an `[aphrodite: no lines matched ...]` prefix,
/// because a filtered-to-nothing answer must stay distinguishable from an entry
/// that genuinely holds nothing.
///
/// It is not a tokenizer and not a regex engine: the query is matched literally,
/// so a query containing regex syntax matches that literal text and nothing else.
pub fn Fn(Content:&str, Query:&str) -> String {
	match Query.is_empty() {
		true => return Content.to_string(),
		false => (),
	}

	let QueryLower = Query.to_lowercase();
	let Matching:Vec<&str> = Content
		.lines()
		.filter(|Line| Line.to_lowercase().contains(&QueryLower))
		.collect();
	match Matching.is_empty() {
		true => format!("[aphrodite: no lines matched {Query:?} - returning full content]\n{Content}"),
		false => Matching.join("\n"),
	}
}
