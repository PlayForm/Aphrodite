//! A unified diff: file count, added lines, deleted lines.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	let Files = Content.lines().filter(|l| l.starts_with("diff --git ")).count();
	if Files > 0 {
		Parts.push(format!("files={}", Files));
	}
	let Adds = Content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
	let Dels = Content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
	if Adds > 0 {
		Parts.push(format!("adds={}", Adds));
	}
	if Dels > 0 {
		Parts.push(format!("dels={}", Dels));
	}
}
