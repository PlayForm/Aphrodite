//! An error payload: the trace location, the first message and the error
//! count. The trace slice walks back to a char boundary - a CJK comment before
//! `.rs:` used to panic the byte slice.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	let mut Trace = String::new();

	let _ = Content.lines().try_for_each(|l| -> Result<(), ()> {
		let t = l.trim();
		let ExtPos = t.find(".rs:").or_else(|| t.find(".py:")).or_else(|| t.find(".go:"));

		match ExtPos {
			Some(Pos) => {
				// `Pos` itself is a valid boundary (`.find` on ASCII
				// patterns), but `Start`/`End` are arbitrary byte offsets
				// from it and can land mid-codepoint on non-ASCII lines
				// (e.g. a CJK comment before ".rs:") - snap both to the
				// nearest valid boundary rather than panicking on `t[..]`.
				let Start = (0..=Pos.saturating_sub(12)).rev().find(|&b| t.is_char_boundary(b)).unwrap_or(0);
				let End = (Pos + 40).min(t.len());
				let End = (End..=t.len()).find(|&b| t.is_char_boundary(b)).unwrap_or(t.len());

				Trace = t[Start..End].to_string();

				Err(())
			},
			None => Ok(()),
		}
	});

	match !Trace.is_empty() {
		true => {
			Parts.push(format!("trace={}", Trace.replace('|', "/")));
		},
		false => {},
	}

	let Message = Content.lines().find(|l| l.contains("Error:") || l.contains("error[")).map(|l| {
		let t = l.trim();
		let Index = t.find("Error:").or_else(|| t.find("error[")).unwrap_or(0);
		t[Index..].chars().take(80).collect::<String>().replace('|', "/")
	});

	match Message {
		Some(m) => {
			Parts.push(format!("msg={}", m));
		},
		None => {
			let Fl = Content.lines().next().unwrap_or("").trim();

			match !Fl.is_empty() {
				true => {
					Parts.push(format!("msg={}", Fl.chars().take(80).collect::<String>().replace('|', "/")));
				},
				false => {},
			}
		},
	}

	let ErrorCount = Content
		.lines()
		.filter(|l| l.contains("error") || l.starts_with("thread '"))
		.count();

	match ErrorCount > 0 {
		true => {
			Parts.push(format!("N_errors={}", ErrorCount));
		},
		false => {},
	}
}
