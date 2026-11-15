//! JSON payloads and raw tool output: the top-level keys (capped at ten) and
//! an entry estimate. The key scan is a byte walk, not a regex, so it cannot
//! backtrack on adversarial input.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	// Extract unique top-level JSON keys via simple scan
	// Look for '"key_name":' patterns without regex
	let mut Keys:Vec<String> = Vec::new();

	let _ = Content.lines().try_for_each(|l| -> Result<(), ()> {
		// Find a sequence: '"' + some chars + '":'
		let bytes = l.as_bytes();
		let mut i = 0;

		loop {
			match () {
				_ if i + 3 >= bytes.len() => break,
				_ if bytes[i] == b'"' => {
					let Start = i + 1;
					let End = bytes[Start..]
						.iter()
						.position(|&b| b == b'"')
						.map(|p| Start + p)
						.unwrap_or(bytes.len());

					match End < bytes.len() && End + 2 < bytes.len() && bytes[End + 1] == b':' {
						true => {
							let Key = &l[Start..End];

							match !Key.starts_with('_') && !Keys.iter().any(|k| k == Key) {
								true => {
									Keys.push(Key.to_string());
								},
								false => {},
							}
						},
						false => {},
					}

					i = End + 1;

					match Keys.len() >= 10 {
						true => break,
						false => {},
					}
				},
				_ => {
					i += 1;
				},
			}
		}

		match Keys.len() >= 10 {
			true => Err(()),
			false => Ok(()),
		}
	});

	match !Keys.is_empty() {
		true => {
			Parts.push(format!("keys={}", Keys.join(",")));
		},
		false => {},
	}

	// Estimate entries count from array-like patterns
	let Entries = Content
		.lines()
		.filter(|l| {
			let t = l.trim();
			t.starts_with('{') || t.starts_with('"') || t.starts_with('[')
		})
		.count();

	match Entries > 1 {
		true => {
			Parts.push(format!("entries={}", Entries));
		},
		false => {},
	}
}
