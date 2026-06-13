#!/usr/bin/env python3
"""Dump Hermes session messages to cache for proxy benchmarks."""

import json
import sqlite3
import sys
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parent.parent
CACHE_DIR = REPO_DIR / ".hermes" / "cache"
CACHE_DIR.mkdir(parents=True, exist_ok=True)
DB_PATH = Path.home() / ".hermes" / "state.db"


def dump(session_id=None, limit=0, output="session.json"):
    conn = sqlite3.connect(str(DB_PATH))
    conn.row_factory = sqlite3.Row

    if not session_id:
        row = conn.execute(
            "SELECT id FROM sessions ORDER BY started_at DESC LIMIT 1"
        ).fetchone()
        if not row:
            print("No sessions found")
            return
        session_id = row["id"]

    query = "SELECT role, content FROM messages WHERE session_id=? ORDER BY id"
    if limit:
        query += f" LIMIT {limit}"

    rows = conn.execute(query, (session_id,)).fetchall()
    conn.close()

    messages = [{"role": r["role"], "content": r["content"] or ""} for r in rows if r["content"]]
    payload = {"session_id": session_id, "message_count": len(messages), "messages": messages}

    out = CACHE_DIR / output
    out.write_text(json.dumps(payload, indent=2))
    print(f"Session: {session_id}")
    print(f"Messages: {len(messages)}")
    print(f"Cache:    {out}")


if __name__ == "__main__":
    sid = next((a.split("=",1)[1] for a in sys.argv if a.startswith("--session=")), None)
    lim = int(next((a.split("=",1)[1] for a in sys.argv if a.startswith("--limit=")), 0) or 0)
    out = next((a.split("=",1)[1] for a in sys.argv if a.startswith("--output=")), "session.json")
    dump(sid, lim, out)
