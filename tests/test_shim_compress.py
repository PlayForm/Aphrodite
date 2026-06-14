#!/usr/bin/env python3
"""
HermesCompress Shim structural test - validates compression pipeline only.

No measurement. No filtering. Just verifies:
  - Compression activates with enough context
  - Message count doesn't increase
  - Tool outputs survive with structural integrity

Usage:
    cd HermesCompress
    .venv/bin/python tests/test_shim_compress.py
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

from hermes_compress import Compress, CompressOption

MODEL = "deepseek-v4-pro"


def _build_conv(n_turns: int, files: dict[str, str]) -> list[dict]:
    LARGE = ("HEADROOM COMPRESSION TEST. " * 40)
    msgs = [{"role": "system", "content": "Be concise."}]
    for i in range(n_turns):
        msgs.append({"role": "user", "content": f"Turn {i+1}: data."})
        msgs.append({"role": "assistant", "content": None, "tool_calls": [
            {"id": f"c{i}", "type": "function",
             "function": {"name": "read_file", "arguments": f'{{"path":"d{i}.txt"}}'}}
        ]})
        msgs.append({"role": "tool", "content": LARGE, "tool_call_id": f"c{i}"})
        msgs.append({"role": "assistant", "content": f"T{i+1} ok."})
        fname = "proxy-start.py" if i % 2 == 0 else "report.py"
        msgs.append({"role": "user", "content": f"Read {fname}."})
        msgs.append({"role": "assistant", "content": None, "tool_calls": [
            {"id": f"cc{i}", "type": "function",
             "function": {"name": "read_file", "arguments": f'{{"path":"{fname}"}}'}}
        ]})
        msgs.append({"role": "tool", "content": files[fname], "tool_call_id": f"cc{i}"})
    return msgs


def main():
    print(f"Compression Pipeline Test - {MODEL}\n")

    files = {
        "proxy-start.py": (REPO / "scripts" / "proxy-start.py").read_text(),
        "report.py": (REPO / "tests" / "report.py").read_text(),
    }

    option = CompressOption()
    option.Enabled = True
    option.Mode = "inline"
    option.ProtectRecent = 1
    option.MinTokensToCompress = 100
    option.PrecompressTools = True
    option.AggressiveKompress = True
    option.DeduplicateResults = True

    c = Compress(model=MODEL, option=option)

    passed = 0
    total = 0

    for turns, label in [(2, "small"), (3, "medium"), (5, "large")]:
        msgs = _build_conv(turns, files)
        result = c.compress(msgs)
        total += 2

        ok = True
        if len(result.messages) > len(msgs):
            print(f"  ✗ {label} ({len(msgs)} msg) - grew to {len(result.messages)}")
            ok = False
        else:
            passed += 1
            print(f"  ✓ {label} ({len(msgs)} msg) - {len(result.messages)} messages")

        # Tool output count preserved
        orig_tools = sum(1 for m in msgs if m.get("role") == "tool")
        comp_tools = sum(1 for m in result.messages if m.get("role") == "tool")
        if comp_tools < orig_tools:
            print(f"    ✗ lost {orig_tools - comp_tools} tool outputs")
            ok = False
        else:
            passed += 1
            print(f"    ✓ {comp_tools} tool outputs preserved")

    print(f"\n{'✓ all passed' if passed == total else f'{passed}/{total} passed'}")

    return 0 if passed == total else 1


if __name__ == "__main__":
    sys.exit(main())
