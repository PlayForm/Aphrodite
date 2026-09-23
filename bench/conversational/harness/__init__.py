"""Aphrodite Conversational Benchmark Harness - atomized package.

Consumers import granularly from the submodules; this __init__ re-exports
every public name so `from harness import ...` keeps working.
"""

from .binary import APHRODITE_BINARY, resolve_aphrodite_binary
from .paths import RESULTS_DIR
from .provider import API_KEY, BASE_URL, MODEL, _resolve_hermes_provider
from .proxy_client import ProxyClient
from .proxy_manager import ProxyManager
from .provider_client import ProviderClient
from .results import ConversationResult, RunManifest, TurnResult
from .run import run_benchmark
from .runner import ConversationRunner
from .scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT, CACHE_PORT, SCENARIO_METADATA, Scenario, TOKEN_PORT
from .tokens import count_message_tokens, estimate_tokens
