"""Ledger scenario verifier: an exact accounting column, amounts read the
way a bookkeeper writes them, a running balance that is itself exact, and a
control total of exactly zero.

Found by shape, never by name; the numbers are driven back through the same
MCP tools an agent uses. See ../timesheet/verify.py for the founding rules.

Exactness is the whole scenario, so every comparison here is `==` against
the cent. A float column would land on 0.6000000000000001 and 5.551e-17 and
sail past any tolerance test, which is precisely the bug this grades.
"""

import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mcp_client import McpClient

# The six lines, in the order they were handed over, with the cent each one
# is worth and the balance standing after it.
AMOUNTS = [1234.56, -1000.00, -234.56, 0.10, 0.20, -0.30]
BALANCES = [1234.56, 234.56, 0.00, 0.10, 0.30, 0.00]


def main():
    document_path = os.environ["FRAMEWORK_DOCUMENT"]
    with open(document_path) as file:
        document = json.load(file)["document"]
    frames = [f for f in document["objects"] if f["kind"] == "frame"]

    # The journal is the frame carrying an accounting column. Nothing in the
    # demo document has one, so the shape is the identification.
    posted = [
        frame
        for frame in frames
        if any(column.get("dataType") == "accounting" for column in frame["columns"])
    ]
    assert posted, "no frame carries an accounting column — the amounts are still floats"
    journal = posted[0]
    amount = next(
        column for column in journal["columns"] if column.get("dataType") == "accounting"
    )
    scale = amount.get("scale")
    assert scale in (None, 2), f"the amount column carries {scale} decimal places, not 2"
    assert len(journal.get("rows") or []) == 6, "the journal does not hold the six lines"
    print(f"ok: '{journal['name']}'.'{amount['name']}' is an exact amount at 2 places")

    assert journal.get("summaries"), "the frame carries no saved summary"
    control = [
        summary
        for summary in journal["summaries"]
        if summary["columnId"] == amount["id"] and summary["operation"] == "sum"
    ]
    assert control, "no saved sum of the amount column — there is no control total"
    print("ok: a control total is saved against the amount column")

    client = McpClient(os.environ["FRAMEWORK_MCP_BINARY"], document_path)
    try:
        snapshot = client.call("get_frame", {"frame": journal["name"], "limit": 100})

        # A running balance is a calculated column, and it has to be exact
        # too: a float balance would be the same bug one column over. The
        # declaration lives in the frame's chain rather than on the stored
        # column, so it is read back through the snapshot.
        balances = [
            column
            for column in snapshot["columns"]
            if column["id"] != amount["id"]
            and column.get("dataType") == "accounting"
            and column.get("formula")
        ]
        assert balances, (
            "no calculated accounting column — the running balance is missing "
            "or is a float"
        )
        balance = balances[0]
        print(f"ok: '{balance['name']}' is calculated and is itself exact")

        def column_values(column_id):
            return [
                cell["numericValue"]
                for row in snapshot["rows"]
                for cell in row["cells"]
                if cell["columnId"] == column_id
            ]

        posted_amounts = column_values(amount["id"])
        assert posted_amounts == AMOUNTS, (
            "the amounts were not read the way they were written "
            f"(a dollar sign and parentheses): {posted_amounts}"
        )
        print("ok: $1,234.56 and (1,000.00) read as the amounts they are")

        running = column_values(balance["id"])
        assert running == BALANCES, f"the running balance does not tie: {running}"
        print(f"ok: the running balance closes at {running[-1]:.2f}")

        total = next(
            summary
            for summary in snapshot["summaries"]
            if summary["columnId"] == amount["id"] and summary["operation"] == "sum"
        )
        assert total.get("error") is None, f"the control total errored: {total['error']}"
        assert total["numericValue"] == 0.0, (
            f"the journal does not foot: the control total is {total['numericValue']!r}, "
            "which is what a float column does to money"
        )
        print(f"ok: the control total is exactly zero ({total['display']})")
    finally:
        client.close()
    print("PASS")


if __name__ == "__main__":
    main()
