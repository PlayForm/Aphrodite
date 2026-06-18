"""
Example extension by a 3rd-party developer.

Installs custom effects into the aphrodite runtime pipelines.
NO adapter.py changes needed — just import runtime and register.

Usage:
  from example_extension import install
  install()
"""

from effects import Effect, runtime
import time


# ── Effect 1: sanitize (runs BEFORE dylib — cleans input) ────────────────

def make_sanitize(args: dict) -> Effect:
    """Strip whitespace from content fields."""
    def _clean():
        if "content" in args:
            args["content"] = args["content"].strip()
        return args
    return Effect.sync(_clean)


# ── Effect 2: log (runs AFTER dylib — observes output) ───────────────────

def make_log(result: dict) -> Effect:
    """Log the hook result via Python logging."""
    def _log():
        import logging
        logging.getLogger("ext.example").info(
            "hook result status=%s", result.get("status", "?")
        )
        return result
    return Effect.sync(_log)


# ── Effect 3: enrich (runs AFTER dylib — adds metadata) ──────────────────

def make_enrich(result: dict) -> Effect:
    """Add extension metadata to every result."""
    def _enrich():
        result["_extended_by"] = "example-extension v1.0"
        result["_timestamp"] = time.time()
        return result
    return Effect.sync(_enrich)


# ── Install into runtime pipelines ────────────────────────────────────────

def install():
    """
    Register extension effects into the runtime's pipelines.

    Pipeline order after install:
      transform_tool_result: [sanitize] → [dylib call] → [log] → [enrich]
    """
    for hook in ("transform_tool_result", "transform_terminal_output"):
        runtime.prepend(hook, make_sanitize)
        runtime.append(hook, make_log)
        runtime.append(hook, make_enrich)

    print(f"[example-extension] installed — {runtime.list_pipelines()}")


# ── Smoke test ────────────────────────────────────────────────────────────
if __name__ == "__main__":
    # Import adapter to bootstrap runtime first
    import adapter
    adapter.bootstrap()

    # Install extension
    install()

    # Run through the pipeline
    r = runtime.run_exit("transform_tool_result",
                         {"content": "  error: something broke  \nline2", "tool_name": "test"})
    print(f"\nresult: {r}")
    val = r.get("value", {})
    print(f"  status:        {val.get('status')}")
    print(f"  _extended_by:  {val.get('_extended_by', 'MISSING')}")
    print(f"  _timestamp:    {val.get('_timestamp', 'MISSING')}")
    print(f"  preview:       {val.get('preview', 'MISSING')}")
