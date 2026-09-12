"""Build the loan comparison through the public MCP surface."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient


def main():
    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"])
    try:
        client.call("search_functions", {"query": "PMT"})
        client.call("create_frame", {"name": "Loans", "grid": [
            ["Principal", "Annual rate", "Months"],
            ["400000", "0.06", "60"], ["100000", "0", "10"],
        ]})
        for name, formula in [
            ("Payment", "(-`Principal`).finance.pmt(`Annual rate` / 12, `Months`)"),
            ("Interest", "(-`Principal`).finance.ipmt(`Annual rate` / 12, 1, `Months`)"),
            ("Principal paid", "(-`Principal`).finance.ppmt(`Annual rate` / 12, 1, `Months`)"),
        ]:
            client.call("add_calculated_column", {"frame": "Loans", "name": name, "formula": formula})
    finally:
        client.close()


if __name__ == "__main__":
    main()
