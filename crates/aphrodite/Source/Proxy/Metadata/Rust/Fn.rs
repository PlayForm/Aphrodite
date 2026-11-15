//! Rust source: the `fn`/`struct`/`impl`/`trait` names a reader
//! needs to navigate the file, plus its line count.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	Parts.push("lang=rs".to_string());
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("async fn ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let AfterFn = t
				.strip_prefix("pub async fn ")
				.or_else(|| t.strip_prefix("pub fn "))
				.or_else(|| t.strip_prefix("async fn "))
				.or_else(|| t.strip_prefix("fn "))?;
			AfterFn.split(['(', ' ', '<']).next().filter(|s| !s.is_empty())
		})
		.collect();
	match !Fns.is_empty() {
		true => {
			Parts.push(format!("fns={}", Fns.join(",")));
		},
		false => {},
	}

	let Structs:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("struct ") || t.starts_with("pub struct ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t
				.strip_prefix("pub struct ")
				.unwrap_or_else(|| t.strip_prefix("struct ").unwrap_or(t));
			After.split(['(', ' ', '<', '{']).next().filter(|s| !s.is_empty())
		})
		.collect();
	match !Structs.is_empty() {
		true => {
			Parts.push(format!("structs={}", Structs.join(",")));
		},
		false => {},
	}

	// impl blocks: impl TypeName or impl Trait for TypeName
	let Impls:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("impl ") || t.starts_with("pub impl ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t
				.strip_prefix("pub impl ")
				.unwrap_or_else(|| t.strip_prefix("impl ").unwrap_or(t));
			After.split_whitespace().next().map(|w| w.trim_end_matches('<'))
		})
		.collect();
	match !Impls.is_empty() {
		true => {
			Parts.push(format!("impls={}", Impls.join(",")));
		},
		false => {},
	}

	let Traits:Vec<&str> = Content
		.lines()
		.filter(|l| {
			let t = l.trim_start();
			t.starts_with("trait ") || t.starts_with("pub trait ")
		})
		.filter_map(|l| {
			let t = l.trim_start();
			let After = t
				.strip_prefix("pub trait ")
				.unwrap_or_else(|| t.strip_prefix("trait ").unwrap_or(t));
			After.split([' ', '<', '{']).next().filter(|s| !s.is_empty())
		})
		.collect();
	match !Traits.is_empty() {
		true => {
			Parts.push(format!("traits={}", Traits.join(",")));
		},
		false => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
