"""aphrodite — live containers: streaming terminal output."""

import hashlib
import json
import logging
import subprocess as _subprocess
import threading as _live_threading

_log = logging.getLogger("aphrodite.hooks.live")

_live_containers = {}  # {hash: {status, chunks, command, pid}}


def _create_live_container(command, env=None):
    """Create a background live container that streams terminal output."""
    hash_val = hashlib.sha256(command.encode()).hexdigest()[:16]
    chunks = []

    def _run():
        try:
            proc = _subprocess.Popen(
                command, shell=True, stdout=_subprocess.PIPE,
                stderr=_subprocess.STDOUT, text=True, bufsize=1,
            )
            _live_containers[hash_val] = {
                "status": "running", "chunks": chunks,
                "command": command, "pid": proc.pid,
            }
            for line in proc.stdout:
                chunks.append(line)
            proc.wait()
            _live_containers[hash_val]["status"] = (
                "done" if proc.returncode == 0 else f"exit={proc.returncode}"
            )
            _live_containers[hash_val]["exit_code"] = proc.returncode
        except Exception as e:
            _live_containers[hash_val] = {
                "status": "error", "error": str(e), "command": command,
            }

    _live_threading.Thread(target=_run, daemon=True).start()
    return (
        f"<<<LIVE:{hash_val}|terminal|streaming>>> "
        f"[terminal:{command[:40]} ...running]"
    )


def _live_container_handler(args=None, **kwargs):
    """Poll a live CCR container for streaming output."""
    args = args if isinstance(args, dict) else {}
    hash_val = args.get("hash", "")
    if not hash_val or hash_val not in _live_containers:
        return json.dumps({"error": f"container {hash_val} not found"})
    c = _live_containers[hash_val]
    output = "".join(c.get("chunks", []))
    return json.dumps({
        "hash": hash_val,
        "status": c.get("status", ""),
        "command": c.get("command", ""),
        "output": output[-4096:],
        "output_len": len(output),
        "pid": c.get("pid"),
    }, indent=2)


LIVE_CONTAINER_SCHEMA = {
    "name": "aphrodite_poll_container",
    "description": "Poll a live CCR container for streaming output.",
    "parameters": {
        "type": "object",
        "properties": {
            "hash": {
                "type": "string",
                "description": "Hash from <<<LIVE:hash>>> marker",
            }
        },
        "required": ["hash"],
    },
}
