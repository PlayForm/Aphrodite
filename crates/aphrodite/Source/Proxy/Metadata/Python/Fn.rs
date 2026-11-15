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

	match !Fns.is_empty() {
		true => {
			Parts.push(format!("fns={}", Fns.join(",")));
		},
		false => {},
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

	match !Classes.is_empty() {
		true => {
			Parts.push(format!("classes={}", Classes.join(",")));
		},
		false => {},
	}

	let Imports:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("import ") || t.starts_with("from ")
		})
		.filter_map(|l| {
			let t = l.trim_start();

			match t.strip_prefix("import ") {
				Some(Rest) => Rest.split([' ', ',', ';']).next().filter(|s| !s.is_empty()),
				None => t.strip_prefix("from ")?.split(' ').next().filter(|s| !s.is_empty()),
			}
		})
		.collect();

	match !Imports.is_empty() {
		true => {
			Parts.push(format!("imports={}", Imports.join(",")));
		},
		false => {},
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

	match !Decorators.is_empty() {
		true => {
			Parts.push(format!("decorators={}", Decorators.join(",")));
		},
		false => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
