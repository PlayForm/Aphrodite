//! Linter output: files with findings, the first error, the line count.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	let FilesLinted = Content
		.lines()
		.filter(|l| {
			(l.contains(".rs:") || l.contains(".py:") || l.contains(".go:") || l.contains(".ts:"))
				&& (l.contains("error") || l.contains("warning"))
		})
		.count();

	match FilesLinted > 0 {
		true => {
			Parts.push(format!("files={}", FilesLinted));
		},
		false => {},
	}

	let FirstError = Content
		.lines()
		.find(|l| l.contains("error[") || l.contains("Error:") || l.starts_with("error: "))
		.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));

	match FirstError {
		Some(e) => {
			Parts.push(format!("first_err={}", e));
		},
		None => {},
	}

	Parts.push(format!("ln={}", LineCount));
}
