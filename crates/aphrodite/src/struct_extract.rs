//! Code structure extractor — regex-based pattern matching per language.
//! Port of plugins/aphrodite/_core/struct.py
//!
//! Extracts function/struct/class signatures from source code with
//! a 300-char preview budget. Used by the preview engine to show
//! code structure in CCR markers like [code_rust:3fns 2structs].

use std::collections::HashMap;

/// Maximum total output in characters (preview budget).
const BUDGET: usize = 300;

/// Maximum length of a single signature line.
const MAX_SIG_LEN: usize = 60;

/// Maximum param string length before truncation.
const MAX_PARAMS_LEN: usize = 35;

/// Extract code structure from source content.
/// Auto-detects language from content prefixes.
/// Returns a map of category → list of short signature strings.
pub fn extract_code_structure(
    content: &str,
    language: &str,
) -> HashMap<String, Vec<String>> {
    let lang = if language.is_empty() {
        auto_detect(content)
    } else {
        language.to_string()
    };

    if lang.is_empty() {
        return HashMap::new();
    }

    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let mut budget = BUDGET as isize;

    match lang.as_str() {
        "rust" => extract_rust(content, &mut result, &mut budget),
        "python" => extract_python(content, &mut result, &mut budget),
        "go" => extract_go(content, &mut result, &mut budget),
        "js" | "ts" => extract_js(content, &mut result, &mut budget),
        _ => {}
    }

    result
}

fn auto_detect(content: &str) -> String {
    let head = if content.len() > 500 { &content[..500] } else { content };
    if head.contains("fn ") && head.contains("->") {
        "rust".into()
    } else if head.contains("def ") && head.contains(":") {
        "python".into()
    } else if head.contains("func ") && head.contains("{") {
        "go".into()
    } else if head.contains("function ") || head.contains("=>") || head.contains("interface ") {
        "js".into()
    } else if head.trim_start().starts_with("#!/") || (head.len() > 200 && head[..200].contains("echo ")) {
        "sh".into()
    } else {
        String::new()
    }
}

/// Truncate a signature to fit the preview budget.
fn sig(kind: &str, text: &str) -> String {
    let s = format!("{} {}", kind, text).trim().to_string();
    if s.len() > MAX_SIG_LEN {
        s[..MAX_SIG_LEN - 3].to_string() + "..."
    } else {
        s
    }
}

fn trunc_params(params: &str) -> String {
    if params.len() > MAX_PARAMS_LEN {
        params[..MAX_PARAMS_LEN - 3].to_string() + "..."
    } else {
        params.to_string()
    }
}

// ── Rust extractor ─────────────────────────────────────

fn extract_rust(
    content: &str,
    result: &mut HashMap<String, Vec<String>>,
    budget: &mut isize,
) {
    // fn (with return type)
    let mut fns: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        let is_fn = (lower.starts_with("fn ")
            || lower.starts_with("pub fn ")
            || lower.starts_with("async fn ")
            || lower.starts_with("pub async fn ")
            || lower.starts_with("pub(crate) fn "))
            && trimmed.contains('(');

        if is_fn {
            // Extract name and params
            let after_fn = trimmed
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub ")
                .trim_start_matches("async ")
                .trim_start_matches("fn ");
            if let Some(paren) = after_fn.find('(') {
                let name = &after_fn[..paren];
                let rest = &after_fn[paren..];
                let params_end = rest.find(')').unwrap_or(rest.len());
                let params = &rest[1..params_end];
                let ret = if rest[params_end..].contains("->") {
                    rest[params_end..]
                        .split("->")
                        .nth(1)
                        .unwrap_or("")
                        .split('{')
                        .next()
                        .unwrap_or("")
                        .trim()
                } else {
                    ""
                };
                let params_trunc = trunc_params(params);
                let ret_str = if ret.is_empty() {
                    String::new()
                } else {
                    format!(" -> {}", ret)
                };
                let s = format!("fn {}({}){}", name, params_trunc, ret_str);
                let s = if s.len() > MAX_SIG_LEN {
                    s[..MAX_SIG_LEN - 3].to_string() + "..."
                } else {
                    s
                };
                let slen = s.len();
                fns.push(s);
                *budget -= slen as isize + 1;
            }
        }
    }
    if !fns.is_empty() {
        result.insert("fns".into(), fns);
    }
    if *budget <= 0 { return; }

    // struct
    let mut structs: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if (lower.starts_with("struct ") || lower.starts_with("pub struct ")) && !trimmed.contains('(') {
            let name = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("struct ")
                .split(|c: char| c.is_whitespace() || c == '<' || c == '{')
                .next()
                .unwrap_or("?");
            let s = sig("struct", name);
            let slen = s.len();
            structs.push(s);
            *budget -= slen as isize + 1;
        }
    }
    if !structs.is_empty() {
        result.insert("structs".into(), structs);
    }
    if *budget <= 0 { return; }

    // trait
    let mut traits: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("trait ") || lower.starts_with("pub trait ") {
            let name = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("trait ")
                .split(|c: char| c.is_whitespace() || c == '<' || c == '{')
                .next()
                .unwrap_or("?");
            let s = sig("trait", name);
            let slen = s.len();
            traits.push(s);
            *budget -= slen as isize + 1;
        }
    }
    if !traits.is_empty() {
        result.insert("traits".into(), traits);
    }
    if *budget <= 0 { return; }

    // impl
    let mut impls: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("impl ") || lower.starts_with("impl<") {
            let name = trimmed
                .trim_start_matches("impl")
                .trim_start_matches('<')
                .split(|c: char| c.is_whitespace() || c == '<' || c == '{')
                .find(|s| !s.is_empty())
                .unwrap_or("?");
            let s = sig("impl", name);
            let slen = s.len();
            impls.push(s);
            *budget -= slen as isize + 1;
        }
    }
    if !impls.is_empty() {
        result.insert("impls".into(), impls);
    }
}

// ── Python extractor ───────────────────────────────────

fn extract_python(
    content: &str,
    result: &mut HashMap<String, Vec<String>>,
    budget: &mut isize,
) {
    let mut fns: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        let is_def = (lower.starts_with("def ") || lower.starts_with("async def ")) && trimmed.contains('(');
        if is_def {
            let after = trimmed.trim_start_matches("async ").trim_start_matches("def ");
            if let Some(paren) = after.find('(') {
                let name = &after[..paren];
                let params_end = after[paren..].find(')').unwrap_or(0);
                let params = if params_end > 1 {
                    &after[paren + 1..paren + params_end]
                } else {
                    ""
                };
                let s = format!("def {}({})", name, trunc_params(params));
                let s_trunc = if s.len() > MAX_SIG_LEN {
                    s[..MAX_SIG_LEN - 3].to_string() + "..."
                } else {
                    s
                };
                let slen = s_trunc.len();
                fns.push(s_trunc);
                *budget -= slen as isize + 1;
            }
        }
    }
    if !fns.is_empty() {
        result.insert("fns".into(), fns);
    }
    if *budget <= 0 { return; }

    let mut classes: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        if trimmed.starts_with("class ") {
            let name = trimmed["class ".len()..]
                .split(|c: char| c == '(' || c == ':')
                .next()
                .unwrap_or("?");
            let s = sig("class", name);
            let slen = s.len();
            classes.push(s);
            *budget -= slen as isize + 1;
        }
    }
    if !classes.is_empty() {
        result.insert("classes".into(), classes);
    }
}

// ── Go extractor ───────────────────────────────────────

fn extract_go(
    content: &str,
    result: &mut HashMap<String, Vec<String>>,
    budget: &mut isize,
) {
    let mut fns: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        if trimmed.starts_with("func ") && trimmed.contains('(') {
            // func Name(...) or func (r *Receiver) Name(...)
            let after_func = &trimmed["func ".len()..];
            let name = if after_func.starts_with('(') {
                // Method: func (r *Type) Name(...)
                after_func
                    .split(')')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .split('(')
                    .next()
                    .unwrap_or("?")
            } else {
                after_func.split('(').next().unwrap_or("?")
            };
            let params_start = trimmed.find('(').unwrap_or(0);
            let params_end = trimmed[params_start..].find(')').unwrap_or(0);
            let params = if params_end > 1 {
                &trimmed[params_start + 1..params_start + params_end]
            } else {
                ""
            };
            let s = format!("func {}({})", name, trunc_params(params));
            let s_trunc = if s.len() > MAX_SIG_LEN {
                s[..MAX_SIG_LEN - 3].to_string() + "..."
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
    if *budget <= 0 { return; }

    let mut types: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        if trimmed.starts_with("type ") && trimmed.contains("struct") {
            let name = trimmed["type ".len()..]
                .split("struct")
                .next()
                .unwrap_or("?")
                .trim();
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

// ── JS/TS extractor ────────────────────────────────────

fn extract_js(
    content: &str,
    result: &mut HashMap<String, Vec<String>>,
    budget: &mut isize,
) {
    let mut fns: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        // function name(...) or export function name(...) or async function name(...)
        if trimmed.starts_with("function ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("async function ")
            || trimmed.starts_with("export async function ")
        {
            let after = trimmed
                .trim_start_matches("export ")
                .trim_start_matches("async ")
                .trim_start_matches("function ");
            let name = after.split('(').next().unwrap_or("?").trim();
            let s = sig("function", name);
            let slen = s.len();
            fns.push(s);
            *budget -= slen as isize + 1;
        }
        // Arrow functions: const name = (...) => { ... }
        if trimmed.contains("=>") && trimmed.contains('=') && !trimmed.starts_with("//") {
            let before_eq = trimmed.split('=').next().unwrap_or("").trim();
            if before_eq.starts_with("const ")
                || before_eq.starts_with("let ")
                || before_eq.starts_with("var ")
            {
                let name = before_eq
                    .trim_start_matches("const ")
                    .trim_start_matches("let ")
                    .trim_start_matches("var ")
                    .trim();
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    let s = sig("=>", name);
                    let slen = s.len();
                    fns.push(s);
                    *budget -= slen as isize + 1;
                }
            }
        }
    }
    if !fns.is_empty() {
        result.insert("fns".into(), fns);
    }
    if *budget <= 0 { return; }

    let mut classes: Vec<String> = Vec::new();
    for line in content.lines() {
        if *budget <= 0 { break; }
        let trimmed = line.trim();
        if trimmed.starts_with("class ") {
            let name = trimmed["class ".len()..]
                .split(|c: char| c == '{' || c == ' ' || c == ':')
                .next()
                .unwrap_or("?");
            let s = sig("class", name);
            let slen = s.len();
            classes.push(s);
            *budget -= slen as isize + 1;
        }
    }
    if !classes.is_empty() {
        result.insert("classes".into(), classes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_rust() {
        assert_eq!(auto_detect("fn main() -> i32 {}\npub struct Foo {}"), "rust");
    }

    #[test]
    fn test_auto_detect_python() {
        assert_eq!(auto_detect("def hello():\n    pass\n"), "python");
    }

    #[test]
    fn test_auto_detect_go() {
        assert_eq!(auto_detect("func main() {\n}\n"), "go");
    }

    #[test]
    fn test_auto_detect_js() {
        assert_eq!(auto_detect("function hello() {\n}\n"), "js");
    }

    #[test]
    fn test_auto_detect_unknown() {
        assert_eq!(auto_detect("plain text no code"), "");
    }

    #[test]
    fn test_extract_rust_fns() {
        let code = "pub fn main() -> i32 {\n    42\n}\nfn helper(x: i32) -> bool {\n    true\n}\n";
        let r = extract_code_structure(code, "rust");
        assert!(r.contains_key("fns"));
        let fns = &r["fns"];
        assert!(fns.iter().any(|s| s.contains("main")));
        assert!(fns.iter().any(|s| s.contains("helper")));
    }

    #[test]
    fn test_extract_rust_structs() {
        let code = "pub struct Foo {\n    x: i32,\n}\nstruct Bar<T> {}\n";
        let r = extract_code_structure(code, "rust");
        assert!(r.contains_key("structs"));
    }

    #[test]
    fn test_extract_python() {
        let code = "def hello(name: str) -> str:\n    return name\n\nclass MyClass:\n    pass\n";
        let r = extract_code_structure(code, "python");
        assert!(r.contains_key("fns"));
        assert!(r.contains_key("classes"));
    }

    #[test]
    fn test_extract_go() {
        let code = "func main() {\n}\n\nfunc (s *Server) Start(addr string) error {\n}\n";
        let r = extract_code_structure(code, "go");
        assert!(r.contains_key("fns"));
    }

    #[test]
    fn test_budget_respected() {
        // Generate lots of functions to test budget
        let mut code = String::new();
        for i in 0..50 {
            code.push_str(&format!("fn func{}(x: i32, y: i32, z: i32) -> i32 {{ 42 }}\n", i));
        }
        let r = extract_code_structure(&code, "rust");
        // Should have stopped before 50 due to budget
        if let Some(fns) = r.get("fns") {
            assert!(fns.len() < 50, "budget should cap output: got {}", fns.len());
        }
    }
}
