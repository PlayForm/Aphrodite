"""Client that talks to an aphrodite proxy (cache or token)."""

from __future__ import annotations
import requests
from .provider import API_KEY, MODEL


class ProxyClient:
    """Client that talks to an aphrodite proxy (cache or token)."""

    def __init__(self, port: int, api_key: str = API_KEY):
        self.base_url = f"http://127.0.0.1:{port}"
        self.api_key = api_key

    def chat_completion(
        self,
        messages: list[dict],
        stream: bool = False,
        tools: Optional[list[dict]] = None,
    ) -> dict:
        """Send a chat completion request through the proxy."""
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        body = {
            "model": MODEL,
            "messages": messages,
            "stream": stream,
        }
        if tools:
            body["tools"] = tools

        resp = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers=headers,
            json=body,
            timeout=300,
        )
        return {
            "status_code": resp.status_code,
            "body": resp.json()
            if resp.headers.get("content-type", "").startswith("application/json")
            else {"raw": resp.text},
        }

    def ccr_create(self, content: str, content_type: str = "text") -> dict:
        """Store content in CCR via the proxy."""
        resp = requests.post(
            f"{self.base_url}/ccr/create",
            json={"content": content, "type": content_type},
            timeout=30,
        )
        return resp.json() if resp.ok else {"error": resp.text}

    def retrieve(self, hash_val: str) -> dict:
        """Retrieve content from CCR."""
        resp = requests.post(
            f"{self.base_url}/retrieve",
            json={"hash": hash_val},
            timeout=10,
        )
        return resp.json() if resp.ok else {"error": resp.text}

    def get_stats(self) -> dict:
        """Get proxy health stats."""
        resp = requests.get(f"{self.base_url}/stats", timeout=5)
        return resp.json() if resp.ok else {}
