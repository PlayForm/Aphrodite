//! JSON payloads and raw tool output: the top-level keys (capped at ten) and
//! an entry estimate. The key scan is a byte walk, not a regex, so it cannot
//! backtrack on adversarial input.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	// Extract unique top-level JSON keys via simple scan
	// Look for '"key_name":' patterns without regex
	let mut Keys:Vec<String> = Vec::new();
	for l in Content.lines() {
		// Find a sequence: '"' + some chars + '":'
		let bytes = l.as_bytes();
		let mut i = 0;
		while i + 3 < bytes.len() {
			if bytes[i] == b'"' {
				let Start = i + 1;
				let mut End = Start;
				while End < bytes.len() && bytes[End] != b'"' {
					End += 1;
				}
				if End < bytes.len() && End + 2 < bytes.len() && bytes[End + 1] == b':' {
					let Key = &l[Start..End];
					if !Key.starts_with('_') && !Keys.iter().any(|k| k == Key) {
						Keys.push(Key.to_string());
						if Keys.len() >= 10 {
							break;
						}
					}
				}
				i = End + 1;
			} else {
				i += 1;
			}
		}
		if Keys.len() >= 10 {
			break;
		}
	}
	if !Keys.is_empty() {
		Parts.push(format!("keys={}", Keys.join(",")));
	}
	// Estimate entries count from array-like patterns
	let Entries = Content
		.lines()
		.filter(|l| {
			let t = l.trim();
			t.starts_with('{') || t.starts_with('"') || t.starts_with('[')
		})
		.count();
	if Entries > 1 {
		Parts.push(format!("entries={}", Entries));
	}
}
