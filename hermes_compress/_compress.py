"""
Hermes-specific headroom compression engine - inline + proxy modes.

Two modes, one API:
  Inline  - headroom runs as a library in-process (50-80ms warm, default)
  Proxy   - headroom runs as a separate server (port 8787, zero code changes)

Both modes share the same `compress(messages)` interface. Use the one
that fits your deployment.

Standalone usage (no Hermes needed):
    from hermes_compress import Compress
    c = Compress(model="deepseek-v4-pro", mode="inline")
    result = c.compress(messages)
    messages = result.messages

Proxy mode:
    from hermes_compress import Proxy
    proxy = Proxy(port=8787)
    proxy.start()
    # Then point your provider base_url to http://127.0.0.1:8787
    proxy.stop()
"""

from __future__ import annotations

import logging
import os
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass, field
from typing import Any, Optional

from hermes_compress._option import CompressOption

import re

logger = logging.getLogger(__name__)

# CCR marker pattern: <<ccr:hash,type,size>>
_CCR_RE = re.compile(r"<<ccr:[a-f0-9]+,[a-z]+,[\d.]+[KMGT]?B?>>")


def _strip_ccr_markers(messages: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Strip headroom CCR markers from message content.

    CCR markers (``<<ccr:hash,type,size>>``) are headroom's way of
    marking compressed content. Once a message has been sent to the LLM,
    these markers are pure overhead - they add tokens without adding
    information. Stripping them before the next compression pass
    recovers ~2-5% additional savings.
    """
    cleaned = []
    for msg in messages:
        content = msg.get("content")
        if isinstance(content, str) and _CCR_RE.search(content):
            msg = {**msg, "content": _CCR_RE.sub("", content).strip()}
        cleaned.append(msg)
    return cleaned


def _preprocess_messages(
    messages: list[dict[str, Any]],
    flags: Any,
) -> list[dict[str, Any]]:
    """Run pre-processing on messages based on dev flags."""
    from hermes_compress._preprocess import preprocess_message
    cleaned = []
    for msg in messages:
        content = msg.get("content")
        if not isinstance(content, str):
            cleaned.append(msg)
            continue
        if msg.get("role") == "tool":
            tool_name = msg.get("name", "")
            content = preprocess_message(
                content, tool_name,
                strip_ansi_enabled=flags.strip_ansi,
                truncate_repeats=flags.truncate_repeats,
                strip_debug=flags.strip_debug,
                compress_patterns=flags.compress_patterns,
                strip_ccr=flags.strip_ccr,
            )
        else:
            if flags.strip_ccr:
                content = _CCR_RE.sub("", content).strip()
        cleaned.append({**msg, "content": content})
    return cleaned


def _precompress_tools(
    messages: list[dict[str, Any]],
    model: str,
) -> tuple[list[dict[str, Any]], int]:
    """Pre-compress large tool outputs individually before the full pass.
    Double-pass gives ~5-15% extra savings on JSON/log-heavy sessions."""
    from headroom import compress as _hr_compress
    cleaned = []
    total_saved = 0
    for msg in messages:
        if msg.get("role") != "tool":
            cleaned.append(msg)
            continue
        content = msg.get("content", "")
        if not isinstance(content, str) or len(content) < 500:
            cleaned.append(msg)
            continue
        try:
            single = [{"role": "user", "content": content}]
            r = _hr_compress(single, model=model or "gpt-4o",
                             protect_recent=0, min_tokens_to_compress=100)
            if r.tokens_saved > 0 and r.messages:
                total_saved += r.tokens_saved
                cleaned.append({**msg, "content": r.messages[0]["content"]})
            else:
                cleaned.append(msg)
        except Exception:
            cleaned.append(msg)
    return cleaned, total_saved

# Global reference for the stats handler
_active_compressor: Optional["Compress"] = None


# ── Result ──────────────────────────────────────────────────────────────────


@dataclass
class CompressResult:
    """Result of compressing a message list."""

    messages: list[dict[str, Any]]
    tokens_before: int = 0
    tokens_after: int = 0
    tokens_saved: int = 0
    compression_ratio: float = 0.0
    duration_ms: float = 0.0
    transforms_applied: list[str] = field(default_factory=list)
    error: Optional[str] = None

    @property
    def compressed(self) -> bool:
        """True when tokens were saved and no error occurred."""
        return self.tokens_saved > 0 and self.error is None


# ── Tool-type routing ───────────────────────────────────────────────────────

TOOL_CONTENT_HINTS: dict[str, str] = {
    "terminal": "mixed",
    "read_file": "code",
    "search_files": "json",
    "web_search": "json",
    "web_extract": "prose",
    "browser_navigate": "html",
    "browser_snapshot": "html",
    "browser_console": "mixed",
    "browser_vision": "image",
    "write_file": "skip",
    "patch": "code",
    "session_search": "json",
    "delegate_task": "prose",
    "memory": "skip",
    "todo": "json",
    "execute_code": "mixed",
    "skill_view": "prose",
    "process": "json",
    "read_terminal": "mixed",
}

TOOL_MIN_SIZES: dict[str, int] = {
    "terminal": 200,
    "read_file": 300,
    "search_files": 250,
    "web_search": 250,
    "web_extract": 500,
    "browser_navigate": 500,
    "browser_snapshot": 500,
    "browser_console": 200,
    "execute_code": 200,
    "delegate_task": 300,
    "session_search": 250,
    "skill_view": 500,
    "process": 200,
    "read_terminal": 300,
    "__default__": 250,
}


# ── Proxy ───────────────────────────────────────────────────────────────────


class Proxy:
    """Manage a headroom proxy server as a subprocess.

    Usage:
        proxy = Proxy(port=8787)
        proxy.start()
        # ... use compression through proxy ...
        proxy.stop()

    The proxy runs headroom as a separate HTTP server. All compression
    happens out-of-process - point your LLM provider's base_url to
    ``http://127.0.0.1:{port}`` to route all traffic through it.
    """

    _instance: Optional["Proxy"] = None

    def __init__(
        self,
        port: int = 8787,
        host: str = "127.0.0.1",
        mode: str = "token",
        auto_start: bool = False,
    ) -> None:
        self.Port = port
        self.Host = host
        self.Mode = mode
        self._process: Optional[subprocess.Popen] = None
        self._headroom_path: str = ""

        Proxy._instance = self

        if auto_start:
            self.start()

    # ── Lifecycle ──────────────────────────────────────────────────────

    def start(self) -> bool:
        """Start the headroom proxy. Returns True if started successfully."""
        if self.running:
            logger.info("headroom proxy already running on :%d", self.Port)
            return True

        self._headroom_path = shutil.which("headroom") or "headroom"

        if not shutil.which("headroom"):
            logger.error("headroom CLI not found in PATH")
            return False

        try:
            self._process = subprocess.Popen(
                [
                    self._headroom_path, "proxy",
                    "--port", str(self.Port),
                    "--host", self.Host,
                    "--mode", self.Mode,
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                preexec_fn=os.setsid,  # detach from parent process group
            )
            # Give it a moment to bind
            time.sleep(1.5)
            if self.healthy:
                logger.info(
                    "headroom proxy started on http://%s:%d (mode=%s)",
                    self.Host, self.Port, self.Mode,
                )
                return True
            else:
                logger.error("headroom proxy failed to start")
                return False
        except Exception as exc:
            logger.error("headroom proxy start failed: %s", exc)
            return False

    def stop(self) -> None:
        """Stop the headroom proxy if running."""
        if self._process is None:
            return
        try:
            os.killpg(os.getpgid(self._process.pid), signal.SIGTERM)
            self._process.wait(timeout=5)
            logger.info("headroom proxy stopped")
        except Exception:
            try:
                self._process.kill()
            except Exception:
                pass
        finally:
            self._process = None

    # ── Status ─────────────────────────────────────────────────────────

    @property
    def running(self) -> bool:
        """True if the proxy process is running."""
        return (
            self._process is not None
            and self._process.poll() is None
        )

    @property
    def healthy(self) -> bool:
        """True if the proxy is responding to health checks."""
        if not self.running:
            return False
        try:
            import urllib.request
            url = f"http://{self.Host}:{self.Port}/health"
            req = urllib.request.Request(url, method="GET")
            with urllib.request.urlopen(req, timeout=5) as resp:
                return resp.status == 200
        except Exception:
            return False

    @property
    def base_url(self) -> str:
        """The URL to set as your provider's base_url for proxy mode."""
        return f"http://{self.Host}:{self.Port}"

    @staticmethod
    def get_instance() -> Optional["Proxy"]:
        """Get the singleton proxy instance."""
        return Proxy._instance


# ── Compressor ──────────────────────────────────────────────────────────────


class Compress:
    """Compression engine - inline or proxy.

    Two modes, identical API:

    - ``mode="inline"`` (default): headroom library runs in-process.
      Fast warm calls (~50-80ms), no network, no extra process.

    - ``mode="proxy"``: headroom runs as a separate proxy server.
      Zero code changes needed - point provider base_url at the proxy.
      Use ``Proxy`` class to manage the server.

    Standalone usage (no Hermes):
        c = Compress(model="deepseek-v4-pro")
        result = c.compress(messages)

    Parameters:
        model: LLM model name for token counting.
        option: Compression options (see CompressOption).
    """

    def __init__(
        self,
        model: str = "",
        option: Optional[CompressOption] = None,
    ) -> None:
        self.model = model
        self._option = option or CompressOption()
        self._headroom_available: Optional[bool] = None
        self._proxy: Optional[Proxy] = None

        self.stats: dict[str, Any] = {
            "calls": 0,
            "total_tokens_saved": 0,
            "total_duration_ms": 0.0,
            "by_tool": {},
        }

        global _active_compressor
        _active_compressor = self

        # Auto-start proxy if configured
        if self._option.Mode == "proxy" and self._option.ProxyAutoStart:
            self._proxy = Proxy(
                port=self._option.ProxyPort,
                host=self._option.ProxyHost,
                mode=self._option.Mode,
                auto_start=True,
            )

        # Auto-update check (non-blocking, silent on failure)
        try:
            from hermes_compress._update import auto_update_check
            auto_update_check()
        except Exception:
            pass

        # Track first-call cold start warning
        self._cold_start_warned = False

    # ── Public API ──────────────────────────────────────────────────────

    @property
    def enabled(self) -> bool:
        return self._option.Enabled

    @enabled.setter
    def enabled(self, value: bool) -> None:
        self._option.Enabled = value

    @property
    def mode(self) -> str:
        return self._option.Mode

    def update_model(self, model: str) -> None:
        self.model = model

    def compress(
        self, messages: list[dict[str, Any]]
    ) -> CompressResult:
        """Compress a message list. Routes to inline or proxy based on mode."""
        if not self._option.Enabled:
            return CompressResult(messages=messages)

        if not messages:
            return CompressResult(messages=messages)

        if self._option.Mode == "proxy":
            return self._compress_proxy(messages)
        return self._compress_inline(messages)

    def _compress_inline(
        self, messages: list[dict[str, Any]]
    ) -> CompressResult:
        """Compress using headroom library in-process.

        Pipeline: pre-process -> pre-compress tools -> headroom compress -> stats
        """
        if self._headroom_available is None:
            self._headroom_available = self._probe_headroom()

        if not self._headroom_available:
            return CompressResult(
                messages=messages,
                error="headroom library not available",
            )

        from hermes_compress._dev import is_dev, DevFlags, get_collector, CallStats

        dev = is_dev()
        flags = DevFlags.from_env() if dev else DevFlags()

        # Merge CompressOption advanced settings into flags
        if self._option.PrecompressTools:
            flags.precompress_tool_outputs = True
        if self._option.AggressiveKompress:
            flags.aggressive_kompress = True
        if self._option.DeduplicateResults:
            flags.deduplicate_tool_results = True
        if self._option.VerboseStats or dev:
            flags.verbose_stats = True

        stats = CallStats() if (dev or flags.verbose_stats) else None

        total_start = time.monotonic()
        chars_before = sum(len(str(m)) for m in messages)
        if stats:
            stats.messages_in = len(messages)
            stats.chars_before = chars_before

        try:
            # Phase 1: Headroom compression (MUST be first)
            # Save original tool content for safety guard against over-compression.
            # Key by tool_call_id (NOT index) — headroom may restructure the
            # message list, breaking index-based lookup.
            _orig_tool_content: dict[str, str] = {}
            for _m in messages:
                if _m.get("role") == "tool":
                    _c = _m.get("content", "")
                    _tcid = _m.get("tool_call_id", "")
                    if isinstance(_c, str) and _c.strip() and _tcid:
                        _orig_tool_content[_tcid] = _c

            compress_start = time.monotonic()
            from headroom import compress as _headroom_compress

            if not self._cold_start_warned and self._headroom_available:
                self._cold_start_warned = True
                logger.warning(
                    "hermes-compress: first call -- headroom is loading compression models "
                    "(Kompress ONNX). This may add 10-15 seconds to this request. "
                    "Subsequent calls will be fast (~50-80ms)."
                )

            from hermes_compress._strategies import get_strategy
            tool_strategy = get_strategy("", dev_mode=dev)
            for msg in messages:
                if msg.get("role") == "tool":
                    ts = get_strategy(msg.get("name", ""), dev_mode=dev)
                    if ts.get("protect_recent", 4) < tool_strategy.get("protect_recent", 4):
                        tool_strategy = ts

            kwargs: dict[str, Any] = {
                "protect_recent": min(self._option.ProtectRecent, tool_strategy.get("protect_recent", self._option.ProtectRecent)),
                "min_tokens_to_compress": min(self._option.MinTokensToCompress, tool_strategy.get("min_tokens_to_compress", self._option.MinTokensToCompress)),
            }
            if self._option.TargetRatio is not None:
                kwargs["target_ratio"] = self._option.TargetRatio
            elif tool_strategy.get("target_ratio") is not None:
                kwargs["target_ratio"] = tool_strategy["target_ratio"]

            result = _headroom_compress(messages, model=self.model or "gpt-4o", **kwargs)
            compress_ms = (time.monotonic() - compress_start) * 1000

            # Phase 2: Post-processing — strip CCR markers only
            pp_start = time.monotonic()
            messages = _strip_ccr_markers(result.messages)

            # Phase 2a: Safety guard — revert tool outputs compressed to empty.
            # Headroom may produce empty strings when Kompress fails on specific
            # content types. Revert only empty content — CCR markers are valid
            # compressed output that the LLM understands natively.
            _empty_guard_count = 0
            for _i, _m in enumerate(messages):
                if _m.get("role") != "tool":
                    continue
                _content = _m.get("content", "")
                _tcid = _m.get("tool_call_id", "")
                _orig = _orig_tool_content.get(_tcid)
                if not _orig:
                    continue
                if isinstance(_content, str) and not _content.strip():
                    messages[_i] = {**_m, "content": _orig}
                    _empty_guard_count += 1
            if _empty_guard_count > 0:
                logger.warning(
                    "hermes-compress: safety guard reverted %d empty tool output(s)",
                    _empty_guard_count,
                )

            if flags.optimize_content:
                from hermes_compress._optimize import optimize_content
                for i, msg in enumerate(messages):
                    if msg.get("role") != "tool":
                        continue
                    content = msg.get("content", "")
                    if not isinstance(content, str) or len(content) < 100:
                        continue
                    optimized = optimize_content(
                        content, msg.get("name", ""),
                        compact_numbers=flags.round_json_numbers,
                        normalize_paths_enabled=flags.normalize_paths,
                        shorten_ts=flags.shorten_timestamps,
                    )
                    if len(optimized) < len(content):
                        messages[i] = {**msg, "content": optimized}

            pp_ms = (time.monotonic() - pp_start) * 1000
            pp_saved = sum(len(str(m)) for m in result.messages) - sum(len(str(m)) for m in messages)
            if stats:
                stats.preprocess_ms = pp_ms
                stats.preprocess_saved = pp_saved

            if dev and flags.simulate_backpressure:
                from hermes_compress._dev import simulate_backpressure
                simulate_backpressure(flags.backpressure_delay_ms)

            total_ms = (time.monotonic() - total_start) * 1000

            self.stats["calls"] += 1
            self.stats["total_tokens_saved"] += result.tokens_saved
            self.stats["total_duration_ms"] += total_ms
            self._track_tool_stats(messages, result)

            if stats:
                stats.tokens_before = result.tokens_before
                stats.tokens_after = result.tokens_after
                stats.tokens_saved = result.tokens_saved
                stats.compress_ms = compress_ms
                stats.duration_ms = total_ms
                stats.transforms = list(result.transforms_applied) if result.transforms_applied else []
                stats.tool_types = {
                    m.get("name", "unknown"): stats.tool_types.get(m.get("name", "unknown"), 0) + 1
                    for m in messages if m.get("role") == "tool"
                }
                get_collector().record(stats)

            if dev and flags.dry_run:
                return CompressResult(
                    messages=messages,
                    tokens_before=result.tokens_before,
                    tokens_after=result.tokens_after,
                    tokens_saved=result.tokens_saved,
                    compression_ratio=result.compression_ratio,
                    duration_ms=total_ms,
                    transforms_applied=list(result.transforms_applied) if result.transforms_applied else [],
                )

            if result.tokens_saved > 0:
                extra = ""
                if pp_saved > 0:
                    extra += f" [+{pp_saved} post-process]"
                logger.info(
                    "hermes-compress: saved %d tokens (%.1f%%) "
                    "from %d messages in %.0fms%s [%s]",
                    result.tokens_saved,
                    result.compression_ratio * 100,
                    len(messages),
                    total_ms,
                    extra,
                    ", ".join(result.transforms_applied) if result.transforms_applied else "none",
                )

            return CompressResult(
                messages=messages,
                tokens_before=result.tokens_before,
                tokens_after=result.tokens_after,
                tokens_saved=result.tokens_saved,
                compression_ratio=result.compression_ratio,
                duration_ms=total_ms,
                transforms_applied=list(result.transforms_applied) if result.transforms_applied else [],
            )

        except ImportError:
            self._headroom_available = False
            return CompressResult(
                messages=messages,
                error="headroom import failed",
            )
        except Exception as exc:
            logger.warning("hermes-compress failed: %s", exc)
            return CompressResult(
                messages=messages,
                error=str(exc),
            )

    def _compress_proxy(
        self, messages: list[dict[str, Any]]
    ) -> CompressResult:
        """Compress via headroom proxy server.

        Sends messages to the proxy HTTP endpoint and returns the
        compressed result. Requires a running proxy (see Proxy class).
        """
        import json
        import urllib.request

        if self._proxy is None or not self._proxy.healthy:
            return CompressResult(
                messages=messages,
                error="proxy not running - start with Proxy(port=8787).start()",
            )

        start = time.monotonic()
        try:
            url = f"{self._proxy.base_url}/v1/compress"
            data = json.dumps({
                "messages": messages,
                "model": self.model or "gpt-4o",
            }).encode("utf-8")

            req = urllib.request.Request(
                url,
                data=data,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=60) as resp:
                result_data = json.loads(resp.read().decode())

            duration_ms = (time.monotonic() - start) * 1000

            self.stats["calls"] += 1
            tokens_saved = result_data.get("tokens_saved", 0)
            self.stats["total_tokens_saved"] += tokens_saved
            self.stats["total_duration_ms"] += duration_ms

            return CompressResult(
                messages=result_data.get("messages", messages),
                tokens_before=result_data.get("tokens_before", 0),
                tokens_after=result_data.get("tokens_after", 0),
                tokens_saved=tokens_saved,
                compression_ratio=result_data.get("compression_ratio", 0.0),
                duration_ms=duration_ms,
            )

        except Exception as exc:
            logger.warning("hermes-compress [proxy] failed: %s", exc)
            return CompressResult(
                messages=messages,
                error=str(exc),
            )

    # ── Helpers ──────────────────────────────────────────────────────────

    def _track_tool_stats(
        self, messages: list[dict[str, Any]], result: Any
    ) -> None:
        for msg in messages:
            tool_name = msg.get("name", "") if msg.get("role") == "tool" else ""
            if not tool_name:
                continue
            hint = TOOL_CONTENT_HINTS.get(tool_name, "unknown")
            entry = self.stats["by_tool"].setdefault(hint, {
                "count": 0, "tokens_saved": 0,
            })
            entry["count"] += 1
            if result.tokens_saved > 0 and len(messages) > 1:
                entry["tokens_saved"] += result.tokens_saved // len(messages)

    @staticmethod
    def _probe_headroom() -> bool:
        try:
            import headroom  # noqa: F401
            return True
        except ImportError:
            return False

    @staticmethod
    def get_tool_hint(tool_name: str) -> str:
        return TOOL_CONTENT_HINTS.get(tool_name, "unknown")

    @staticmethod
    def get_tool_min_size(tool_name: str) -> int:
        return TOOL_MIN_SIZES.get(
            tool_name, TOOL_MIN_SIZES["__default__"]
        )
