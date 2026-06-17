"""Template-driven CCR preview generation."""

from .._core import PREVIEW_MAX_CHARS, _extract_code_structure, _render_template
from .classify import _classify_content


def _make_ccr_preview(content: str, klass: dict | None = None, model_family: str = "compact") -> str:
    """Generate a template-driven CCR preview from TOML config.

    Builds template variables from the classifier, then delegates to
    _render_template() for family-aware formatting. Falls back to hardcoded
    defaults if TOML is missing or incomplete.

    Args:
        content: Raw content string.
        klass: Pre-computed classification dict. If None, classified inline.
        model_family: 'compact' | 'code_first' | 'balance' — selects template set.

    Returns:
        A rich, single-line preview string (≤ preview_max_chars, pipe-safe).
    """
    if klass is None:
        klass = _classify_content(content)

    ctype = klass.get("type", "text")
    ln = klass.get("ln", "?")
    max_chars = PREVIEW_MAX_CHARS

    # ── Build template vars from classifier ──────────────────────────
    vars: dict = {
        "type": ctype,
        "ln": str(ln),
        "first": content[:110].replace("\\n", " ").replace("\\r", " ").strip(),
        "err": str(klass.get("errors", "0")),
        "warn": str(klass.get("warnings", "0")),
        "code": str(klass.get("code", "?")),
        "loc": str(klass.get("loc", "")),
        "msg": str(klass.get("msg", ""))[:110],
        "commit": str(klass.get("hash", "???????")),
        "subject": str(klass.get("subject", ""))[:100],
        "exit": str(klass.get("exit", "?")),
        "cmd": str(klass.get("cmd", "")),
        "files": str(klass.get("files", klass.get("total", "?"))),
        "fn": str(klass.get("fn", "")),
        "plus": str(klass.get("+", "?")),
        "minus": str(klass.get("-", "?")),
        "keys": str(klass.get("keys", "")),
        "items": str(klass.get("len", klass.get("rows", "?"))),
        "pid": str(klass.get("pid", "?")),
        "uptime": str(klass.get("uptime", "")),
        "fns": "?",
        "structs": "0",
        "impls": "0",
        "classes": "0",
        "types": "0",
        "sigs": "",
        "size": str(klass.get("size", "?")),
        "entries": str(klass.get("entries", "0")),
        "elements": str(klass.get("elements", "0")),
        "total": str(klass.get("total", "0")),
        "errors": str(klass.get("errors", "0")),
    }

    # ── Code structure-map enrichment ────────────────────────────────
    if ctype in ("code", "code_rust", "code_python", "code_go", "code_js", "code_ts", "code_sh"):
        lang = {"code_rust": "rust", "code_python": "python",
                "code_go": "go", "code_js": "js", "code_ts": "js",
                "code_sh": "sh"}.get(ctype, "")
        struct = _extract_code_structure(content, lang)
        if struct:
            sigs = struct.get("fns", [])
            vars["fns"] = str(len(sigs))
            vars["structs"] = str(len(struct.get("structs", [])))
            vars["impls"] = str(len(struct.get("impls", [])))
            vars["classes"] = str(len(struct.get("classes", [])))
            vars["types"] = str(len(struct.get("types", [])))
            vars["sigs"] = "; ".join(sigs[:2]) if sigs else ""

    # ── Delegate to template system ──────────────────────────────────
    result = _render_template(model_family, ctype, vars, "")

    if not result:
        result = f"[{ctype}:{vars['first']}]"

    return result[:max_chars].replace("|", "-")
