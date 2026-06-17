"""aphrodite — code structure extractor with regex-based patterns per language."""

import re as _re

_CODE_PATTERNS: dict[str, dict[str, _re.Pattern]] = {
    "rust": {
        "fn": _re.compile(
            r'^\s*(?:pub(?:\s*\(\s*crate\s*\))?\s+)?(?:async\s+)?fn\s+(\w+(?:::\w+)*)\s*\(([^)]*)\)(?:\s*->\s*(\S+(?:\s*\+\s*\S+)*))?',
            _re.MULTILINE,
        ),
        "struct": _re.compile(r'^\s*(?:pub\s+)?struct\s+(\w+)', _re.MULTILINE),
        "impl": _re.compile(
            r'^\s*impl(?:\s*<\s*\w+(?:\s*,\s*\w+)*\s*>)?\s+(\w+(?:::\w)*(?:\s*<\s*\w+(?:\s*,\s*\w+)*\s*>)?)',
            _re.MULTILINE,
        ),
        "trait": _re.compile(r'^\s*(?:pub\s+)?trait\s+(\w+)', _re.MULTILINE),
        "mod": _re.compile(r'^\s*(?:pub\s+)?mod\s+(\w+)', _re.MULTILINE),
    },
    "python": {
        "def": _re.compile(r'^\s*(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)', _re.MULTILINE),
        "class": _re.compile(r'^\s*class\s+(\w+)', _re.MULTILINE),
    },
    "go": {
        "func": _re.compile(
            r'^\s*func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)\s*\(([^)]*)\)', _re.MULTILINE
        ),
        "type": _re.compile(r'^\s*type\s+(\w+)\s+struct', _re.MULTILINE),
        "interface": _re.compile(r'^\s*type\s+(\w+)\s+interface', _re.MULTILINE),
    },
    "js": {
        "function": _re.compile(
            r'(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)|\b(\w+)\s*=\s*(?:async\s*)?\(([^)]*)\)\s*=>',
            _re.MULTILINE,
        ),
        "class": _re.compile(r'class\s+(\w+)', _re.MULTILINE),
    },
}


def _extract_code_structure(content: str, language: str = "") -> dict:
    """Extract function/class/struct signatures from source code.

    Returns a dict with keys like 'fns', 'structs', 'impls', 'classes', etc.
    Each value is a list of short (<60 char) signature strings.
    Total output ≤ 300 chars to stay within preview budget.
    """
    # Auto-detect language
    if not language:
        if "fn " in content[:500] and "->" in content[:500]:
            language = "rust"
        elif "def " in content[:500] and ":" in content[:500]:
            language = "python"
        elif "func " in content[:500] and "{" in content[:500]:
            language = "go"
        elif "function " in content[:500] or "=>" in content[:500] or "interface " in content[:500]:
            language = "js"  # TS also matches JS patterns
        elif content[:500].strip().startswith("#!/") or "echo " in content[:200]:
            language = "sh"
        else:
            return {}

    pats = _CODE_PATTERNS.get(language)
    if not pats:
        return {}

    result: dict[str, list[str]] = {}
    budget = 300

    def _sig(kind: str, text: str) -> str:
        """Truncate a signature to fit preview budget."""
        s = f"{kind} {text}".strip()
        return s[:60]

    # Collect function signatures (most important for navigation)
    if "fn" in pats:
        fns = []
        for m in pats["fn"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            ret = m.group(3) if m.lastindex and m.lastindex >= 3 and m.group(3) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            ret_str = f" -> {ret.strip()}" if ret else ""
            s = f"fn {name}({params}){ret_str}"
            if len(s) > 60:
                s = s[:57] + "..."
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    if "def" in pats:
        fns = []
        for m in pats["def"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            s = f"def {name}({params})"
            s = s[:60]
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    if "func" in pats:
        fns = []
        for m in pats["func"].finditer(content):
            name = m.group(1)
            params = m.group(2).strip() if m.group(2) else ""
            if len(params) > 35:
                params = params[:32] + "..."
            s = f"func {name}({params})"
            s = s[:60]
            fns.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if fns:
            result["fns"] = fns

    if budget <= 0:
        return result

    # Collect structs/types/classes
    for kind, key in [("struct", "structs"), ("class", "classes"), ("type", "types")]:
        if kind in pats:
            items = []
            for m in pats[kind].finditer(content):
                s = f"{kind} {m.group(1)}"
                s = s[:60]
                items.append(s)
                budget -= len(s) + 1
                if budget < 0:
                    break
            if items:
                result.setdefault(key, items)

    if budget <= 0:
        return result

    # Collect impls (Rust)
    if "impl" in pats:
        items = []
        for m in pats["impl"].finditer(content):
            s = f"impl {m.group(1)}"
            s = s[:60]
            items.append(s)
            budget -= len(s) + 1
            if budget < 0:
                break
        if items:
            result["impls"] = items

    return result
