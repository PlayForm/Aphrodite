"""aphrodite — file reference tracking and listing tool."""

import json
import logging
import os

from .._core import _FILE_TOOLS, _referenced_files

_log = logging.getLogger("aphrodite.hooks.files")


def _fmt_files(data: dict) -> str:
    """Format files JSON into a readable markdown summary."""
    files = data.get("files", [])
    count = data.get("count", len(files) if isinstance(files, list) else 0)

    if count == 0:
        return "Referenced Files: 0 files"

    lines = [f"Referenced Files: {count} files", ""]

    if isinstance(files, dict):
        for tool, paths in files.items():
            lines.append(f"{tool}:")
            for p in paths[:20]:
                lines.append(f"  {p}")
    elif isinstance(files, list):
        for f in files[:30]:
            if isinstance(f, dict):
                lines.append(f"  {f.get('path', '')} ({f.get('tool', '')})")
            else:
                lines.append(f"  {f}")

    return "\n".join(lines)


def _track_file_refs(tool_name, args):
    """Track file paths referenced by tool calls. Uses OrderedDict LRU eviction."""
    if tool_name not in _FILE_TOOLS:
        return
    args = args if isinstance(args, dict) else {}
    path = args.get("path", args.get("file", ""))
    if path and isinstance(path, str) and len(path) < 500:
        _referenced_files[path] = tool_name
        _referenced_files.move_to_end(path)
        if len(_referenced_files) > 200:
            _referenced_files.popitem(last=False)


def _files_handler(args=None, **kwargs):
    """List all files referenced in the current session."""
    if not _referenced_files:
        return json.dumps({"files": [], "count": 0, "hint": "No file operations yet"})
    by_tool = {}
    for path, tool in sorted(_referenced_files.items()):
        by_tool.setdefault(tool, []).append(path)
    return json.dumps({
        "count": len(_referenced_files),
        "by_tool": {t: sorted(paths) for t, paths in sorted(by_tool.items())},
        "all": sorted(_referenced_files.keys()),
    })


FILES_SCHEMA = {
    "name": "aphrodite_files",
    "description": "List all file paths referenced in the current session. "
    "Grouped by tool type. Use to see what files have been touched before making decisions.",
    "parameters": {"type": "object", "properties": {}},
}
