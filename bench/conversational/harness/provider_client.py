"""Direct provider API client (baseline scenario, no proxy)."""

from __future__ import annotations
import requests
from .provider import API_KEY, BASE_URL, MODEL


class ProviderClient:
    """Direct provider API client for the baseline scenario.

    Uses the Hermes-resolved provider (BASE_URL / API_KEY / MODEL), the same
    upstream a normal Hermes session talks to.
    """

    def __init__(self, api_key: str = API_KEY, model: str = MODEL, base_url: str = BASE_URL):
        self.api_key = api_key
        self.model = model
        self.base_url = base_url

    def chat_completion(
        self,
        messages: list[dict],
        stream: bool = False,
        tools: Optional[list[dict]] = None,
    ) -> dict:
        """Send a chat completion request and return the full response."""
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        body = {
            "model": self.model,
            "messages": messages,
            "stream": stream,
        }
        if tools:
            body["tools"] = tools

        resp = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers=headers,
            json=body,
            timeout=120,
        )
        return {
            "status_code": resp.status_code,
            "body": resp.json()
            if resp.headers.get("content-type", "").startswith("application/json")
            else {"raw": resp.text},
        }
