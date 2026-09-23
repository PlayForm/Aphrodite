"""Shared dataclasses for conversation fixtures (ToolCall, Turn, Conversation)."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class ToolCall:
    id: str
    name: str
    arguments: str  # JSON string


@dataclass
class Turn:
    role: str  # "system" | "user" | "assistant" | "tool"
    content: str
    tool_calls: Optional[list[ToolCall]] = None
    tool_call_id: Optional[str] = None  # for tool role responses


@dataclass
class Conversation:
    name: str
    description: str
    system_prompt: str
    turns: list[Turn]


# ═══════════════════════════════════════════════════════════════════════════════
# Script 1: Multi-file Refactoring Task
# Exercises: code reading (large file outputs), diff generation,
#            build output, error fixing
