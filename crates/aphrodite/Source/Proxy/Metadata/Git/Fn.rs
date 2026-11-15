//! `git status` / `git log` output: the branch and the commit count.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	for l in Content.lines() {
		let t = l.trim();
		if let Some(Rest) = t.strip_prefix("On branch ") {
			Parts.push(format!("branch={}", Rest.trim().replace('|', "/")));
			break;
		}
	}
	if !Parts.iter().any(|p| p.starts_with("branch=")) {
		for l in Content.lines() {
			let t = l.trim();
			if !t.is_empty() && !t.starts_with("* ") && !t.starts_with("  ") {
				Parts.push(format!("branch={}", t.chars().take(40).collect::<String>().replace('|', "/")));
				break;
			}
		}
	}
	let Commits = Content
		.lines()
		.filter(|l| l.starts_with("commit ") || l.trim().starts_with("* ") || l.contains("commit"))
		.count();
	if Commits > 0 {
		Parts.push(format!("commits={}", Commits));
	}
}
