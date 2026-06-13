"""
CLI entry point for hermes-compress.

Usage:
    hermes-compress install              # Patch hermes-agent for headroom
    hermes-compress uninstall            # Revert patches
    hermes-compress status               # Check if installed
    hermes-compress proxy [--port 8787]  # Start headroom proxy
    hermes-compress compress <text>      # Compress text/JSON
    hermes-compress --version
"""

from __future__ import annotations

import argparse
import json
import sys


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="hermes-compress",
        description="Headroom-powered context compression - install, proxy, compress.",
    )
    parser.add_argument(
        "--version", action="version",
        version=f"hermes-compress {__import__('hermes_compress').__version__}",
    )

    sub = parser.add_subparsers(dest="command")

    # ── install ───────────────────────────────────────────────────────
    install = sub.add_parser("install", help="Patch hermes-agent for headroom compression")
    install.add_argument("--agent-dir", help="Path to hermes-agent (default: ~/.hermes/hermes-agent)")

    # ── uninstall ─────────────────────────────────────────────────────
    uninstall = sub.add_parser("uninstall", help="Remove patches from hermes-agent")
    uninstall.add_argument("--agent-dir", help="Path to hermes-agent")

    # ── status ────────────────────────────────────────────────────────
    status = sub.add_parser("status", help="Check hermes-compress installation status")
    status.add_argument("--agent-dir", help="Path to hermes-agent")

    # ── proxy ─────────────────────────────────────────────────────────
    proxy = sub.add_parser("proxy", help="Start headroom proxy server")
    proxy.add_argument("--port", type=int, default=8787)
    proxy.add_argument("--host", default="127.0.0.1")
    proxy.add_argument("--mode", default="token", choices=["token", "cache"])

    # ── compress ──────────────────────────────────────────────────────
    compress = sub.add_parser("compress", help="Compress text/JSON")
    compress.add_argument("text", nargs="?", help="Text to compress (or stdin)")
    compress.add_argument("--model", default="gpt-4o")
    compress.add_argument("--json", action="store_true", help="Output as JSON")

    args = parser.parse_args()

    if args.command == "install":
        _cmd_install(args)
    elif args.command == "uninstall":
        _cmd_uninstall(args)
    elif args.command == "status":
        _cmd_status(args)
    elif args.command == "proxy":
        _cmd_proxy(args)
    elif args.command == "compress":
        _cmd_compress(args)
    else:
        parser.print_help()


def _cmd_install(args) -> None:
    from pathlib import Path
    agent_dir = Path(args.agent_dir) if args.agent_dir else None

    from hermes_compress._install import install
    result = install(agent_dir)

    if result.patched:
        print(f"✓ Patched {len(result.patched)} file(s):")
        for p in result.patched:
            print(f"  • {p}")
    if result.skipped:
        print(f"⊙ Skipped {len(result.skipped)} file(s) (already patched)")
    if result.errors:
        print(f"✗ Errors:")
        for e in result.errors:
            print(f"  • {e}")

    if result.success:
        print(f"\n✓ hermes-compress installed to {result.agent_dir}")
        print("  Enable in config: compression.headroom.enabled: true")
        print("  Then restart Hermes.")
    else:
        sys.exit(1)


def _cmd_uninstall(args) -> None:
    from pathlib import Path
    agent_dir = Path(args.agent_dir) if args.agent_dir else None

    from hermes_compress._install import uninstall
    result = uninstall(agent_dir)

    if result.patched:
        print(f"✓ Reverted {len(result.patched)} file(s):")
        for p in result.patched:
            print(f"  • {p}")
    if result.errors:
        print(f"✗ Errors: {result.errors}")
        sys.exit(1)
    print("✓ hermes-compress uninstalled")


def _cmd_status(args) -> None:
    from pathlib import Path
    agent_dir = Path(args.agent_dir) if args.agent_dir else None

    from hermes_compress._install import status
    info = status(agent_dir)
    print(json.dumps(info, indent=2))


def _cmd_proxy(args) -> None:
    from hermes_compress import Proxy

    proxy = Proxy(port=args.port, host=args.host, mode=args.mode)
    print(f"Starting headroom proxy on http://{args.host}:{args.port} (mode={args.mode})")
    print("Set your LLM provider base_url to this address.")
    print("Press Ctrl+C to stop.")

    ok = proxy.start()
    if not ok:
        print("Failed to start proxy. Is headroom installed?", file=sys.stderr)
        sys.exit(1)

    try:
        import signal
        signal.pause()
    except KeyboardInterrupt:
        print("\nStopping...")
        proxy.stop()


def _cmd_compress(args) -> None:
    text = args.text
    if text is None:
        text = sys.stdin.read()

    from headroom import compress as hr_compress
    result = hr_compress(
        [{"role": "user", "content": text}],
        model=args.model,
    )

    if args.json:
        print(json.dumps({
            "tokens_before": result.tokens_before,
            "tokens_after": result.tokens_after,
            "tokens_saved": result.tokens_saved,
            "compression_ratio": round(result.compression_ratio * 100, 1),
        }, indent=2))
    else:
        print(f"Tokens: {result.tokens_before:,} → {result.tokens_after:,}")
        print(f"Saved:  {result.tokens_saved:,} ({result.compression_ratio*100:.1f}%)")
        if result.messages:
            print(f"\nCompressed:\n{result.messages[0]['content'][:500]}")
