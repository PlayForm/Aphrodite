//! HTML preview arm.

use crate::preview::input::Input;

/// HTML preview: title, heading count, link count, body size estimate.
pub(crate) fn build_html_preview(inp:&Input<'_>) -> String {
	// Extract <title>…</title> text (anywhere on a line, case-insensitive).
	let title = inp.raw.lines().find_map(|l| {
		let lower = l.to_lowercase();
		let start = lower.find("<title>")? + 7;
		let end = lower[start..].find("</title>")?;
		Some(l[start..start + end].trim().chars().take(60).collect::<String>())
	});
	// Count common structural elements.
	let headings = inp.raw.matches("<h1").count()
		+ inp.raw.matches("<h2").count()
		+ inp.raw.matches("<h3").count()
		+ inp.raw.matches("<H1").count()
		+ inp.raw.matches("<H2").count()
		+ inp.raw.matches("<H3").count();
	let links = inp.raw.matches("<a ").count() + inp.raw.matches("<A ").count();
	let imgs = inp.raw.matches("<img ").count() + inp.raw.matches("<IMG ").count();
	let scripts = inp.raw.matches("<script").count() + inp.raw.matches("<SCRIPT").count();

	let mut parts:Vec<String> = Vec::new();
	if let Some(t) = title {
		parts.push(t);
	}
	let mut stats:Vec<String> = Vec::new();
	if headings > 0 {
		stats.push(format!("{}h", headings));
	}
	if links > 0 {
		stats.push(format!("{}a", links));
	}
	if imgs > 0 {
		stats.push(format!("{}img", imgs));
	}
	if scripts > 0 {
		stats.push(format!("{}script", scripts));
	}
	stats.push(format!("{}L", inp.total));
	let stats_str = stats.join(" ");
	if parts.is_empty() {
		format!("[html:{}]", stats_str)
	} else {
		format!("[html:{} | {}]", stats_str, parts.join(" | "))
	}
}