//! Log output: the highest level seen and the last line.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	let _ = Content.lines().try_for_each(|l| -> Result<(), ()> {
		let t = l.trim();

		match ["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "TRACE", "FATAL", "PANIC"]
			.iter()
			.find(|Level| t.contains(**Level))
		{
			Some(Level) => {
				Parts.push(format!("level={}", Level.to_lowercase()));
			},
			None => {},
		}

		match Parts.iter().any(|p| p.starts_with("level=")) {
			true => Err(()),
			false => Ok(()),
		}
	});

	let LastLine = Content.lines().last().unwrap_or("").trim().chars().take(60).collect::<String>();

	match !LastLine.is_empty() {
		true => {
			Parts.push(format!("last={}", LastLine.replace('|', "/")));
		},
		false => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
