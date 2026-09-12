"""Build live investment returns through the public MCP surface."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"])
    try:
        client.call("search_functions", {"query": "XIRR"})
        client.call("create_frame", {"name": "Flows", "grid": [
            ["Date", "Amount"], ["2025-01-01", "-100"], ["2027-01-01", "121"],
        ]})
        client.call("create_block", {"name": "Returns"})
        client.call("set_block_source", {
            "block": "Returns",
            "source": "periodic = `Flows`.`Amount`.finance.irr()\nannual = `Flows`.`Amount`.finance.xirr(`Flows`.`Date`)",
        })
    finally:
        client.close()


if __name__ == "__main__":
    main()
