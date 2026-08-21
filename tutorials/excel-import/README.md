# Importing an Excel workbook

This practical lesson ships with two ordinary Excel files and a start/answer
pair of FrameWork workbooks. It teaches the deliberate interchange boundary:
pick one Excel Table, suggested rectangular region, or explicit range; preview
its cached values; and import it
as a static FrameWork table without attempting to copy Excel formulas or
formatting.

## Files

- `excel-import-start.fw` — markdown instructions and an empty canvas.
- `excel-import-finished.fw` — the same instructions plus six completed,
  artifact-backed imports.
- `source/simple-customers.xlsx` — one clean named Excel Table.
- `source/multi-table-operations.xlsx` — two worksheets, three named Excel
  Tables, two loose pasted ranges, and an isolated note that should be ignored.

Regenerate the workbooks after changing the lesson or import behavior with:

```sh
cargo run -p framework-core --example generate_excel_import_tutorial
```

The desktop copies both Excel source files beside each editable tutorial
workbook, so the lesson works in an installed app without this repository.
