"""Result dataclasses (TurnResult, ConversationResult, RunManifest)."""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional

@dataclass
class TurnResult:
    """Captured result for a single conversation turn."""

    turn_index: int
    role: str
    request: Optional[dict] = None  # The API request sent
    response: Optional[dict] = None  # The API response received
    response_status: Optional[int] = None
    elapsed_ms: float = 0.0
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0
    error: Optional[str] = None
    # CCR events from this turn (if using proxy)
    ccr_events: list[dict] = field(default_factory=list)


@dataclass
class ConversationResult:
    """Aggregate result for one conversation under one scenario."""

    scenario: str
    conversation_name: str
    turns: list[TurnResult] = field(default_factory=list)
    proxy_stats_snapshots: list[dict] = field(default_factory=list)
    total_prompt_tokens: int = 0
    total_completion_tokens: int = 0
    total_tokens: int = 0
    total_elapsed_ms: float = 0.0
    errors: list[str] = field(default_factory=list)


@dataclass
class RunManifest:
    """Top-level manifest for a full benchmark run."""

    run_id: str
    timestamp: str
    aphrodite_version: str
    model: str
    scenarios_run: list[str] = field(default_factory=list)
    conversations_run: list[str] = field(default_factory=list)
    total_turns: int = 0
    total_errors: int = 0
    results: list[ConversationResult] = field(default_factory=list)
