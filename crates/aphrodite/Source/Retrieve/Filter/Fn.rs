//! Keep only the lines of a payload that match a query.

/// Keep only lines of `Content` matching `Query` (case-insensitive substring),
/// or return `Content` unchanged if `Query` is `None`/empty.
///
/// The query is truncated to 512 chars BEFORE it is lowercased, so the cap
/// bounds the filter itself and not just a display string; it is cut on a
/// char boundary, never a raw byte slice, so a multi-byte query cannot panic.
pub fn Fn(Content:&str, Query:Option<&str>) -> String {
	match Query {
		Some(q) if !q.is_empty() => {
			// Truncate FIRST (char-boundary-safe, not a raw byte slice), then
			// derive q_lower from the truncated query - previously q_lower
			// came from the untruncated q while only the *display* string
			// was capped, so the 512-char cap never actually bounded the
			// filter itself (report 05 F2).
			let q = crate::StructExtract::Format::Floor::Fn(q, 512);
			let QLower = q.to_ascii_lowercase();
			let Filtered:Vec<&str> = Content
				.lines()
				.filter(|Line| Line.to_ascii_lowercase().contains(&QLower))
				.collect();
			match Filtered.is_empty() {
				true => format!("[no lines matching {:?} in {} lines]", q, Content.lines().count()),
				false => Filtered.join("\n"),
			}
		},
		_ => Content.to_string(),
	}
}
