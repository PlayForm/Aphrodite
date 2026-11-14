//! Go extractor: `func` / `type ... struct` signatures.

use std::collections::HashMap;

use super::{MAX_SIG_LEN, floor_boundary, sig, trunc_params};

// ── Go extractor ───────────────────────────────────────

pub(super) fn extract_go(content:&str, result:&mut HashMap<String, Vec<String>>, budget:&mut isize) {
	let mut fns:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		if trimmed.starts_with("func ") && trimmed.contains('(') {
			// func Name(...) or func (r *Receiver) Name(...)
			let after_func = &trimmed["func ".len()..];
			// T10 (F8): for a method, the receiver's own `(...)` comes before
			// the method's real parameter list - searching for the first `(`
			// in the whole line (as the old code did) finds the RECEIVER's
			// paren, not the params, so `func (s *Server) Start(addr string)`
			// used to report `func Start(s *Server)` instead of
			// `func Start(addr string)`. Search for the params paren only
			// after skipping past the receiver's closing `)`.
			let receiver_end = if after_func.starts_with('(') {
				after_func.find(')').map(|i| i + 1)
			} else {
				None
			};
			let name = match receiver_end {
				Some(end) => after_func[end..].trim_start().split('(').next().unwrap_or("?"),
				None => after_func.split('(').next().unwrap_or("?"),
			};
			let search_from = match receiver_end {
				// Absolute offset into `trimmed`: "func ".len() + receiver_end
				Some(end) => "func ".len() + end,
				None => 0,
			};
			let params_start = search_from + trimmed[search_from..].find('(').unwrap_or(0);
			let params_end = trimmed[params_start..].find(')').unwrap_or(0);
			let params = if params_end > 1 {
				&trimmed[params_start + 1..params_start + params_end]
			} else {
				""
			};
			let s = format!("func {}({})", name, trunc_params(params));
			let s_trunc = if s.len() > MAX_SIG_LEN {
				floor_boundary(&s, MAX_SIG_LEN - 3).to_string() + "..."
			} else {
				s
			};
			let slen = s_trunc.len();
			fns.push(s_trunc);
			*budget -= slen as isize + 1;
		}
	}
	if !fns.is_empty() {
		result.insert("fns".into(), fns);
	}
	if *budget <= 0 {
		return;
	}

	let mut types:Vec<String> = Vec::new();
	for line in content.lines() {
		if *budget <= 0 {
			break;
		}
		let trimmed = line.trim();
		if trimmed.starts_with("type ") && trimmed.contains("struct") {
			let name = trimmed["type ".len()..].split("struct").next().unwrap_or("?").trim();
			let s = sig("type", name);
			let slen = s.len();
			types.push(s);
			*budget -= slen as isize + 1;
		}
	}
	if !types.is_empty() {
		result.insert("types".into(), types);
	}
}
