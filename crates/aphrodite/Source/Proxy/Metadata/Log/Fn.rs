//! Log output: the highest level seen and the last line.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	for l in Content.lines() {
		let t = l.trim();
		for Level in &["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "TRACE", "FATAL", "PANIC"] {
			if t.contains(Level) {
				Parts.push(format!("level={}", Level.to_lowercase()));
				break;
			}
		}
		if Parts.iter().any(|p| p.starts_with("level=")) {
			break;
		}
	}
	let LastLine = Content.lines().last().unwrap_or("").trim().chars().take(60).collect::<String>();
	if !LastLine.is_empty() {
		Parts.push(format!("last={}", LastLine.replace('|', "/")));
	}
	Parts.push(format!("ln={}", LineCount));
}
