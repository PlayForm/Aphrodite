"""Proxy lifecycle management for benchmark scenarios."""

from __future__ import annotations
import os
import subprocess
import time
import requests
from .provider import API_KEY, BASE_URL, MODEL
from .scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT, SCENARIO_METADATA


class ProxyManager:
    """Manages aphrodite proxy processes for benchmark scenarios."""

    def __init__(self, bin_path: str, work_dir: Path):
        self.bin_path = bin_path
        self.work_dir = work_dir
        self.processes: dict[str, subprocess.Popen] = {}

    def start_proxy(self, mode: str, port: int) -> subprocess.Popen:
        """Start a single aphrodite proxy process."""
        listen = f"127.0.0.1:{port}"
        db_path = self.work_dir / f"ccr_{mode}_{port}.db"

        # Remove stale DB from previous runs
        if db_path.exists():
            db_path.unlink()

        env = os.environ.copy()
        env["APHRODITE_CONFIG_PATH"] = "/nonexistent/aphrodite-bench.toml"
        env["APHRODITE_API_KEY"] = API_KEY
        env["APHRODITE_API_URL"] = BASE_URL
        env["APHRODITE_MODEL"] = MODEL

        proc = subprocess.Popen(
            [
                self.bin_path,
                "--mode",
                mode,
                "--listen",
                listen,
                "--api-url",
                BASE_URL,
                "--api-key",
                API_KEY,
                "--ccr-db-path",
                str(db_path),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
        self.processes[f"{mode}:{port}"] = proc

        # Wait for it to be ready
        deadline = time.time() + 10
        while time.time() < deadline:
            try:
                sock = __import__("socket").create_connection(("127.0.0.1", port), timeout=0.5)
                sock.close()
                print(f"  [proxy] {mode} proxy ready on :{port}")
                return proc
            except (OSError, ConnectionRefusedError):
                time.sleep(0.1)

        raise RuntimeError(f"Proxy {mode}:{port} failed to start within 10s")

    def start_for_scenario(self, scenario: Scenario) -> dict[str, int]:
        """Start proxies needed for a scenario. Returns {mode: port}."""
        meta = SCENARIO_METADATA[scenario]
        ports = {}

        if meta["uses_cache_proxy"]:
            self.start_proxy("cache", BENCH_CACHE_PORT)
            ports["cache"] = BENCH_CACHE_PORT
            print(f"  [scenario] cache proxy started on :{BENCH_CACHE_PORT}")

        if meta["uses_token_proxy"]:
            self.start_proxy("token", BENCH_TOKEN_PORT)
            ports["token"] = BENCH_TOKEN_PORT
            print(f"  [scenario] token proxy started on :{BENCH_TOKEN_PORT}")

        return ports

    def stop_all(self):
        """Stop all managed proxy processes."""
        for name, proc in list(self.processes.items()):
            try:
                proc.terminate()
                proc.wait(timeout=5)
            except (subprocess.TimeoutExpired, ProcessLookupError):
                try:
                    proc.kill()
                except ProcessLookupError:
                    pass
            print(f"  [proxy] stopped {name}")
        self.processes.clear()

    def get_stats(self, port: int) -> dict:
        """Fetch proxy stats from GET /stats."""
        try:
            resp = requests.get(f"http://127.0.0.1:{port}/stats", timeout=2)
            return resp.json() if resp.ok else {}
        except Exception:
            return {}

    def get_ccr_catalog(self, port: int) -> list[dict]:
        """Fetch CCR catalog entries."""
        try:
            resp = requests.get(f"http://127.0.0.1:{port}/ccr/catalog", timeout=2)
            return resp.json() if resp.ok else []
        except Exception:
            return []
