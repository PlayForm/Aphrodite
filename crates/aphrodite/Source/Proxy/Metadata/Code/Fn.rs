//! Code of unknown language: any signature-looking line, labelled `gen`.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	Parts.push("lang=gen".to_string());
	// Try to extract function-like signatures from unknown code
	let Sigs:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("fn ")
				|| t.starts_with("def ")
				|| t.starts_with("func ")
				|| t.starts_with("function ")
				|| t.starts_with("class ")
				|| t.starts_with("struct ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t
				.strip_prefix("fn ")
				.or_else(|| t.strip_prefix("def "))
				.or_else(|| t.strip_prefix("func "))
				.or_else(|| t.strip_prefix("function "))
				.or_else(|| t.strip_prefix("class "))
				.or_else(|| t.strip_prefix("struct "))?;
			After.split(['(', ' ']).next()
		})
		.collect();

	match !Sigs.is_empty() {
		true => {
			Parts.push(format!("sigs={}", Sigs.join(",")));
		},
		false => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
