"""Live conversational benchmark for Aphrodite - atomized package.

Consumers import granularly from the submodules; this __init__ re-exports the
public names of the former flat live_runner.py.
"""

from .agent import make_agent
from .cli import main
from .scenario import run_scenario_conversation
from .stats import extract_usage, proxy_manager_ccr_stats
from .task_prompt import task_prompt_for
