#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""pre_llm_call hook: fast context injection from .hermes archive.

Reads the incoming prompt, searches .hermes for related sessions/skills/
memory/reference files, and returns a JSON {"context": "..."} that Hermes
injects into the user message. In an Aphrodite session this surfaces prior
CCR work, hook contracts, and the repo's dev skills (.hermes/skills/,
Development branch) as context before the model responds.

Drop into ~/.hermes/agent-hooks/prompt-context.py, add to config.yaml:

  hooks:
    pre_llm_call:
      - command: ~/.hermes/agent-hooks/prompt-context.py
        timeout: 15

Then allowlist: hermes hooks doctor (will prompt on first run).
"""

import json
import os
import re
import sys
import time
from pathlib import Path

HERMES_HOME = Path(os.path.expanduser("~/.hermes"))
CACHE_DIR = HERMES_HOME / "cache" / "prompt-context"
CACHE_TTL = 1800  # 30 minutes

STOP_WORDS = frozenset(
    {
        "the",
        "and",
        "for",
        "you",
        "this",
        "that",
        "with",
        "from",
        "have",
        "not",
        "but",
        "what",
        "let",
        "find",
        "add",
        "need",
        "want",
        "check",
        "run",
        "make",
        "just",
        "will",
        "would",
        "should",
        "could",
        "does",
        "did",
        "also",
        "then",
        "them",
        "than",
        "when",
        "where",
        "which",
        "while",
        "about",
        "into",
        "over",
        "after",
        "your",
        "some",
        "more",
        "each",
        "only",
        "used",
        "using",
        "been",
        "all",
        "any",
        "are",
        "has",
        "how",
        "its",
        "may",
        "out",
        "put",
        "set",
        "way",
        "see",
        "too",
        "now",
        "new",
        "old",
        "one",
        "get",
        "try",
        "use",
        "was",
        "were",
        "it",
        "in",
        "on",
        "at",
        "to",
        "of",
        "no",
        "my",
        "his",
        "her",
        "our",
    }
)

SIMPLE_PATTERNS = [
    r"^thank",
    r"^thanks",
    r"^ok\b",
    r"^sure",
    r"^yes[!.]*$",
    r"^no[!.]*$",
    r"^hello",
    r"^hi[!]*$",
    r"^hey",
    r"^good (morning|evening|afternoon)",
    r"^what\'?s up",
    r"^how are you",
    r"^who are you",
    r"^\d+$",
]


def should_search(prompt):
    p = prompt.strip()
    if len(p) < 12:
        return False
    for pat in SIMPLE_PATTERNS:
        if re.match(pat, p, re.IGNORECASE):
            return False
    return True


def extract_keywords(prompt):
    words = re.findall(r"[a-zA-Z]{4,}", prompt.lower())
    words = [w for w in words if w not in STOP_WORDS]
    return list(dict.fromkeys(words))[:12]


def cache_key(keywords):
    return "|".join(sorted(keywords[:6])).replace("|", "_")


def get_cached(keywords):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    path = CACHE_DIR / f"{cache_key(keywords)}.json"
    if path.exists():
        try:
            entry = json.loads(path.read_text())
            if time.time() - entry.get("ts", 0) < CACHE_TTL:
                return entry.get("ctx")
        except Exception:
            pass
    return None


def set_cache(keywords, context):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    path = CACHE_DIR / f"{cache_key(keywords)}.json"
    try:
        json.dump({"ctx": context, "ts": time.time()}, path.open("w"))
    except Exception:
        pass


def search_sessions(keywords, limit=3):
    sessions_dir = HERMES_HOME / "sessions"
    if not sessions_dir.exists():
        return []
    results = []
    for sf in sorted(
        sessions_dir.glob("session_*.json"), key=lambda p: p.stat().st_mtime, reverse=True
    )[:25]:
        try:
            data = json.loads(sf.read_text(errors="ignore"))
            user_content = " ".join(
                m.get("content", "") for m in data.get("messages", []) if m.get("role") == "user"
            ).lower()
            hits = [kw for kw in keywords if kw in user_content]
            if len(hits) < 2:
                continue
            title = data.get("title", "")
            for m in data.get("messages", []):
                if m.get("role") == "user":
                    title = title or m.get("content", "").strip()[:120]
                    break
            date = ""
            dm = re.search(r"(\d{8})", sf.name)
            if dm and len(dm.group(1)) == 8:
                raw = dm.group(1)
                date = f"{raw[:4]}-{raw[4:6]}-{raw[6:]}"
            results.append(
                {
                    "date": date,
                    "summary": title,
                    "match_keywords": hits[:3],
                }
            )
            if len(results) >= limit:
                break
        except Exception:
            continue
    return results


def search_skills(keywords):
    skills_dir = HERMES_HOME / "skills"
    if not skills_dir.exists():
        return []
    results = []
    for sf in skills_dir.rglob("SKILL.md"):
        try:
            content = sf.read_text(errors="ignore")
            hits = [kw for kw in keywords if kw in content.lower()]
            if len(hits) < 2:
                continue
            name = sf.parent.name
            desc = ""
            for line in content.split("\n")[:15]:
                s = line.strip()
                if s.startswith("name:"):
                    name = s.split(":", 1)[1].strip().strip('"')
                elif s.startswith("description:"):
                    desc = s.split(":", 1)[1].strip().strip('"')[:120]
            results.append(
                {
                    "name": name,
                    "description": desc,
                    "match_keywords": hits[:2],
                }
            )
        except Exception:
            continue
    return results[:5]


def search_memory(keywords):
    mem_file = HERMES_HOME / "memory"
    if not mem_file.exists():
        return []
    try:
        hits = []
        for line in mem_file.read_text(errors="ignore").split("\n"):
            s = line.strip()
            if not s or s.startswith("\xc2\xa7"):
                continue
            if sum(1 for kw in keywords if kw in s.lower()) >= 2:
                hits.append(s[:150])
        return hits[:5]
    except Exception:
        return []


def search_reference_files(keywords):
    results = []
    for mf in HERMES_HOME.glob("*.md"):
        if not any(x in mf.name.lower() for x in ["aphrodite-", "reference", "audit", "codebase"]):
            continue
        try:
            content = mf.read_text(errors="ignore").lower()
            hits = [kw for kw in keywords if kw in content]
            if len(hits) < 2:
                continue
            header = " ".join(
                l.strip() for l in mf.read_text(errors="ignore").split("\n")[:3] if l.strip()
            )[:120]
            results.append({"file": mf.name, "header": header, "match_keywords": hits[:2]})
        except Exception:
            continue
    return results[:3]


def build_context(sessions, skills, memory, references):
    parts = []
    if sessions:
        lines = [
            f"  [{s['date']}] {s['summary']} (matched: {', '.join(s['match_keywords'])})"
            for s in sessions
        ]
        parts.append("Related past sessions:\n" + "\n".join(lines))
    if skills:
        lines = []
        for s in skills:
            desc = f" - {s['description']}" if s["description"] else ""
            lines.append(f"  {s['name']}{desc} (matched: {', '.join(s['match_keywords'])})")
        parts.append("Relevant installed skills:\n" + "\n".join(lines))
    if memory:
        parts.append("Relevant memory entries:\n" + "\n".join(f"  {m}" for m in memory))
    if references:
        lines = [
            f"  {r['file']} - {r['header']} (matched: {', '.join(r['match_keywords'])})"
            for r in references
            if r["header"]
        ]
        if lines:
            parts.append("Reference files in .hermes:\n" + "\n".join(lines))
    return "\n\n".join(parts) if parts else None


def main():
    try:
        raw = sys.stdin.read()
        if not raw.strip():
            return
        payload = json.loads(raw)
    except (json.JSONDecodeError, ValueError):
        return

    prompt = payload.get("prompt", "")
    if not prompt or not should_search(prompt):
        return

    keywords = extract_keywords(prompt)
    if len(keywords) < 2:
        return

    cached = get_cached(keywords)
    if cached:
        print(json.dumps({"context": f"PRIOR CONTEXT FROM .HERMES (cached):\n{cached}"}))
        return

    sessions = search_sessions(keywords, limit=4)
    skills = search_skills(keywords)
    memory = search_memory(keywords)
    references = search_reference_files(keywords)
    context = build_context(sessions, skills, memory, references)
    if not context:
        return

    set_cache(keywords, context)
    print(
        json.dumps(
            {
                "context": f"PRIOR CONTEXT FROM YOUR .HERMES ARCHIVE:\nBefore responding, consider this relevant content:\n\n{context}"
            }
        )
    )


if __name__ == "__main__":
    main()
