//! Go source: `func` names and the line count.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	Parts.push("lang=go".to_string());
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("func ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t.strip_prefix("func ")?;
			After.split(['(', ' ']).next().filter(|s| !s.is_empty())
		})
		.collect();

	match !Fns.is_empty() {
		true => {
			Parts.push(format!("fns={}", Fns.join(",")));
		},
		false => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
