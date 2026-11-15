//! A unified diff: file count, added lines, deleted lines.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	let Files = Content.lines().filter(|l| l.starts_with("diff --git ")).count();

	match Files > 0 {
		true => {
			Parts.push(format!("files={}", Files));
		},
		false => {},
	}

	let Adds = Content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();

	let Dels = Content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();

	match Adds > 0 {
		true => {
			Parts.push(format!("adds={}", Adds));
		},
		false => {},
	}

	match Dels > 0 {
		true => {
			Parts.push(format!("dels={}", Dels));
		},
		false => {},
	}
}
