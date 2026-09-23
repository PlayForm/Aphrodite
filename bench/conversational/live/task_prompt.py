"""Task-prompt extraction + workbench staging for live benchmark runs.

CONTAINMENT MODEL (2026-09-23): every cell runs inside a pre-staged
workbench at results/<run>/<scenario>/<task>/workbench/ - a copy of the
task's fixture workspace (or an empty dir when the fixture has no workspace
yet). The agent is CONFINED to that workbench: its cwd is the workbench and
the prompt forbids reading, writing, or searching anywhere else. This makes
benchmark runs safe and reproducible (no machine-wide scans, no writes
outside results/), per the operator's directive.

Task fixtures without a staged workspace still get a workbench (empty +
scaffold note) so the confinement rule holds uniformly - the agent must
either work with what's there or report that the fixture is incomplete,
never search the machine.
"""

import shutil
from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent.parent
WORKSPACES_DIR = BENCH_DIR / "workspaces"

# Fixture name -> workspace subdir under bench/conversational/workspaces/.
# Tasks without a workspace yet get an empty workbench (scaffold note).
_WORKSPACE_BY_TASK = {
    "coding_task": "coding_task",
    "exploration_task": "exploration_task",  # TODO: stage an HTTP-proxy codebase
    "debugging_task": "debugging_task",  # TODO: stage a broken Rust project
    "release_flow_task": "release_flow_task",  # TODO: stage a release mirror
    "compression_aware_task": "compression_aware_task",  # TODO: point at aphrodite src
}


def source_workspace(conversation) -> Path | None:
    """The fixture's pre-authored workspace dir (or None if unstaged)."""
    sub = _WORKSPACE_BY_TASK.get(conversation.name)
    if not sub:
        return None
    ws = WORKSPACES_DIR / sub
    return ws if ws.is_dir() else None


def stage_workbench(conversation, cell_dir: Path) -> Path:
    """Create/prepare this cell's workbench under cell_dir, return its path.

    The workbench is a fresh copy of the fixture workspace (or an empty dir)
    per cell, so every session starts from identical, isolated state and
    every write the agent makes stays inside results/.
    """
    workbench = cell_dir / "workbench"
    if workbench.exists():
        shutil.rmtree(workbench)
    workbench.mkdir(parents=True, exist_ok=True)
    src = source_workspace(conversation)
    if src is not None:
        # Copy workspace contents (excluding build artifacts) into the workbench
        for child in src.iterdir():
            if child.name in ("target", ".git", "__pycache__"):
                continue
            if child.is_dir():
                shutil.copytree(child, workbench / child.name)
            else:
                shutil.copy2(child, workbench / child.name)
    else:
        # Unstaged fixture: write a scaffold note so the agent knows the
        # workbench is deliberately empty (and does not search elsewhere).
        (workbench / "FIXTURE-NOTE.txt").write_text(
            "This workbench is intentionally empty: the task fixture references a "
            "project that has not been staged yet. Work ONLY inside this directory. "
            "If you cannot complete the task without missing files, report exactly "
            "what is missing - do not search or modify anything outside this workbench.\n"
        )
    return workbench


def task_prompt_for(conversation, workbench: Path) -> str:
    """Build the task prompt with a hard confinement contract.

    The agent is told its workbench path explicitly and forbidden from
    touching anything outside it - reads, writes, searches, and terminal
    commands all confined. This is the primary containment mechanism
    (cwd = workbench is the secondary one).
    """
    first_user = next((t.content for t in conversation.turns if t.role == "user"), "")
    return (
        f"[bench task: {conversation.description}]\n"
        f"[CONTAINMENT - mandatory]\n"
        f"Your entire working environment is the directory:\n"
        f"  {workbench}\n"
        f"You MUST work only inside that directory. Rules:\n"
        f"1. READ/WRITE/SEARCH/TERMINAL: only inside {workbench}. Never use\n"
        f"   absolute paths outside it, never `find`/`ls`/`cd` outside it.\n"
        f"2. If the files you need are not inside the workbench, say exactly what\n"
        f"   is missing and STOP - do not search the machine for them.\n"
        f"3. Do not modify anything under the repo, /Users, /tmp, /Volumes, or\n"
        f"   any path outside the workbench.\n"
        f"4. Your cwd is the workbench; use relative paths inside it.\n"
        f"\n[task]\n{first_user}"
    )