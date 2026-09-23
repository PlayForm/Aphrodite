"""Task-prompt extraction for live benchmark runs."""


def task_prompt_for(conversation) -> str:
    """Extract the runnable task prompt from a scripted conversation fixture.

    Uses the first user turn (the actual task instruction) + the description
    as context so the live agent gets the same task the simulation scripts.
    """
    first_user = next((t.content for t in conversation.turns if t.role == "user"), "")
    return f"[bench task: {conversation.description}]\n\n{first_user}"