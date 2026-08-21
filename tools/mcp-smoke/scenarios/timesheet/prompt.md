You are smoke-testing FrameWork, a reactive spreadsheet successor, through its MCP tools (server "framework"). Your job is to rebuild a small timesheet app and report honestly how easy or hard it was. The document you are connected to starts as a demo — ignore or delete its demo objects as you prefer.

The app to build (mirrors a real Excel timesheet):

1. A single "Timesheet date" value, initially 2026-09-30. It is the app's only parameter.
2. Import entry_lines.csv from this directory (the timesheet's ~38 template rows: Line, Section, Dept code, Project, ...).
3. A frame holding every date of the timesheet period: the calendar month containing "Timesheet date", from the 1st through the date itself. HARD REQUIREMENT: do NOT write a CSV or literal rows for this — the dates must be derived so that editing "Timesheet date" regrows them automatically. If you don't know how, search the tools for what you'd type in Excel.
4. A "Sheet" frame: every entry line paired with every period date (the long timesheet skeleton).
5. An "Hours" column on the Sheet that a person can type into, where a typed value must SURVIVE the period changing and coming back — set whatever the tools require to make that possible.
6. Enter 8 hours against the first line on 2026-09-15, and 4 hours against the second line on 2026-09-03.
7. Prove the app works: set "Timesheet date" to 2026-10-31 and confirm the sheet shows October dates with no hours; set it back to 2026-09-30 and confirm both entries reappear exactly where they were entered.
8. If the tools offer a way to view the long sheet wide (lines as rows, dates as columns), turn it on.

Work through the MCP tools only. When something fails, read the error and adapt — the errors are written to guide you. When you cannot find a capability, use the discovery tools before concluding it doesn't exist.

End your run with a report:
- FRICTION LOG: each place you got stuck, what you tried, what worked.
- SEARCHES: what you searched for and whether the results led you to the answer.
- VERIFICATION: the row counts and hour values you observed at step 7, before/after each date change.
- VERDICT: could an Excel-savvy non-programmer have done this through a UI offering these same operations? What single change would most improve the experience?
