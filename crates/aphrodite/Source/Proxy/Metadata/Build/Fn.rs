//! Build output: OK/FAIL, the files compiled, the first error.

pub(crate) fn Fn(Content:&str, Parts:&mut Vec<String>) {
	if Content.contains("error") || Content.contains("aborting") {
		Parts.push("status=FAIL".to_string());
	} else {
		Parts.push("status=OK".to_string());
	}
	let Files = Content
		.lines()
		.filter(|l| l.starts_with("Compiling ") || l.contains(" Compiling "))
		.count();
	if Files > 0 {
		Parts.push(format!("files={}", Files));
	}
	let FirstError = Content
		.lines()
		.find(|l| l.contains("error[") || l.contains("Error:"))
		.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));
	if let Some(e) = FirstError {
		Parts.push(format!("first_err={}", e));
	}
}
