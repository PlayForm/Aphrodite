//! CCR Center  -  temporary LLM memory deposits.
//!
//! A center is a point where the LLM places information that converges
//! with content when it later flows through the pipeline. Like a
//! pincer movement: the LLM sets the center NOW, content arrives LATER,
//! both meet at the same understanding.
//!
//! Centers are embedded in CCR markers and survive retrievals.
//! They're simple strings  -  the LLM says what it means.

/// Parse a center string from the LLM.
/// Returns the content type override if the center names a known code type.
pub fn parse_center(s:&str) -> Option<&str> {
	match s {
		"" => None,
		s if s.starts_with("code_") => Some(s),
		"debug" | "verbose" => Some("debug"),
		"compact" | "summary" => Some("compact"),
		_ => None,
	}
}

/// Apply a center to content type detection.
/// If the LLM provided a code_* center, use it instead of auto-detection.
pub fn centered_content_type(center:Option<&str>, auto_detected:&str) -> String {
	match center {
		Some(c) if c.starts_with("code_") => c.to_string(),
		_ => auto_detected.to_string(),
	}
}

/// Apply a center to preview length.
/// "debug"/"verbose" → show more. "compact" → show less.
pub fn centered_preview_len(center:Option<&str>) -> usize {
	match center {
		Some("debug") | Some("verbose") => 500,
		Some("compact") | Some("summary") => 100,
		_ => 250,
	}
}
