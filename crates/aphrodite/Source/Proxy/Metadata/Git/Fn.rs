//! `git status` / `git log` output: the branch and the commit count.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	let _ = Content.lines().try_for_each(|l| -> Result<(), ()> {
		let t = l.trim();

		match t.strip_prefix("On branch ") {
			Some(Rest) => {
				Parts.push(format!("branch={}", Rest.trim().replace('|', "/")));

				Err(())
			},
			None => Ok(()),
		}
	});

	match !Parts.iter().any(|p| p.starts_with("branch=")) {
		true => {
			let _ = Content.lines().try_for_each(|l| -> Result<(), ()> {
				let t = l.trim();

				match !t.is_empty() && !t.starts_with("* ") && !t.starts_with("  ") {
					true => {
						Parts.push(format!("branch={}", t.chars().take(40).collect::<String>().replace('|', "/")));

						Err(())
					},
					false => Ok(()),
				}
			});
		},
		false => {},
	}

	let Commits = Content
		.lines()
		.filter(|l| l.starts_with("commit ") || l.trim().starts_with("* ") || l.contains("commit"))
		.count();

	match Commits > 0 {
		true => {
			Parts.push(format!("commits={}", Commits));
		},
		false => {},
	}
}
