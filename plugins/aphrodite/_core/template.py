"""aphrodite — template rendering: preview markers, prompt templates with TOML fallback."""

import re as _re

from .config import CCR_MARKER_HINT, _toml_section


def _render_template(
    family: str,            # "compact" | "code_first" | "balance"
    ctype: str,             # content type key (diff, build_output, ...)
    vars: dict,             # template variables
    default: str = "",      # fallback format string
) -> str:
    """Render a preview template for the given family + content type.

    Resolution: toml[templates][preview][{family}][{ctype}] → reverse map
    → [templates][preview][{family}][_default] → hardcoded default.
    {variable} substitution with str.format(**safe_vars).
    Unknown vars are left as-is.
    """
    templates = _toml_section("templates")
    previews = templates.get("preview", {})
    family_templates = previews.get(family, {})

    # Resolve template string
    tmpl = family_templates.get(ctype)
    if tmpl is None:
        # Try reverse key map
        reverse_map = templates.get("reverse", {})
        mapped = reverse_map.get(ctype, ctype)
        tmpl = family_templates.get(mapped)
    if tmpl is None:
        tmpl = family_templates.get("_default", default)

    if not tmpl:
        return default

    # Safe substitution: only vars that exist, strip None values
    safe = {}
    for k, v in vars.items():
        if v is None:
            safe[k] = ""
        elif isinstance(v, str):
            safe[k] = v
        else:
            safe[k] = str(v)

    # Computed vars
    fn_val = safe.get("fn", "")
    safe["fx"] = f" {fn_val[:40]}" if fn_val else ""
    cmd_val = safe.get("cmd", "")
    safe["cmx"] = _re.sub(r'^[\$>]\s*', '', cmd_val.strip())[:40] if cmd_val else "?"
    sigs_val = safe.get("sigs", "")
    safe["sig1"] = sigs_val.split(";")[0].strip() if sigs_val else ""

    try:
        return tmpl.format(**safe)
    except (KeyError, ValueError):
        return tmpl  # return raw template if substitution fails


def _render_marker_tmpl(
    preview: str,
    ctype: str,
    meta: str,
    center: str | None,
    hash_val: str,
    size: int,
) -> str:
    """Render a CCR marker block using template configuration.

    Returns the full three-line block (preview + structure + marker)
    or a compact single-line fallback.
    """
    templates = _toml_section("templates")
    marker = templates.get("marker", {})
    fmt = marker.get("format", "")
    hint_str = marker.get("hint", "")

    if not fmt:
        # Fallback hardcoded format
        center_seg = f";center={center}" if center else ""
        hint = hint_str if CCR_MARKER_HINT and hint_str else ""
        return f"{preview}{hint}\n[{ctype}: {meta}{center_seg}]\n<<<CCR:{hash_val}|{ctype}|{size}>>>"

    center_seg = f";center={center}" if center else ""
    hint = hint_str if CCR_MARKER_HINT and hint_str else ""

    return fmt.format(
        preview=preview,
        type=ctype,
        meta=meta,
        center_seg=center_seg,
        hash=hash_val,
        size=size,
        hint=hint,
    )


def _render_prompt_tmpl(name: str, vars: dict | None = None) -> str:
    """Render a prompt template from [templates.prompts].

    Args:
        name: template key (session_inject, engine_offload, auto_expand_guidance, ...)
        vars: optional {variable: value} dict for substitution
    """
    templates = _toml_section("templates")
    prompts = templates.get("prompts", {})
    tmpl = prompts.get(name, "")

    if not tmpl:
        # Hardcoded fallbacks
        fallbacks = {
            "session_inject": "CCR markers (<<<CCR:hash|type|size>>>) point to compressed content. Retrieve if the preview doesn't tell you enough; aphrodite_catalog lists available entries.",
            "engine_offload": "These messages were offloaded to reduce context. Use aphrodite_retrieveif needed. The {tail} messages below are your active context.",
            "auto_expand_guidance": "Tool outputs are auto-expanded — you see full content inline. If you see a CCR marker, retrieve only if the preview hints at useful content.",
            "live_container_guidance": (
                "NEVER use read_file. ALWAYS use aphrodite_prefetch for ALL file reads. "
                "Prefetch returns CCR markers instantly — files load in background concurrently. "
                "Poll with aphrodite_prefetch_status, retrieve with aphrodite_retrieve(hash). "
                "Continue reasoning immediately — NEVER wait for file content you may not need."
            ),
            "catalog_context_warn": "context={ctx} msgs — prefer catalog over scanning history",
            "search_hint": "Use aphrodite_retrieve(hash) to expand any result hash.",
        }
        tmpl = fallbacks.get(name, "")

    if vars:
        safe = {k: str(v) if v is not None else "" for k, v in vars.items()}
        try:
            return tmpl.format(**safe)
        except (KeyError, ValueError):
            return tmpl

    return tmpl
