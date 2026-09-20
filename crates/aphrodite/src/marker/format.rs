use std::collections::HashMap;

/// Build a CCR output block.
///
/// - `hash_val`: the content hash
/// - `ccr_type`: content type string (e.g. "code_rust", "build")
/// - `size`: original content size in bytes
/// - `preview`: the formatted preview string
/// - `headroom_budget`: optional token budget for truncation
/// - `meta`: optional metadata key-value pairs
/// - `center`: optional center annotation
pub fn ccr_marker(
	hash_val:&str,
	ccr_type:&str,
	size:usize,
	preview:&str,
	headroom_budget:Option<u32>,
	meta:Option<&HashMap<String, String>>,
	center:Option<&str>,
) -> String {
	// Sanitize preview: newlines → spaces (| is safe - the marker is on its own line).
	let mut safe = preview.replace(['\n', '\r'], " ").trim().to_string();
	// Strip control chars
	safe = safe.chars().filter(|c| *c >= ' ').collect();

	// Headroom budget truncation
	if let Some(budget) = headroom_budget {
		safe = if budget < 25 {
			safe.chars().take(30).collect()
		} else if budget < 50 {
			safe.chars().take(60).collect()
		} else if budget < 75 {
			safe.chars().take(100).collect()
		} else {
			safe
		};
	}

	// Metadata string
	let meta_str = if let Some(m) = meta {
		let parts:Vec<String> = m
			.iter()
			.filter_map(|(k, v)| {
				let sv = v.replace('|', "/").replace('\n', " ").trim().to_string();
				if sv.is_empty() { None } else { Some(format!("{}={}", k, sv)) }
			})
			.collect();
		let mut s = parts.join(";");
		if s.len() > 300 {
			s = format!("{}...", crate::struct_extract::floor_boundary(&s, 297));
		}
		s
	} else {
		String::new()
	};

	// Build marker using the standard template
	render_marker(&safe, ccr_type, &meta_str, center, hash_val, size)
}

/// Render the marker using the canonical three-line format.
///
/// Doubling-bug fix (report 09 §5): `build_preview` already returns a
/// self-describing, fully bracketed preview like `[text:53L 1913B]` or
/// `[git:5M 2A | src/x.rs]`. Re-wrapping that in `[{center_str}:{preview}]`
/// produced the visible `[text:[text:53L 1913B]]` doubling. When the preview
/// is already a `[label:...]`-shaped string, emit it verbatim on the preview
/// line; only wrap bare previews (e.g. cache-mode raw excerpts) in the
/// `[{center_str}:...]` frame. A preview is thus produced exactly once.
fn render_marker(preview:&str, ccr_type:&str, meta:&str, center:Option<&str>, hash:&str, size:usize) -> String {
	let center_str = center.unwrap_or(ccr_type);
	let meta_part = if meta.is_empty() { String::new() } else { format!("\n[meta:{}]", meta) };

	let preview_line = if is_self_bracketed_preview(preview) {
		preview.to_string()
	} else {
		format!("[{}:{}]", center_str, preview)
	};

	format!("<<<CCR:{}|{}|{}>>>\n{}{}", hash, ccr_type, size, preview_line, meta_part)
}

/// True when `preview` is already a self-describing `[label:...]` preview (as
/// produced by `build_preview`), so `render_marker` must not wrap it again.
/// Requires a leading `[`, a matching trailing `]`, and a `[word:` label head
/// (`\[\w+:`) so a bare excerpt that merely happens to start with `[` isn't
/// mistaken for a preview.
fn is_self_bracketed_preview(preview:&str) -> bool {
	let p = preview.trim();
	if !p.starts_with('[') || !p.ends_with(']') {
		return false;
	}
	let inner = &p[1..];
	match inner.find(':') {
		Some(colon) => {
			let label = &inner[..colon];
			!label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
		},
		None => false,
	}
}
