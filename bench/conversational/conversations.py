"""Compatibility shim - the conversation corpus now lives in the fixtures/ package.

The 2141-line flat module was split into fixtures/{types,coding_task,
exploration_task,debugging_task,release_flow_task,compression_aware_task}.py.
This shim preserves the flat-module public names for existing consumers
(e.g. Maintain/scripts/bench/benchmark-eval.py: `import conversations`).
"""

from fixtures import (
    ALL_CONVERSATIONS,
    CODING_TASK,
    COMPRESSION_AWARE_TASK,
    Conversation,
    DEBUGGING_TASK,
    EXPLORATION_TASK,
    RELEASE_FLOW_TASK,
    ToolCall,
    Turn,
)