# Importing an Excel workbook

This practical lesson ships with two ordinary Excel files and a start/answer
pair of FrameWork workbooks. It teaches the deliberate interchange boundary:
pick one Excel Table, suggested rectangular region, or explicit range; preview
its cached values; and import it
as a static FrameWork table without attempting to copy Excel formulas or
formatting.

This guide is rendered as the **Tutorial walkthrough** markdown card in both
the Start workbook and the Answer key.

## Files

- `excel-import-start.fw` — markdown instructions and an empty canvas.
- `excel-import-finished.fw` — the same instructions plus six completed,
  artifact-backed imports.
- `source/simple-customers.xlsx` — one clean named Excel Table.
- `source/multi-table-operations.xlsx` — two worksheets, three named Excel
  Tables, two loose pasted ranges, and an isolated note that should be ignored.

## 1. Import one clean table

1. Open **Data Library** and choose **Excel range…**.
2. Choose `simple-customers.xlsx` beside this FrameWork file.
3. Select `CustomersTable` — or enter `Customers!A4:D11` with **First row
   contains headers** enabled.
4. Preview the values, name the table `Customers`, and import it.

Checkpoint: Customers has seven data rows and the headers Customer ID, Name,
Region, and Active.

## 2. Import several defined tables from one workbook

Choose `multi-table-operations.xlsx`. Import each named Excel Table separately:

- `InventoryTable` from `Operations!A4:F10`, named `Inventory`;
- `SuppliersTable` from `Operations!H4:L8`, named `Suppliers`;
- `OrdersTable` from `Sales!B5:I25`, named `Orders`.

Checkpoint: Inventory has 6 rows, Suppliers has 4, and Orders has 20. The
Revenue values are the cached results saved by Excel; the Excel formulas
themselves are not copied into FrameWork.

## 3. Import pasted ranges without Excel Tables

The same workbook also contains two ordinary rectangular blocks. They are not
defined Excel Tables. Choose them under **Suggested region**:

- `Operations!A15:D23`, named `Adjustments`;
- `Sales!P15:S25`, named `Targets`.

The note in `Operations!N2` is intentionally ignored. Suggestions are
conservative starting points: preview them before importing, and type a manual
range when a workbook is too irregular.

Checkpoint: Adjustments has 8 rows and Targets has 10.

## Finish line

You should have six static FrameWork tables: `Customers`, `Inventory`,
`Suppliers`, `Orders`, `Adjustments`, and `Targets`. They are artifact-backed
imports: visible and queryable in FrameWork, but not presented as editable
source rows. Reopen the Answer key to compare names and row counts.

Regenerate the workbooks after changing the lesson or import behavior with:

```sh
cargo run -p framework-core --example generate_excel_import_tutorial
```

The desktop copies both Excel source files beside each editable tutorial
workbook, so the lesson works in an installed app without this repository.
