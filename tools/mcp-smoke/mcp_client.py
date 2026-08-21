"""A minimal MCP stdio client for driving framework-mcp from a script.

The verifier uses this to exercise the exact tool surface an agent uses —
same server binary, same JSON-RPC frames — so a scenario's assertions are
made through the door the product actually exposes, not a private back
channel that could pass while the real surface is broken.
"""

import json
import subprocess


class McpClient:
    def __init__(self, binary, document):
        self.process = subprocess.Popen(
            [binary, "--document", document],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        self.next_id = 0
        self._request(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "mcp-smoke-verify", "version": "0"},
            },
        )
        self._notify("notifications/initialized", {})

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=10)

    def call(self, tool, arguments=None):
        """Call one tool; returns the parsed structured result.

        Raises RuntimeError with the server's message on a tool error, so a
        scenario assertion failure reads as the product's own words.
        """
        result = self._request(
            "tools/call", {"name": tool, "arguments": arguments or {}}
        )
        if result.get("isError"):
            text = "; ".join(
                block.get("text", "") for block in result.get("content", [])
            )
            raise RuntimeError(f"{tool} failed: {text}")
        structured = result.get("structuredContent")
        if structured is not None:
            return structured
        for block in result.get("content", []):
            if block.get("type") == "text":
                try:
                    return json.loads(block["text"])
                except json.JSONDecodeError:
                    return block["text"]
        return None

    def _notify(self, method, params):
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def _request(self, method, params):
        self.next_id += 1
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
        }
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()
        while True:
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError(f"server exited during {method}")
            try:
                reply = json.loads(line)
            except json.JSONDecodeError:
                continue
            if reply.get("id") != self.next_id:
                continue
            if "error" in reply:
                raise RuntimeError(f"{method}: {reply['error'].get('message')}")
            return reply.get("result", {})
