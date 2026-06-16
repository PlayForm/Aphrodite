"""aphrodite - marker formatting and proxy compression."""

import contextlib
import http.client
import json
import logging
import re
import threading
import time

from ._core import _CCR_RE

_log = logging.getLogger("aphrodite")

# Compiled regex for valid CCR hash validation
_VALID_HASH_RE = re.compile(r"^(?:[0-9a-f]{24,}|i:[0-9a-f]{6,})$")
_parse_errors = 0  # count of malformed CCR markers silently skipped

# ── Thread-local connection pool (keep-alive reuse) ──────────
_tls = threading.local()


def _get_conn(port: int) -> http.client.HTTPConnection:
    """Get or create a keep-alive HTTPConnection for the given port (thread-local).

    Connections idle for >60s are evicted and recreated.
    """
    if not hasattr(_tls, "conns"):
        _tls.conns = {}
        _tls.conn_ts = {}
    # Evict idle connections (>60s since last use)
    now = time.time()
    if port in _tls.conns and now - _tls.conn_ts.get(port, 0) > 60:
        with contextlib.suppress(Exception):
            _tls.conns[port].close()
        del _tls.conns[port]
        del _tls.conn_ts[port]
    if port not in _tls.conns:
        _tls.conns[port] = http.client.HTTPConnection("127.0.0.1", port, timeout=0.5)
    return _tls.conns[port]


def _put_conn(port: int) -> None:
    """Close and remove a connection (recovery after error)."""
    if hasattr(_tls, "conns") and port in _tls.conns:
        with contextlib.suppress(Exception):
            _tls.conns[port].close()
        del _tls.conns[port]
        if hasattr(_tls, "conn_ts") and port in _tls.conn_ts:
            del _tls.conn_ts[port]


def _classify_content(content: str) -> dict:
    """Classify content into structured metadata dict.

    Detects content type from content structure and extracts relevant metadata,
    mirroring the logic in _extract_tool_metadata but working from raw content
    alone (no tool name/args context). Safe, best-effort, never throws.

    Returns dict with at minimum {"type": "<detected_type>"} plus type-specific
    keys. Returns {"type": "text", "ln": N} for unrecognised content.
    """
    try:
        if not content or not isinstance(content, str):
            return {"type": "text", "ln": 0}
        lines = content.splitlines()
        ln = len(lines)
        trimmed = content[:5000]  # only analyse first 5KB for classification

        # ── diff content (absorptive) ────────────────────────────
        if trimmed.startswith("diff --git") or trimmed.startswith("---") or any(
            line.startswith("diff --git") for line in lines[:5]
        ):
            meta = {"type": "diff", "ln": str(ln)}
            files = set()
            plus = minus = 0
            for line in lines:
                m = re.match(r"^\+\+\+ b/(.+)$", line)
                if m:
                    meta["fn"] = m.group(1)
                    files.add(m.group(1))
                elif line.startswith("--- a/"):
                    files.add(line.split("/", 2)[-1].split(None, 1)[0] if "/" in line else line)
                elif line.startswith("+") and not line.startswith("+++"):
                    plus += 1
                elif line.startswith("-") and not line.startswith("---"):
                    minus += 1
            if files:
                meta["files"] = str(len(files))
            meta["+"] = str(plus)
            meta["-"] = str(minus)
            return meta

        # ── Terminal output (exit code pattern) ──
        # Must come before build_output — terminal captures of build
        # commands have "Compiling" lines but end with an exit code.
        for line in lines[-5:]:
            m = re.match(r"exit code[\s:]+(\d+)", line.strip(), re.IGNORECASE)
            if m:
                last_line = ""
                for l2 in lines:
                    s = l2.strip()
                    if s:
                        last_line = s
                meta = {"type": "terminal", "exit": m.group(1)}
                if last_line:
                    meta["last"] = last_line[:60]
                for l2 in lines[:3]:
                    if l2.strip().startswith("$") or l2.strip().startswith(">"):
                        meta["cmd"] = l2.strip()[:40]
                        break
                return meta

        # ── Build output (absorptive) ────────────────────────────
        # Must come before build_error — "Compiling" is normal output,
        # "error[E" is a subset. We detect build output first, then
        # let build_error refine only when it's the dominant signal.
        if any("Compiling" in line or "Compiling" in line for line in lines[:30]):
            meta = {"type": "build_output", "ln": str(ln)}
            error_count = sum(1 for line in lines if "error[" in line or line.strip().startswith("error:"))
            warning_count = sum(1 for line in lines if "warning:" in line)
            meta["errors"] = str(error_count)
            meta["warnings"] = str(warning_count)
            return meta

        # ── Rust build errors ────────────────────────────────────
        if "error[E" in trimmed:
            meta = {"type": "build_error", "ln": str(ln)}
            for line in lines[:20]:
                m = re.match(r"error\[(E\d+)\]", line)
                if m:
                    meta["code"] = m.group(1)
                    break
                m = re.match(r" --> (.+:\d+:\d+)", line)
                if m and "loc" not in meta:
                    meta["loc"] = m.group(1)
            return meta

        # ── JSON content ──────────────────────────────────────────
        stripped = trimmed.strip()
        if stripped.startswith("{") or stripped.startswith("["):
            try:
                data = json.loads(stripped)
                if isinstance(data, dict):
                    # search_results pattern
                    if "total_count" in data:
                        meta = {"type": "search_results"}
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        meta["total"] = str(data["total_count"])
                        return meta
                    # process_output (session-based)
                    if "session_id" in data:
                        meta = {"type": "process_output"}
                        meta["pid"] = str(data.get("pid", data.get("process_id", "?")))
                        if "uptime" in data:
                            meta["uptime"] = str(data["uptime"])
                        return meta
                    # terminal-like output with exit_code
                    if "exit_code" in data or "output" in data:
                        meta = {"type": "terminal"}
                        if "exit_code" in data:
                            meta["exit"] = str(data["exit_code"])
                        if "output" in data and isinstance(data["output"], str):
                            last = data["output"].splitlines()
                            if last:
                                meta["last"] = last[-1][:60]
                        return meta
                    # search_files (matches key)
                    if "matches" in data:
                        meta = {"type": "search_files"}
                        meta["files"] = str(len(data["matches"])) if isinstance(data["matches"], (list, tuple)) else str(data["matches"])
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        return meta
                    # Fallback JSON: extract top-level keys
                    keys = list(data.keys())[:8]
                    meta = {"type": "json", "ln": str(ln)}
                    if keys:
                        meta["keys"] = ",".join(keys)
                    return meta
                elif isinstance(data, list):
                    return {"type": "json_list", "ln": str(ln), "len": str(len(data))}
            except (json.JSONDecodeError, ValueError):
                pass

        # ── Search output (file:line: text pattern) ───────────────
        file_line_count = 0
        for line in lines[:200]:
            if re.match(r"^[^\s]+:\d+:", line):
                file_line_count += 1
        if file_line_count > 3 and file_line_count > ln * 0.3:
            return {"type": "search_files", "files": str(file_line_count), "ln": str(ln)}

        # ── Tabular/structured output ──────────────────────────
        pipe_count = sum(1 for line in lines[:50] if "|" in line)
        if pipe_count >= 3 and pipe_count > ln * 0.2:
            return {"type": "tabular", "rows": str(pipe_count), "ln": str(ln)}

        # ── Error / traceback (absorptive) ─────────────────────
        if any("Traceback" in line or "panic" in line or "Error:" in line for line in lines[:10]):
            error_msg = "unknown"
            for line in lines:
                s = line.strip()
                if "Error:" in s or "panic" in s:
                    error_msg = s[:80]
                    break
                if "error:" in s.lower():
                    error_msg = s[:80]
                    break
            # Also try last line for simple error messages
            if error_msg == "unknown" and lines:
                last = lines[-1].strip()
                if last:
                    error_msg = last[:80]
            return {"type": "error", "msg": error_msg, "ln": str(ln)}

        # ── Git commit log (absorptive) ────────────────────────
        if ln >= 2 and re.match(r"^[a-f0-9]{7,40}\s", lines[0].strip()):
            parts = lines[0].strip().split(None, 2)
            if len(parts) >= 2:
                subj = parts[-1] if len(parts) > 1 else ""
                return {"type": "commit", "hash": parts[0][:8], "subject": subj[:80], "ln": str(ln)}

        # ── Fallback: text ─────────────────────────────────────
        return {"type": "text", "ln": str(ln)}
    except Exception:
        if logging.getLogger("aphrodite").isEnabledFor(logging.DEBUG):
            logging.getLogger("aphrodite").debug("_classify_content: failed for %d-char content", len(content) if isinstance(content, str) else 0)
        return {"type": "text", "ln": str(len(content.splitlines())) if isinstance(content, str) else 0}


def _make_ccr_preview(content: str, klass: dict | None = None) -> str:
    """Generate an absorptive, content-aware CCR preview.

    Uses the classification to produce a structured 1-liner that helps
    the LLM decide whether to retrieve. Each content type gets a
    consistent, scannable format — the "absorptive" pattern means new
    content of the same type automatically gets the same treatment.

    Args:
        content: Raw content string.
        klass: Pre-computed classification dict (from _classify_content).
               If None, classification runs inline.

    Returns:
        A rich, single-line preview string (≤120 chars, pipe-safe).
    """
    if klass is None:
        klass = _classify_content(content)

    ctype = klass.get("type", "text")
    ln = klass.get("ln", "?")

    if ctype == "diff":
        fn = klass.get("fn", "")
        files = klass.get("files", "")
        plus = klass.get("+", "?")
        minus = klass.get("-", "?")
        if fn and not files:
            files = "1"
        parts = [f"{files or '?'}f", f"+{plus}/-{minus}", f"{ln}L"]
        if fn:
            parts.append(fn[:40])
        return f"[diff:{' '.join(parts)}]"
    elif ctype == "build_output":
        return f"[build:{klass.get('errors','0')}E {klass.get('warnings','0')}W {ln}L]"
    elif ctype == "build_error":
        code = klass.get("code", "?")
        loc = klass.get("loc", "")
        parts = [code]
        if loc:
            parts.append(loc[:40])
        parts.append(f"{ln}L")
        return f"[error:{' '.join(parts)}]"
    elif ctype == "error":
        return f"[error:{klass.get('msg','?')[:110]}]"
    elif ctype == "commit":
        return f"[commit:{klass.get('hash','???????')} {klass.get('subject','')[:100]}]"
    elif ctype == "terminal":
        exit_code = klass.get("exit", "?")
        cmd = klass.get("cmd", "")
        clean = re.sub(r"^[\$>]\s*", "", cmd.strip()) if cmd else "?"
        return f"[terminal:{clean[:40]} exit={exit_code}]"[:120]
    elif ctype == "search_files" or ctype == "search_results":
        files = klass.get("files", klass.get("total", "?"))
        return f"[grep:{files} matches {ln}L]"
    elif ctype == "tabular":
        return f"[table:{klass.get('rows','?')} rows {ln}L]"
    elif ctype == "json":
        keys = klass.get("keys", "")
        return f"[json:{keys} {ln}L]"[:120]
    elif ctype == "json_list":
        return f"[json:{klass.get('len','?')} items {ln}L]"
    elif ctype == "process_output":
        pid = klass.get("pid", "?")
        uptime = klass.get("uptime", "")
        parts = [f"pid={pid}"]
        if uptime:
            parts.append(f"up={uptime}")
        parts.append(f"{ln}L")
        return f"[process:{' '.join(parts)}]"
    else:
        # Fallback: first meaningful content, classified
        first = content[:110].replace("\n", " ").replace("\r", " ").strip()
        return f"[{ctype}:{first}]"[:120]


def _ccr_marker(hash_val, ccr_type, size, mode="", preview="", headroom_budget=None, meta=None, center=None):
    """Build a CCR output block: preview, structure, marker  -  each on its own line.

    Matches the Rust ``format_ccr_output`` layout. The LLM reads the
    preview + structure first, then decides whether to retrieve.

    Args:
        hash_val: CCR hash string.
        ccr_type: Type label (tool, terminal, code_rust, etc.).
        size: Original size in bytes.
        mode: Proxy mode (token, cache, inline).
        preview: Text preview (pipe-safe, control-char-stripped).
        headroom_budget: If set, truncates preview under tight budget.
        meta: Dict of structured metadata (lang, fns, structs, etc.).
        center: Optional center string  -  travels with the marker.
    """
    # Line 1: preview
    lines = []
    if preview:
        safe = preview.replace("|", "-").replace("\n", " ").replace("\r", " ").strip()
        safe = "".join(c if c >= " " else " " for c in safe)
        if headroom_budget is not None:
            try:
                budget = int(headroom_budget)
                if budget < 25:
                    safe = safe[:30]
                elif budget < 50:
                    safe = safe[:60]
                elif budget < 75:
                    safe = safe[:100]
            except (ValueError, TypeError):
                pass
        lines.append(safe)

    # Line 2: structure summary [type: key=val; key=val]
    meta_parts = []
    if meta:
        for k, v in meta.items():
            safe_v = str(v).replace("|", "/").replace("\n", " ").strip()
            if safe_v:
                meta_parts.append(f"{k}={safe_v}")
    meta_str = ";".join(meta_parts)
    if len(meta_str) > 300:
        meta_str = meta_str[:297] + "..."
    # Append center to structure line if present
    if center:
        meta_str = f"{meta_str};center={center}" if meta_str else f"center={center}"
    lines.append(f"[{ccr_type}: {meta_str}]" if meta_str else f"[{ccr_type}]")

    # Line 3: CCR marker
    parts = [hash_val, ccr_type, str(size)]
    if mode:
        parts.append(mode)
    lines.append(f"<<<CCR:{'|'.join(parts)}>>>")

    return "\n".join(lines)


def _compress_via_proxy(content, target_port, headers=None):
    """Compress content through proxy CCR. Returns (hash, compressed_size) or None.

    Uses a thread-local keep-alive HTTP connection pool to avoid TCP handshake
    overhead on repeated calls to the same proxy port.

    Sends raw bytes with ``Content-Type: application/octet-stream`` to skip
    JSON serialization overhead - the proxy reads the request body directly.
    """
    try:
        data = content.encode("utf-8")
        conn = _get_conn(target_port)
        hdrs = {"Content-Type": "application/octet-stream", "Connection": "keep-alive"}
        if headers:
            for k, v in headers.items():
                if k.lower().startswith("x-headroom-"):
                    hdrs[k] = str(v)
        conn.request(
            "POST",
            "/ccr/create",
            body=data,
            headers=hdrs,
        )
        r = conn.getresponse()
        ccr = json.loads(r.read(4096))
        r.close()
        _tls.conn_ts[target_port] = time.time()
        if "error" in ccr:
            _log.debug("_compress_via_proxy: proxy returned error on port %s - %s", target_port, ccr.get("error"))
            return None
        return ccr["hash"], len(content)
    except Exception:
        _put_conn(target_port)  # ditch broken connection, reopen fresh next time
        return None


def _is_valid_ccr_hash(h):
    """Check if h is a valid CCR hash.

    Fast-reject short strings before regex; the regex enforces >=24 hex chars
    for proxy hashes (>=6 hex for i: inline hashes).
    """
    if not h or len(h) < 8:
        return False
    return bool(_VALID_HASH_RE.match(h.lower()))


def _parse_ccr_markers(text):
    """Parse <<<CCR:hash|type|size|key=value|...>>> markers from text.

    Returns list of dicts each containing:
        hash, type, size, mode, preview, meta

    ``meta`` is a dict with all key=value pairs found past parts[3],
    including ``preview`` (which is stored both in ``preview`` and
    ``meta["preview"]`` for backward compat).
    """
    global _parse_errors
    markers = []
    for match in _CCR_RE.finditer(text):
        full = match.group(0)
        # Extract inner content between CCR: and the closing delimiter
        inner = full.split("CCR:", 1)[1]
        for suffix in (">>>", "]", "⫸"):
            if inner.endswith(suffix):
                inner = inner[: -len(suffix)]
                break
        parts = inner.split("|")
        h = parts[0]  # hash from first pipe-delimited field
        if len(parts) >= 3:
            try:
                sz = int(parts[2])
                # Parse mode + key=value pairs from parts[3:]
                mode = "?"
                meta = {}
                for part in parts[3:]:
                    if "=" in part:
                        k, _, v = part.partition("=")
                        meta[str(k)] = str(v)
                    elif mode == "?":
                        mode = str(part)
                    else:
                        # Extra positional part after mode  -  ignore
                        pass
                preview = meta.pop("preview", "")
                # Remaining kv pairs are structured metadata (e.g. lang=rs, fns=main)
                parsed_meta = meta if meta else {}
                markers.append(
                    {
                        "hash": h,
                        "type": str(parts[1]),
                        "size": sz,
                        "mode": mode,
                        "preview": preview,
                        "meta": parsed_meta,
                    }
                )
            except ValueError:
                _parse_errors += 1
                _log.debug("_parse_ccr_markers: malformed marker skipped in %d-char text", len(text) if isinstance(text, str) else 0)
    # Filter: real CCR hashes are hex (0-9,a-f), >=8 chars,
    # or start with "i:" followed by pure hex.
    return [m for m in markers if _is_valid_ccr_hash(m["hash"])]
