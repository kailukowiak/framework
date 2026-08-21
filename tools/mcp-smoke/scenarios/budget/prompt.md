You are smoke-testing FrameWork, a reactive spreadsheet successor, through its MCP tools (server "framework"). Your job is to build a small budget-vs-actual tracker and report honestly how easy or hard it was. The document you are connected to starts as a demo — ignore or delete its demo objects as you prefer.

The app to build:

1. Import `actuals.csv` from this directory (Category, Month, Amount — five months of spending across six categories).
2. A "Budget" frame you create directly, with two columns: Category (matching the six categories in the actuals) and "Monthly budget", with these values: Travel 2000, Software 1500, Hardware 3000, Training 1200, Marketing 2500, Facilities 3500.
3. A totals frame: actual spending summed per category across all months.
4. Join the budget onto the totals by category. HARD REQUIREMENT: the join must be a validated one — the tools will tell you what the budget frame needs first.
5. A "Used" column on the joined frame: total actual spending divided by the five-month budget (Monthly budget × 5), displayed as a percentage.
6. Prove the app is live: double one category's monthly budget by editing the budget cell, and confirm that category's "Used" percentage halves. Report the before/after values.

Work through the MCP tools only. When something fails, read the error and adapt — the errors are written to guide you. When you cannot find a capability, use the discovery tools before concluding it doesn't exist.

End your run with a report:
- FRICTION LOG: each place you got stuck, what you tried, what worked.
- SEARCHES: what you searched for and whether the results led you to the answer.
- VERIFICATION: the before/after "Used" values from step 6.
- VERDICT: could an Excel-savvy non-programmer have done this through a UI offering these same operations? What single change would most improve the experience?
