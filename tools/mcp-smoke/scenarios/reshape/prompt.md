You are smoke-testing FrameWork, a reactive spreadsheet successor, through its MCP tools (server "framework"). Your job is to reshape a wide spreadsheet into a proper long table and report honestly how easy or hard it was. The document you are connected to starts as a demo — ignore or delete its demo objects as you prefer.

The task: `sales_wide.csv` in this directory holds sales in the classic wide layout — one Product column, then a column per month (Jan through Jun). This shape is why spreadsheets rot; fix it.

1. Bring the data in as a frame you can edit: create the frame from the file's contents directly (typed/pasted, not a live file link — you will be editing cells in step 4).
2. A long frame derived from it: one row per (Product, Month, Sales) — 48 rows. HARD REQUIREMENT: derive it with a reshaping transformation, not by hand-writing rows.
3. Show the long frame wide again as a *view* (months across, products down) if the tools offer a way — the long data with the wide reading.
4. Prove it is live: change one product's Jan value in the source frame, and confirm the long frame's matching row shows the new number. Report the before/after.

Work through the MCP tools only. When something fails, read the error and adapt. When you cannot find a capability, use the discovery tools before concluding it doesn't exist.

End your run with a report:
- FRICTION LOG: each place you got stuck, what you tried, what worked.
- SEARCHES: what you searched for and whether the results led you to the answer.
- VERIFICATION: the before/after values from step 4.
- VERDICT: could an Excel-savvy non-programmer have done this through a UI offering these same operations? What single change would most improve the experience?
