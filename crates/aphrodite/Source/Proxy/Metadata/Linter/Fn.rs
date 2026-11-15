//! Linter output: files with findings, the first error, the line count.

pub(crate) fn Fn(Content:&str, LineCount:usize, Parts:&mut Vec<String>) {
	let FilesLinted = Content
		.lines()
		.filter(|l| {
			(l.contains(".rs:") || l.contains(".py:") || l.contains(".go:") || l.contains(".ts:"))
				&& (l.contains("error") || l.contains("warning"))
		})
		.count();
	if FilesLinted > 0 {
		Parts.push(format!("files={}", FilesLinted));
	}
	let FirstError = Content
		.lines()
		.find(|l| l.contains("error[") || l.contains("Error:") || l.starts_with("error: "))
		.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));
	if let Some(e) = FirstError {
		Parts.push(format!("first_err={}", e));
	}
	Parts.push(format!("ln={}", LineCount));
}
