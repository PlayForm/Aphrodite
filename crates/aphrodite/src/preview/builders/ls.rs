//! directory-listing preview arm.

use crate::preview::input::Input;

/// directory-listing preview: file/dir counts + top extensions.
/// `[ls:42 files 7 dirs | .rs×18 .md×9 …]`.
pub(crate) fn build_ls_preview(inp:&Input<'_>) -> String {
	use std::collections::HashMap;
	let mut files = 0usize;
	let mut dirs = 0usize;
	let mut ext:HashMap<String, usize> = HashMap::new();
	for line in inp.raw.lines() {
		let t = line.trim();
		if t.is_empty() {
			continue;
		}
		// Skip the `total N` header `ls -l` prints (not a filesystem entry).
		if let Some(rest) = t.strip_prefix("total ")
			&& rest.chars().all(|c| c.is_ascii_digit())
			&& !rest.is_empty()
		{
			continue;
		}
		let b = line.as_bytes();
		// `ls -l` long form: mode string in the first column.
		let is_long = b.len() >= 10
			&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
			&& b[1..10]
				.iter()
				.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'));
		let (is_dir, name) = if is_long {
			let name = line.split_whitespace().last().unwrap_or("");
			(b[0] == b'd', name)
		} else if let Some(stripped) = t.strip_suffix('/') {
			(true, stripped)
		} else {
			(false, t)
		};
		if is_dir {
			dirs += 1;
		} else {
			files += 1;
			// File extension: text after the last `.` in the basename.
			let base = name.rsplit('/').next().unwrap_or(name);
			if let Some(dot) = base.rfind('.')
				&& dot > 0
				&& dot < base.len() - 1
			{
				let e:String = base[dot..].chars().take(8).collect();
				*ext.entry(e).or_insert(0) += 1;
			}
		}
	}
	if files == 0 && dirs == 0 {
		return format!("[ls:{}L]", inp.total);
	}
	let mut top:Vec<(String, usize)> = ext.into_iter().collect();
	top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
	let ext_str = top
		.iter()
		.take(3)
		.map(|(e, n)| format!("{}×{}", e, n))
		.collect::<Vec<_>>()
		.join(" ");
	if ext_str.is_empty() {
		format!("[ls:{} files {} dirs]", files, dirs)
	} else {
		format!("[ls:{} files {} dirs | {}]", files, dirs, ext_str)
	}
}
