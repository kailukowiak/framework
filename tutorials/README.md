# Learn FrameWork

These tutorials use one sales story to teach FrameWork from a blank table to a
multi-table model. Each practical lesson includes a starting workbook, a
finished answer key, expected values, and screenshots captured from the current
Tauri application.

The lessons are also manual smoke tests. If a named control is missing, a step
cannot be completed, or the observed result differs from the checkpoint, record
it in the lesson's smoke-test notes rather than working around it silently.

## How the practical lessons work

Each practical lesson follows the same rhythm:

1. Open the finished answer key and spend a minute identifying the named tables,
   blocks, tabs, and plots.
2. Open the starting workbook and make every change there. Do not edit the
   answer key.
3. Complete one numbered section at a time. Text in **bold** names a control as
   it appears in FrameWork; fenced text is content to enter exactly.
4. Stop at each **Checkpoint** and compare the visible result before continuing.
   A later step may depend on the table shape, order, or name established there.
5. If a checkpoint differs, use undo once, reread the current section, and
   compare with the answer key. Reset the tutorials only when you want a clean
   copy of every starting and finished workbook.

The lessons intentionally explain both the gesture and its consequence. The
gesture helps you complete the task; the consequence—such as a Sort appearing
in Wrangle—helps you understand what will stay live downstream.

## Learning path

1. **[Your first FrameWork workbook](first-workbook/README.md)** — paste a
   typed table, add one calculated column, format it, declare a sort, branch a
   filtered tab, write block calculations, and make a plot. About 15 minutes.
2. **[Importing an Excel workbook](excel-import/README.md)** — import one
   clean Excel Table, then choose three explicit tables from a two-sheet
   workbook. About 10 minutes.
3. **[Month-over-month formulas by pointing](formula-clicks/README.md)** — use
   click-to-build, `.shift(1)`, table-first autocomplete, and live checks.
   About 15 minutes.
4. **[Excel concepts in FrameWork](excel-to-framework/README.md)** — a short
   translation guide for formulas, sheets, lookups, PivotTables, and Power
   Query. Read this before the advanced project if Excel is your starting
   point.
5. **[Month-end close](month-end-close/README.md)** — join actuals to budget,
   build a variance table, summarize, pivot, isolate exceptions, and keep
   control totals live. About 30–45 minutes.

## How to use the answer keys

Open the finished workbook first and look at the result for a minute. Then open
the starting workbook and rebuild it from the guide. Keep the finished file
unchanged so it remains a known-good comparison.

Every generated workbook is built through `Store::apply(Operation::...)`, the
same validation and persistence boundary used by the desktop and MCP. Rebuild
all practical tutorials with:

```bash
cargo run -p framework-core --example generate_tutorial_workbooks
cargo run -p framework-core --example generate_excel_import_tutorial
cargo run -p framework-core --example generate_formula_click_tutorial
```

## In the desktop app

Open **Data Library** and choose **Create tutorials**. FrameWork creates eight
editable workbooks in `Documents/FrameWork Tutorials`: a starting workbook and
an answer key for each practical lesson. Open a starting workbook, work through
the guide, and use its answer key only to compare the result.

If you are working directly from this repository instead, open the `.fw` files
linked under **Files** in each lesson with **File → Open**.

**Reset tutorials** is deliberately explicit: it replaces those eight known
working copies, their histories, and the tiny Excel lesson sources with the
bundled canonical files. It does not remove notes, exports, or any other files
in the tutorial folder.

## Smoke-test note template

When something feels wrong, capture the point of friction, not only whether it
eventually worked:

```text
Lesson and step:
Expected:
Observed:
Could I discover the control without the guide?
Did undo restore the previous visible state?
Screenshot or error text:
```
