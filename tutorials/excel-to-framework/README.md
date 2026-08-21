# Excel concepts in FrameWork

FrameWork is easiest to learn by translating jobs, not memorizing a second list
of function names. An Excel workbook often mixes data entry, formulas,
Power Query, lookup logic, PivotTables, and presentation across sheets.
FrameWork keeps those capabilities together on one canvas but makes their
lineage explicit.

## How to use this guide

This page is a translation reference, not a separate workbook. Use it when a
tutorial asks for a gesture that feels different from Excel:

1. Find the familiar Excel concept in the table below.
2. Read the FrameWork equivalent and the important difference together. The
   difference usually explains why there is no direct copy of the Excel UI.
3. Try the gesture in the linked practical lesson:

   - calculated columns, sorting, formatting, and plots in
     [Your first FrameWork workbook](../first-workbook/README.md);
   - relative-row formulas and table-qualified autocomplete in
     [Month-over-month formulas by pointing](../formula-clicks/README.md);
   - joins, summaries, pivots, and exception tables in
     [Month-end close](../month-end-close/README.md).
4. Return here when you are tempted to search for a fill handle, hidden filter,
   lookup formula, or frozen intermediate result.

| Excel concept | FrameWork equivalent | Important difference |
|---|---|---|
| Worksheet range | Table | A table has identity, types, and named columns from the start. |
| Excel Table | Table | No separate “convert to table” step. |
| Cell formula copied down | Calculated column in Wrangle | Write the rule once; appended rows inherit it. |
| `A2-B2` | `` `Revenue` - `Cost` `` | Names survive layout changes and renames resolve through stable IDs. |
| Relative row reference | `.shift(n)` | FrameWork requires a declared ordering before accepting row-relative math. |
| Named cell | Named formula-block line | Several scalar calculations live densely in one block. |
| `SEQUENCE` / fill series | `sequence(start, stop, step)` in Scratchwork | The stop is excluded; dates take a duration such as `1mo`. A list is not silently aligned to table rows. |
| Another sheet reference | `` `Table`.`Column` `` | Autocomplete can choose the table, then narrow its columns. |
| Sort/filter arrows | Sort or Filter in Wrangle | The choice is visible lineage and reaches downstream tables. |
| Duplicate sheet for another analysis | Branched table tab | The branch has its own chain without copying source rows. |
| `VLOOKUP` / `XLOOKUP` | Join another table | The lookup side must declare a unique key, preventing accidental row multiplication. |
| `SUMIFS` family | Filter rows + Summarize | Filtering and aggregation are separate, inspectable operations. |
| PivotTable | Summarize or Pivot | The result is another ordinary table that can feed later work. |
| Power Query | Wrangle | The transformation chain stays beside the result on the canvas. |
| Chart source range | Plot reading a table | The plot reads the table by identity rather than a coordinate rectangle. |
| Save a value result | Freeze this answer | Freezing is explicit; a live source is not silently cached. |

## Three habits to change

1. **Name the information, not its coordinates.** Click-to-build can still be
   the authoring gesture, but the saved formula names the column and operation.
2. **Make order explicit.** A calculation such as “previous month” is only
   meaningful after Month has been declared as the ordering.
3. **Branch instead of hiding state.** If two analyses need different filters,
   give each one a named tab and visible Wrangle chain.

## Formula difficulty ladder

Use the formula catalog as a ladder rather than a checklist:

| Level | Start with | Example |
|---|---|---|
| Basic | arithmetic, comparisons, and direct column references | `` `Revenue` - `Cost` `` |
| Intermediate | typed namespaces, aggregates, and conditionals | `` when(`Revenue` > 1000).then("Large").otherwise("Regular") `` |
| Advanced | declared-order windows and generated lists | `` `Revenue`.shift(1) `` and `sequence(2026-01-01, 2027-01-01, 1mo)` |

Try `sequence` on a Scratchwork line. Open the result to inspect the whole
date spine and copy it. Its end is deliberately excluded, as in Python:
`sequence(1, 8, 2)` is `1, 3, 5, 7`. FrameWork will not line that list up with
a table merely because their lengths happen to match; filling table rows is
an ordered transformation, not an implicit positional join.

For Excel-style row numbering, first add a Sort in Wrangle, then add a
calculated column with `sequence(1, table.len() + 1)`. `table.len()` counts the
rows reaching that point in the chain, so a Filter before it changes the count
and a Filter after it does not renumber the surviving rows.

## A five-minute translation check

Use any scratch workbook to verify the three habits before starting the
advanced project:

1. Add a calculated column using a column name rather than a cell coordinate.
   Confirm that there is no fill-down step.
2. Sort a table from its header and confirm that **Sort** appears in Wrangle.
3. Branch a table, filter only the branch, and switch between the two tabs.
   Confirm that the source keeps all of its rows.

If any result is surprising, revisit **Three habits to change** before moving
on. These three checks cover naming, order, and branching—the ideas the
month-end project assumes.

## What not to hunt for

- There is no fill handle for repeating a column formula.
- There is no hidden display-only filter layer.
- There is no need to encode lookups as formulas once two tables have a key.
- A joined result is an ordinary Wrangle input: add calculated columns,
  filters, sorts, and summaries directly after the fixed join step.

Continue with [Month-end close](../month-end-close/README.md) to apply these
translations in one model.
