"""Builds the ledger scenario's reference solution through the MCP tools —
no model involved. Selfcheck runs this, then the scenario's verifier: a
pass proves the task is achievable through the surface AND the verifier
recognizes a correct answer, all for zero tokens.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient

# As written on the printout: a dollar sign, parentheses for credits, and
# the three cents a float cannot hold. They foot to exactly zero.
LINES = [
    ["1", "Invoice 4471", "$1,234.56"],
    ["2", "Payment on account", "(1,000.00)"],
    ["3", "Settlement", "(234.56)"],
    ["4", "Postage", "0.10"],
    ["5", "Stationery", "0.20"],
    ["6", "Petty cash in", "(0.30)"],
]


def main():
    client = McpClient(
        os.environ["FRAMEWORK_MCP_BINARY"], os.environ["FRAMEWORK_DOCUMENT"]
    )
    try:
        client.call(
            "create_frame",
            {"name": "Journal", "grid": [["Line", "Memo", "Amount"]] + LINES},
        )
        client.call(
            "set_column_type",
            {
                "frame": "Journal",
                "column": "Amount",
                "dataType": "accounting",
                "scale": 2,
            },
        )
        # A running balance needs the row order declared, and the journal's
        # order is its line numbering.
        client.call(
            "sort_frame",
            {
                "frame": "Journal",
                "keys": [{"column": "Line", "descending": False}],
            },
        )
        client.call(
            "add_calculated_column",
            {
                "frame": "Journal",
                "name": "Balance",
                "formula": "`Amount`.cum_sum(false)",
            },
        )
        client.call(
            "add_summary",
            {"frame": "Journal", "column": "Amount", "operation": "sum"},
        )
    finally:
        client.close()


if __name__ == "__main__":
    main()
