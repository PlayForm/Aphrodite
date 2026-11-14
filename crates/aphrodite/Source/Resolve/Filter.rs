/// Filter content to lines containing the query string (case-insensitive).
/// Returns filtered lines, or original with prefix if no matches.
pub fn filter_lines(content:&str, query:&str) -> String {
	if query.is_empty() {
		return content.to_string();
	}
	let query_lower = query.to_lowercase();
	let matching:Vec<&str> = content
		.lines()
		.filter(|line| line.to_lowercase().contains(&query_lower))
		.collect();
	if matching.is_empty() {
		format!("[aphrodite: no lines matched {query:?} - returning full content]\n{content}")
	} else {
		matching.join("\n")
	}
}
