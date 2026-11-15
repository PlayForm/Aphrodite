//! The code reducer.
//!
//! Source files are recognised by their declaration lines, so the reduction is
//! a scan for the nine declaration shapes Aphrodite cares about across Rust,
//! Python, Go and JS/TS. Each match becomes one `[kind] line` entry, capped at
//! 50 so a generated file cannot produce a "reduction" bigger than its input.
//!
//! The lowercase form of each line is computed ONCE per line and passed to
//! every pattern test (bug 18-P12: it used to be recomputed inside the matcher
//! for each of the 9 patterns - 9 allocations per line). That is why the
//! matcher takes both `Line` and `Lower` instead of lowercasing internally.

/// Reduce code: extract function/struct/class signatures.
pub fn ReduceCode(Content:&str) -> Option<String> {
	#[derive(Debug)]
	struct SigPattern {
		Kind:&'static str,
		Pattern:&'static str,
	}

	let Patterns = [
		SigPattern { Kind:"fn", Pattern:r"^(pub\s+)?(async\s+)?fn\s+(\w+)" },
		SigPattern { Kind:"struct", Pattern:r"^(pub\s+)?struct\s+(\w+)" },
		SigPattern { Kind:"enum", Pattern:r"^(pub\s+)?enum\s+(\w+)" },
		SigPattern { Kind:"trait", Pattern:r"^(pub\s+)?trait\s+(\w+)" },
		SigPattern { Kind:"impl", Pattern:r"^(pub\s+)?impl\b" },
		SigPattern { Kind:"def", Pattern:r"^def\s+(\w+)\s*\(" },
		SigPattern { Kind:"class", Pattern:r"^class\s+(\w+)" },
		SigPattern { Kind:"func", Pattern:r"^func\s+(\w+)\s*\(" },
		SigPattern { Kind:"function", Pattern:r"^export\s+(?:async\s+)?function\s+(\w+)" },
	];

	let mut Sigs:Vec<String> = Vec::new();
	for Line in Content.lines() {
		let Trimmed = Line.trim();
		// Lowercase ONCE per line (was recomputed inside regex_match for
		// every one of the 9 patterns → 9 allocs/line). Bug 18-P12.
		let Lower = Trimmed.to_lowercase();
		for Sp in &Patterns {
			if let Some(_Caps) = crate::Stage2::Regex::Fn::RegexMatch(Trimmed, &Lower, Sp.Pattern) {
				let SigLine:String = Trimmed.chars().take(120).collect();
				Sigs.push(format!("  [{}] {}", Sp.Kind, SigLine));
			}
		}
	}

	if Sigs.is_empty() {
		None
	} else {
		let mut Result = String::from("[code structure]");
		for Sig in Sigs.iter().take(50) {
			Result.push('\n');
			Result.push_str(Sig);
		}
		Some(Result)
	}
}
