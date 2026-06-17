"""Content classification — detect type from raw content string."""

import json
import logging
import re

_log = logging.getLogger("aphrodite")


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
        trimmed = content[:5000]

        # ── diff content ─────────────────────────────────────
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

        # ── Terminal output (exit code pattern) ──────────────
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

        # ── Build output ─────────────────────────────────────
        if any("Compiling" in line or "Compiling" in line for line in lines[:30]):
            meta = {"type": "build_output", "ln": str(ln)}
            error_count = sum(1 for line in lines if "error[" in line or line.strip().startswith("error:"))
            warning_count = sum(1 for line in lines if "warning:" in line)
            meta["errors"] = str(error_count)
            meta["warnings"] = str(warning_count)
            return meta

        # ── Rust build errors ────────────────────────────────
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

        # ── JSON content ──────────────────────────────────────
        stripped = trimmed.strip()
        if stripped.startswith("{") or stripped.startswith("["):
            try:
                data = json.loads(stripped)
                if isinstance(data, dict):
                    if "total_count" in data:
                        meta = {"type": "search_results"}
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        meta["total"] = str(data["total_count"])
                        return meta
                    if "session_id" in data:
                        meta = {"type": "process_output"}
                        meta["pid"] = str(data.get("pid", data.get("process_id", "?")))
                        if "uptime" in data:
                            meta["uptime"] = str(data["uptime"])
                        return meta
                    if "exit_code" in data or "output" in data:
                        meta = {"type": "terminal"}
                        if "exit_code" in data:
                            meta["exit"] = str(data["exit_code"])
                        if "output" in data and isinstance(data["output"], str):
                            last = data["output"].splitlines()
                            if last:
                                meta["last"] = last[-1][:60]
                        return meta
                    if "matches" in data:
                        meta = {"type": "search_files"}
                        meta["files"] = str(len(data["matches"])) if isinstance(data["matches"], (list, tuple)) else str(data["matches"])
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        return meta
                    if data.get("status") == "written" or ("path" in data and "bytes" in data):
                        meta = {"type": "write_file", "ln": str(ln)}
                        meta["fn"] = str(data.get("path", ""))
                        if "bytes" in data:
                            meta["size"] = str(data["bytes"])
                        if "syntax_errors" in data:
                            errs = data["syntax_errors"]
                            meta["errors"] = str(len(errs)) if isinstance(errs, list) else str(errs)
                        return meta
                    if "level" in data or ("entries" in data and isinstance(data.get("entries"), list)):
                        meta = {"type": "log", "ln": str(ln)}
                        entries = data.get("entries", [data])
                        if isinstance(entries, list):
                            meta["entries"] = str(len(entries))
                            errs = sum(1 for e in entries if isinstance(e, dict) and e.get("level") == "error")
                            warns = sum(1 for e in entries if isinstance(e, dict) and e.get("level") == "warn")
                            if errs:
                                meta["errors"] = str(errs)
                            if warns:
                                meta["warnings"] = str(warns)
                        return meta
                    if "elements" in data and isinstance(data.get("elements"), list):
                        meta = {"type": "browser_snapshot"}
                        meta["elements"] = str(len(data["elements"]))
                        if "total_elements" in data:
                            meta["total"] = str(data["total_elements"])
                        return meta
                    if "title" in data and "url" in data or ("results" in data and isinstance(data.get("results"), list)):
                        meta = {"type": "web_search", "ln": str(ln)}
                        results = data.get("results", [data] if "title" in data else [])
                        if isinstance(results, list):
                            meta["total"] = str(len(results))
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        return meta
                    if "image" in data or "prompt" in data:
                        meta = {"type": "image_generate", "ln": str(ln)}
                        if "prompt" in data:
                            meta["msg"] = str(data["prompt"])[:80]
                        return meta
                    if "todos" in data or ("id" in data and "content" in data and "status" in data):
                        meta = {"type": "todo", "ln": str(ln)}
                        todos = data.get("todos", [])
                        if isinstance(todos, list):
                            meta["items"] = str(len(todos))
                            meta["total"] = str(sum(1 for t in todos if isinstance(t, dict) and t.get("status") != "completed"))
                        elif "status" in data:
                            meta["items"] = "1"
                        return meta
                    if "success" in data and ("target" in data or "entries" in data):
                        meta = {"type": "memory", "ln": str(ln)}
                        entries = data.get("entries", [])
                        if isinstance(entries, list):
                            meta["items"] = str(len(entries))
                        return meta
                    if "schedule" in data and ("id" in data or "job_id" in data):
                        meta = {"type": "cronjob", "ln": str(ln)}
                        if "status" in data:
                            meta["msg"] = str(data["status"])
                        return meta
                    if "results" in data and ("query" in data or "total_count" in data):
                        meta = {"type": "search_results"}
                        if "total_count" in data:
                            meta["total"] = str(data["total_count"])
                        if "query" in data:
                            meta["q"] = str(data["query"])[:40]
                        results = data.get("results", [])
                        if isinstance(results, list):
                            meta["files"] = str(len(results))
                        return meta
                    keys = list(data.keys())[:8]
                    meta = {"type": "json", "ln": str(ln)}
                    if keys:
                        meta["keys"] = ",".join(keys)
                    return meta
                elif isinstance(data, list):
                    return {"type": "json_list", "ln": str(ln), "len": str(len(data))}
            except (json.JSONDecodeError, ValueError):
                pass

        # ── Search output (file:line: text pattern) ──────────
        file_line_count = 0
        for line in lines[:200]:
            if re.match(r"^[^\s]+:\d+:", line):
                file_line_count += 1
        if file_line_count > 3 and file_line_count > ln * 0.3:
            return {"type": "search_files", "files": str(file_line_count), "ln": str(ln)}

        # ── Tabular/structured output ────────────────────────
        pipe_count = sum(1 for line in lines[:50] if "|" in line)
        if pipe_count >= 3 and pipe_count > ln * 0.2:
            return {"type": "tabular", "rows": str(pipe_count), "ln": str(ln)}

        # ── Error / traceback ────────────────────────────────
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
            if error_msg == "unknown" and lines:
                last = lines[-1].strip()
                if last:
                    error_msg = last[:80]
            return {"type": "error", "msg": error_msg, "ln": str(ln)}

        # ── Git commit log ───────────────────────────────────
        if ln >= 2 and re.match(r"^[a-f0-9]{7,40}\s", lines[0].strip()):
            parts = lines[0].strip().split(None, 2)
            if len(parts) >= 2:
                subj = parts[-1] if len(parts) > 1 else ""
                return {"type": "commit", "hash": parts[0][:8], "subject": subj[:80], "ln": str(ln)}

        # ── Fallback: text ───────────────────────────────────
        return {"type": "text", "ln": str(ln)}
    except Exception:
        if logging.getLogger("aphrodite").isEnabledFor(logging.DEBUG):
            logging.getLogger("aphrodite").debug("_classify_content: failed for %d-char content", len(content) if isinstance(content, str) else 0)
        return {"type": "text", "ln": str(len(content.splitlines())) if isinstance(content, str) else 0}
