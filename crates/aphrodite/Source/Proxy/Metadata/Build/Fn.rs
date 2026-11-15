//! Build output: OK/FAIL, the files compiled, the first error.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	match Content.contains("error") || Content.contains("aborting") {
		true => {
			Parts.push("status=FAIL".to_string());
		},
		false => {
			Parts.push("status=OK".to_string());
		},
	}

	let Files = Content
		.lines()
		.filter(|l| l.starts_with("Compiling ") || l.contains(" Compiling "))
		.count();

	match Files > 0 {
		true => {
			Parts.push(format!("files={}", Files));
		},
		false => {},
	}

	let FirstError = Content
		.lines()
		.find(|l| l.contains("error[") || l.contains("Error:"))
		.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));

	match FirstError {
		Some(e) => {
			Parts.push(format!("first_err={}", e));
		},
		None => {},
	}
}
