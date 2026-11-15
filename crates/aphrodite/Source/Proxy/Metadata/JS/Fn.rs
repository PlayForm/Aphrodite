//! JavaScript source: `function` and `const` bindings.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	Parts.push("lang=js".to_string());
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("function ") || t.starts_with("const ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			if let Some(Rest) = t.strip_prefix("function ") {
				Rest.split(['(', ' ']).next().filter(|s| !s.is_empty())
			} else {
				t.strip_prefix("const ")?
					.split([' ', '=', ':'])
					.next()
					.filter(|s| !s.is_empty())
			}
		})
		.collect();
	if !Fns.is_empty() {
		Parts.push(format!("fns={}", Fns.join(",")));
	}
	Parts.push(format!("ln={}", LineCount));
}
