#!/usr/bin/env python3
"""Mock LLM upstream for wire-compat bench. Stdlib only.

Listens on 127.0.0.1:19899. Records every request (path, headers, body)
into an in-process log retrievable via GET /__bench/last. Returns canned
completions per dialect, keyed off the request path + body:

  POST /v1/chat/completions          -> OpenAI chat completion
      body {"stream": true}          -> SSE stream (text/event-stream)
      body marker "want_tool_calls"  -> completion with big tool_calls args
      body marker "want_big_content" -> completion with ~30KB content
  POST /v1/messages                  -> Anthropic Messages response
  POST /v1beta/models/...:generateContent -> Gemini response
  anything else                      -> 200 JSON echo

Run: python3 mock_upstream.py [port]
"""

import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 19899

_lock = threading.Lock()
_last = {}  # last request seen, keyed by a client-supplied bench id

BIG_CONTENT = "fn compute(x: u64) -> u64 { x.wrapping_mul(2654435761) }\n" * 520  # ~30KB
BIG_TOOL_ARGS = json.dumps(
    {
        "file_path": "/tmp/very/long/path/to/file.rs",
        "content": "let mut total = 0u64;\n" * 900,  # ~20KB inside args
    }
)


def openai_completion(content, tool_calls=None):
    msg = {"role": "assistant", "content": content}
    if tool_calls is not None:
        msg["tool_calls"] = tool_calls
        msg["content"] = None if content == "" else content
    return {
        "id": "chatcmpl-bench-1",
        "object": "chat.completion",
        "created": 1760000000,
        "model": "bench-model",
        "choices": [
            {"index": 0, "message": msg, "finish_reason": "tool_calls" if tool_calls else "stop"}
        ],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
    }


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):  # quiet
        pass

    def _read_body(self):
        n = int(self.headers.get("Content-Length", 0) or 0)
        return self.rfile.read(n) if n else b""

    def _record(self, body):
        bench_id = self.headers.get("X-Bench-Id", "")
        if not bench_id:
            try:
                bench_id = json.loads(body).get("bench_id", "anon")
            except Exception:
                bench_id = "anon"
        with _lock:
            _last[bench_id] = {
                "path": self.path,
                "headers": {k.lower(): v for k, v in self.headers.items()},
                "body_b64_len": len(body),
                "body": body.decode("utf-8", "replace"),
            }
        return bench_id

    def _send_json(self, obj, status=200):
        data = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("X-Mock-Upstream", "bench")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path.startswith("/__bench/last"):
            key = self.path.split("?", 1)[1] if "?" in self.path else "anon"
            with _lock:
                self._send_json(_last.get(key, {}))
            return
        self._send_json({"object": "list", "data": [{"id": "bench-model"}]})

    def do_POST(self):
        body = self._read_body()
        self._record(body)
        text = body.decode("utf-8", "replace")

        if self.path.endswith("/chat/completions"):
            try:
                req = json.loads(text)
            except Exception:
                req = {}
            if req.get("stream"):
                self._send_sse()
                return
            if "want_tool_calls" in text:
                tc = [
                    {
                        "id": "call_bench_1",
                        "type": "function",
                        "function": {"name": "write_file", "arguments": BIG_TOOL_ARGS},
                    }
                ]
                self._send_json(openai_completion("", tool_calls=tc))
                return
            if "want_big_content" in text:
                self._send_json(openai_completion(BIG_CONTENT))
                return
            self._send_json(openai_completion("ok: short canned answer."))
            return

        if self.path.endswith("/v1/messages") or self.path.endswith("/messages"):
            self._send_json(
                {
                    "id": "msg_bench_1",
                    "type": "message",
                    "role": "assistant",
                    "model": "bench-model",
                    "content": [{"type": "text", "text": "anthropic canned answer."}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 10, "output_tokens": 5},
                }
            )
            return

        if ":generateContent" in self.path:
            self._send_json(
                {
                    "candidates": [
                        {
                            "content": {
                                "parts": [{"text": "gemini canned answer."}],
                                "role": "model",
                            },
                            "finishReason": "STOP",
                        }
                    ],
                    "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5},
                }
            )
            return

        self._send_json({"echo": True, "path": self.path})

    def _send_sse(self):
        chunks = []
        for i, piece in enumerate(["Hello", " from", " SSE", " stream."]):
            chunks.append(
                "data: "
                + json.dumps(
                    {
                        "id": "chatcmpl-bench-1",
                        "object": "chat.completion.chunk",
                        "created": 1760000000,
                        "model": "bench-model",
                        "choices": [
                            {"index": 0, "delta": {"content": piece}, "finish_reason": None}
                        ],
                    }
                )
                + "\n\n"
            )
        chunks.append(
            "data: "
            + json.dumps(
                {
                    "id": "chatcmpl-bench-1",
                    "object": "chat.completion.chunk",
                    "created": 1760000000,
                    "model": "bench-model",
                    "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                }
            )
            + "\n\n"
        )
        chunks.append("data: [DONE]\n\n")
        payload = "".join(chunks).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("X-Mock-Upstream", "bench")
        self.end_headers()
        self.wfile.write(payload)


if __name__ == "__main__":
    srv = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print(f"mock upstream on 127.0.0.1:{PORT}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass
