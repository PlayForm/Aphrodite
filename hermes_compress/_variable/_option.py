"""
Default compression configuration.

Mirrors the TypeScript Variable/Option.ts pattern.
"""

from __future__ import annotations

from hermes_compress._option import CompressOption, CompressToolHint

# ── Default option ──────────────────────────────────────────────────────────

DefaultOption = CompressOption(
    Enabled=False,
    Mode="inline",
    ProtectRecent=1,
    TargetRatio=None,
    MinTokensToCompress=250,
    Threshold=0,
    ProxyPort=8787,
    ProxyHost="127.0.0.1",
    ProxyAutoStart=False,
    PrecompressTools=False,
    AggressiveKompress=False,
    DeduplicateResults=False,
    VerboseStats=False,
)

# ── Default tool hints ──────────────────────────────────────────────────────

ToolHints: dict[str, CompressToolHint] = {
    # Terminal & execution
    "terminal": CompressToolHint(Name="terminal", Hint="mixed", MinSize=200),
    "execute_code": CompressToolHint(Name="execute_code", Hint="mixed", MinSize=200),
    "read_terminal": CompressToolHint(Name="read_terminal", Hint="mixed", MinSize=300),
    "process": CompressToolHint(Name="process", Hint="json", MinSize=200),

    # File operations
    "read_file": CompressToolHint(Name="read_file", Hint="code", MinSize=300),
    "search_files": CompressToolHint(Name="search_files", Hint="json", MinSize=250),
    "write_file": CompressToolHint(Name="write_file", Hint="skip", MinSize=99999),
    "patch": CompressToolHint(Name="patch", Hint="code", MinSize=200),

    # Web tools
    "web_search": CompressToolHint(Name="web_search", Hint="json", MinSize=250),
    "web_extract": CompressToolHint(Name="web_extract", Hint="prose", MinSize=500),
    "browser_navigate": CompressToolHint(Name="browser_navigate", Hint="html", MinSize=500),
    "browser_snapshot": CompressToolHint(Name="browser_snapshot", Hint="html", MinSize=500),
    "browser_console": CompressToolHint(Name="browser_console", Hint="mixed", MinSize=200),
    "browser_click": CompressToolHint(Name="browser_click", Hint="skip", MinSize=99999),
    "browser_type": CompressToolHint(Name="browser_type", Hint="skip", MinSize=99999),
    "browser_scroll": CompressToolHint(Name="browser_scroll", Hint="skip", MinSize=99999),
    "browser_press": CompressToolHint(Name="browser_press", Hint="skip", MinSize=99999),
    "browser_back": CompressToolHint(Name="browser_back", Hint="skip", MinSize=99999),
    "browser_get_images": CompressToolHint(Name="browser_get_images", Hint="json", MinSize=200),
    "browser_vision": CompressToolHint(Name="browser_vision", Hint="skip", MinSize=99999),

    # Agent operations
    "delegate_task": CompressToolHint(Name="delegate_task", Hint="prose", MinSize=300),
    "session_search": CompressToolHint(Name="session_search", Hint="json", MinSize=250),
    "memory": CompressToolHint(Name="memory", Hint="skip", MinSize=99999),
    "todo": CompressToolHint(Name="todo", Hint="json", MinSize=100),
    "clarify": CompressToolHint(Name="clarify", Hint="skip", MinSize=99999),
    "cronjob": CompressToolHint(Name="cronjob", Hint="json", MinSize=200),
    "skill_view": CompressToolHint(Name="skill_view", Hint="prose", MinSize=500),
    "skills_list": CompressToolHint(Name="skills_list", Hint="json", MinSize=100),
    "skill_manage": CompressToolHint(Name="skill_manage", Hint="json", MinSize=100),

    # Computer use
    "vision_analyze": CompressToolHint(Name="vision_analyze", Hint="skip", MinSize=99999),
}
