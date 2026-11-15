//! Python source: functions, classes, imports and decorators.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	Parts.push("lang=py".to_string());
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("def ") || t.starts_with("async def ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t
				.strip_prefix("async def ")
				.unwrap_or_else(|| t.strip_prefix("def ").unwrap_or(t));
			After.split(['(', ' ', ':']).next().filter(|s| !s.is_empty())
		})
		.collect();
	if !Fns.is_empty() {
		Parts.push(format!("fns={}", Fns.join(",")));
	}
	let Classes:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("class ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t.strip_prefix("class ")?;
			After.split(['(', ' ', ':']).next().filter(|s| !s.is_empty())
		})
		.collect();
	if !Classes.is_empty() {
		Parts.push(format!("classes={}", Classes.join(",")));
	}
	let Imports:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("import ") || t.starts_with("from ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			if let Some(Rest) = t.strip_prefix("import ") {
				Rest.split([' ', ',', ';']).next().filter(|s| !s.is_empty())
			} else {
				t.strip_prefix("from ")?.split(' ').next().filter(|s| !s.is_empty())
			}
		})
		.collect();
	if !Imports.is_empty() {
		Parts.push(format!("imports={}", Imports.join(",")));
	}
	// Decorators: @route, @dataclass, @staticmethod, etc.
	let Decorators:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with('@')
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let name = &t[1..];
			name.split(['(', ' ']).next().filter(|s| !s.is_empty())
		})
		.collect();
	if !Decorators.is_empty() {
		Parts.push(format!("decorators={}", Decorators.join(",")));
	}
	Parts.push(format!("ln={}", LineCount));
}
