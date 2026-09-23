"""Task-prompt extraction for live benchmark runs.

Maps each fixture to its executable workspace under
bench/conversational/workspaces/ so the live agent works on REAL files
instead of searching the machine. The fixtures were written for the scripted
simulation (which had no on-disk projects); the live runner must point the
agent at an actual workspace or it burns its turn budget searching.
"""

from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent.parent
WORKSPACES_DIR = BENCH_DIR / "workspaces"

# Fixture name -> workspace subdir under bench/conversational/workspaces/.
# Tasks without a workspace yet fall back to the bare prompt (flagged).
_WORKSPACE_BY_TASK = {
    "coding_task": "coding_task",
    "exploration_task": "exploration_task",  # TODO: stage an HTTP-proxy codebase
    "debugging_task": "debugging_task",  # TODO: stage a broken Rust project
    "release_flow_task": "release_flow_task",  # TODO: stage a release mirror
    "compression_aware_task": "compression_aware_task",  # TODO: point at aphrodite src
}


def workspace_for(conversation) -> Path | None:
    """Return the workspace dir for a conversation fixture, or None."""
    sub = _WORKSPACE_BY_TASK.get(conversation.name)
    if not sub:
        return None
    ws = WORKSPACES_DIR / sub
    return ws if ws.is_dir() else None


def task_prompt_for(conversation, workspace: Path | None = None) -> str:
    """Extract the runnable task prompt from a scripted conversation fixture.

    Uses the first user turn (the actual task instruction) + the description
    as context so the live agent gets the same task the simulation scripts.
    When a workspace exists, the prompt pins the agent to it (absolute path)
    so it reads/edits real files instead of searching the machine.
    """
    first_user = next((t.content for t in conversation.turns if t.role == "user"), "")
    prompt = f"[bench task: {conversation.description}]\n\n{first_user}"
    if workspace is not None:
        prompt = (
            f"[bench task: {conversation.description}]\n"
            f"[workspace: {workspace} - work ONLY inside this directory; "
            f"it contains the project files for the task. Do not search the "
            f"whole machine - the project is here.]\n\n{first_user}"
        )
    return prompt
