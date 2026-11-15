//! An error payload: the trace location, the first message and the error
//! count. The trace slice walks back to a char boundary - a CJK comment before
//! `.rs:` used to panic the byte slice.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	let mut Trace = String::new();
	for l in Content.lines() {
		let t = l.trim();
		let ExtPos = t.find(".rs:").or_else(|| t.find(".py:")).or_else(|| t.find(".go:"));
		if let Some(Pos) = ExtPos {
			// `Pos` itself is a valid boundary (`.find` on ASCII
			// patterns), but `Start`/`End` are arbitrary byte offsets
			// from it and can land mid-codepoint on non-ASCII lines
			// (e.g. a CJK comment before ".rs:") - snap both to the
			// nearest valid boundary rather than panicking on `t[..]`.
			let mut Start = Pos.saturating_sub(12);
			while Start > 0 && !t.is_char_boundary(Start) {
				Start -= 1;
			}
			let mut End = (Pos + 40).min(t.len());
			while End < t.len() && !t.is_char_boundary(End) {
				End += 1;
			}
			Trace = t[Start..End].to_string();
			break;
		}
	}
	if !Trace.is_empty() {
		Parts.push(format!("trace={}", Trace.replace('|', "/")));
	}
	let Message = Content.lines().find(|l| l.contains("Error:") || l.contains("error[")).map(|l| {
		let t = l.trim();
		let Index = t.find("Error:").or_else(|| t.find("error[")).unwrap_or(0);
		t[Index..].chars().take(80).collect::<String>().replace('|', "/")
	});
	if let Some(m) = Message {
		Parts.push(format!("msg={}", m));
	} else {
		let Fl = Content.lines().next().unwrap_or("").trim();
		if !Fl.is_empty() {
			Parts.push(format!("msg={}", Fl.chars().take(80).collect::<String>().replace('|', "/")));
		}
	}
	let ErrorCount = Content
		.lines()
		.filter(|l| l.contains("error") || l.starts_with("thread '"))
		.count();
	if ErrorCount > 0 {
		Parts.push(format!("N_errors={}", ErrorCount));
	}
}
