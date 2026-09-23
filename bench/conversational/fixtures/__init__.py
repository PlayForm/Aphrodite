"""Conversation scripts for benchmarking Aphrodite compression across scenarios.

Each script is a list of turns. Each turn has:
  - role: "user" | "assistant" | "tool"
  - content: the message text
  - tool_calls (optional): tool calls the assistant would make
  - tool_results (optional): results of those tool calls (these are what
    the cache proxy compresses when large enough)

The scripts exercise different content types that trigger different
compression paths: code, diffs, build output, errors, search results,
terminal output, JSON, git output, and CCR marker/preview traffic.
Scripts 4 and 5 additionally exercise the Development release ritual
(git/build/tool-output content) and deliberate context minimization
(CCR markers, prefetch, previews over raw output).
"""

from .coding_task import CODING_TASK
from .compression_aware_task import COMPRESSION_AWARE_TASK
from .debugging_task import DEBUGGING_TASK
from .exploration_task import EXPLORATION_TASK
from .release_flow_task import RELEASE_FLOW_TASK
from .types import Conversation, ToolCall, Turn

# All scripts registry
ALL_CONVERSATIONS = [
    CODING_TASK,
    EXPLORATION_TASK,
    DEBUGGING_TASK,
    RELEASE_FLOW_TASK,
    COMPRESSION_AWARE_TASK,
]
