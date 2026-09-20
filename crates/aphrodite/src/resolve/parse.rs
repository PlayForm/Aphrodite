/// CCR marker prefix/suffix
const CCR_PREFIX:&str = "<<<CCR:";
const CCR_SUFFIX:&str = ">>>";

/// Parse a CCR marker string to extract the hash.
/// Marker format: <<<CCR:hash|type|size>>>
pub(crate) fn parse_marker_hash(marker:&str) -> Option<String> {
	let inner = marker.strip_prefix(CCR_PREFIX)?.strip_suffix(CCR_SUFFIX)?;
	inner.split('|').next().map(|h| h.to_string())
}

/// Find all CCR markers in content. Returns (full_marker, hash) pairs.
pub(crate) fn find_markers(content:&str) -> Vec<(String, String)> {
	let mut markers = Vec::new();
	let mut search_from = 0;

	while let Some(start) = content[search_from..].find(CCR_PREFIX) {
		let abs_start = search_from + start;
		let after_prefix = abs_start + CCR_PREFIX.len();

		if let Some(end) = content[after_prefix..].find(CCR_SUFFIX) {
			let abs_end = after_prefix + end + CCR_SUFFIX.len();
			let full_marker = content[abs_start..abs_end].to_string();
			if let Some(hash) = parse_marker_hash(&full_marker) {
				markers.push((full_marker, hash));
			}
			search_from = abs_end;
		} else {
			// Unclosed marker - skip past this prefix
			search_from = after_prefix;
		}
	}

	markers
}
