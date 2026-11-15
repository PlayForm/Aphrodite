//! HTML preview arm.
//!
//! A page's title, heading count and link count say what it is about without
//! shipping the markup. This arm renders those. It is not an HTML parser and it
//! does not execute anything - the tags are read as text, so a `<title>` inside
//! a script string would be found too.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of HTML content.
///
/// Promises the `<title>` text, a heading count, a link count and a body-size
/// estimate. It is not a DOM: nothing is normalised, so malformed markup yields
/// whatever counts the text happens to produce.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	// Extract <title>...</title> text (anywhere on a line, case-insensitive).
	let Title = Inp.raw.lines().find_map(|l| {
		let Lower = l.to_lowercase();
		let Start = Lower.find("<title>")? + 7;
		let End = Lower[Start..].find("</title>")?;
		Some(l[Start..Start + End].trim().chars().take(60).collect::<String>())
	});
	// Count common structural elements.
	let Headings = Inp.raw.matches("<h1").count()
		+ Inp.raw.matches("<h2").count()
		+ Inp.raw.matches("<h3").count()
		+ Inp.raw.matches("<H1").count()
		+ Inp.raw.matches("<H2").count()
		+ Inp.raw.matches("<H3").count();
	let Links = Inp.raw.matches("<a ").count() + Inp.raw.matches("<A ").count();
	let Imgs = Inp.raw.matches("<img ").count() + Inp.raw.matches("<IMG ").count();
	let Scripts = Inp.raw.matches("<script").count() + Inp.raw.matches("<SCRIPT").count();

	let Parts:Vec<String> = Title.into_iter().collect();
	let mut Stats:Vec<String> = Vec::new();

	match Headings > 0 {
		true => {
			Stats.push(format!("{}h", Headings));
		},
		false => {},
	}

	match Links > 0 {
		true => {
			Stats.push(format!("{}a", Links));
		},
		false => {},
	}

	match Imgs > 0 {
		true => {
			Stats.push(format!("{}img", Imgs));
		},
		false => {},
	}

	match Scripts > 0 {
		true => {
			Stats.push(format!("{}script", Scripts));
		},
		false => {},
	}

	Stats.push(format!("{}L", Inp.Total));
	let StatsStr = Stats.join(" ");

	match Parts.is_empty() {
		true => format!("[html:{}]", StatsStr),
		false => format!("[html:{} | {}]", StatsStr, Parts.join(" | ")),
	}
}
