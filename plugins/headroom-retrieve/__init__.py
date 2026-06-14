"""headroom-retrieve — Hermes plugin. Local read first, proxy fallback."""
import json, os, urllib.request

PROXY = "http://127.0.0.1:8788"

SCHEMA = {
    "name": "headroom_retrieve",
    "description": "Retrieve original content behind CCR markers like '<<ccr:abc,string,5KB>>'. Include `path` for instant local read; omit for proxy cache.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {"type": "string", "description": "CCR marker or hash"},
            "path": {"type": "string", "description": "File path for local read"},
        },
        "required": ["hash"],
    },
}

def _hash(raw):
    for s in ("<<ccr:", "hash="):
        if s in raw:
            return raw.split(s, 1)[1].split(",")[0].rstrip(">")
    return raw.strip("<>").split(",")[0]

def _file(path):
    if not path or not os.path.isfile(path):
        return None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        return "".join(f"{i+1}|{l}" for i, l in enumerate(lines))
    except Exception:
        return None

def _proxy(h):
    try:
        req = urllib.request.Request(f"{PROXY}/v1/retrieve",
            data=json.dumps({"hash": h}).encode(),
            headers={"Content-Type": "application/json"})
        c = json.loads(urllib.request.urlopen(req, timeout=5).read()).get("original_content", "")
        if c and not c.lstrip()[:6] == "<<ccr:":
            return c
    except Exception:
        pass
    return None

def _handle_headroom_retrieve(args, **kw):
    h = _hash(str(args.get("hash", "")))
    if not h:
        return json.dumps({"error": "no hash"})
    p = str(args.get("path", "")).strip()

    c = _file(p) if p else None
    if c:
        return json.dumps({"content": c, "source": "local"})
    c = _proxy(h)
    if c:
        return json.dumps({"content": c, "source": "proxy"})
    return json.dumps({"error": "expired" if p else "expired — re-run command"})

def register(ctx):
    ctx.register_tool(name="headroom_retrieve", toolset="headroom", schema=SCHEMA, handler=_handle_headroom_retrieve, emoji="🗜️")
