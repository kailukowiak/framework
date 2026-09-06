# The FrameWork tour

Start here. This is one pass over the things FrameWork does that a spreadsheet
does not, using six rows of monthly sales that the rest of the tutorials use
too. In about 25 minutes you will paste a typed table, write one formula for a
whole column, declare an order, keep a scratchpad in its own window, join a
budget by dragging a header, summarize a branch, run the same model under three
sets of assumptions, and chart the result.

It is a tour, not a reference. Each section shows one idea and moves on; the
lesson that teaches it properly is named at the end of the section. Nothing
here is a preview or a demo mode—every gesture is the real one, and the
workbook you finish with is a real model.

This guide is rendered as the **Tutorial walkthrough** markdown card in both
the Start workbook and the Answer key.

## Files

- [`grand-tour-start.fw`](grand-tour-start.fw) — an empty `Monthly sales`
  table and a `Budget` table.
- [`grand-tour-finished.fw`](grand-tour-finished.fw) — the answer key.

## Before you begin

Allow about 25 minutes. No prior FrameWork experience is assumed, and no
earlier lesson is required.

In the app, open **Data Library**, choose **Create tutorials** if the tutorial
workbooks are not present, and open **The FrameWork tour — Answer key** for a
minute before opening the Start workbook. Make every change in the Start
workbook. Repository contributors can instead open the two linked `.fw` files
with **File → Open**.

Work the sections in order. Later sections read the columns and tables the
earlier ones create, so a checkpoint that disagrees is worth fixing before
continuing rather than after.

The lessons are also manual smoke tests. If a named control is missing or a
checkpoint differs, record it with the template at the end rather than working
around it.

## 0. See the destination

Open the finished workbook. On the canvas you should find a `Monthly sales`
table with Profit and Forecast columns, a `Sales vs budget` table that carries
a Variance column and a chart on its tab strip, a `By region` summary, an
`Assumptions` group holding one `Growth` value, and a `Scratchwork` block with
three live answers.

In the bottom-right corner of the canvas there is a **Scenario** menu reading
`Base`. Choose `Upside`, then `Downside`, and watch the Forecast column and the
`Forecast total` answer move together. Put it back to `Base`.

That is the whole tour in one gesture: nothing was copied to make those three
answers, and nothing has to be put back by hand.

Now open the Start workbook.

## 1. Paste a table

Select the first cell of the empty `Monthly sales` table and paste this whole
tab-separated block:

```text
Month	Region	Revenue	Cost
2026-04	West	142000	91000
2026-01	East	118000	76000
2026-06	East	168000	104000
2026-03	East	136000	85000
2026-02	West	124000	79000
2026-05	East	151000	96000
```

Checkpoint: the table has four named columns and six rows. Revenue and Cost
arrive as numbers, not as text that looks like numbers.

There was no *convert to table* step, because a table is the only thing a
FrameWork frame can be. Its columns have names and types from the moment they
exist, which is what lets every later section refer to `Revenue` instead of to
a rectangle of cells. Pasting onto empty canvas, with nothing selected, makes a
whole new table the same way.

*Taught properly in: [Your first FrameWork workbook](../first-workbook/README.md).*

## 2. Find anything, run anything

Three keys are worth learning before anything else.

1. Press **⌘F** and type `142000`. The Find palette searches the whole
   workbook—card names, column names, formulas, Scratchwork lines, and the
   values in your tables. Choose the result and FrameWork brings that card
   forward and selects the cell.
2. Press **⌘⇧P**. Quick Commands is a searchable list of everything FrameWork
   can do to this document, canvas and view, each with its shortcut. Type
   `scratch` and read the two entries there without running them; press
   **Escape**.
3. Press **⌘K**. The Reference covers every formula function and the rules
   behind Variables, Scratchwork, Wrangle, ownership and row order. While a
   formula editor is active it opens on formulas automatically. Press
   **Escape**.

Checkpoint: ⌘F selected the April Revenue cell, and both palettes closed on
Escape leaving the workbook untouched.

You are never more than one key from the name of the thing you are looking for,
which matters more here than in a spreadsheet: FrameWork's controls are
attached to objects rather than spread across a ribbon.

## 3. One formula, one column

Right-click the `Cost` header and choose **Add calculated column**. Name it
`Profit` and enter:

```text
`Revenue` - `Cost`
```

Commit with **Return**.

Checkpoint: all six rows show a profit, and the values in chronological order
are `42000`, `45000`, `51000`, `51000`, `55000`, and `64000`.

There is nothing to fill down, and no last row to remember to extend. The
formula belongs to the column, so a row added tomorrow is already calculated.
That is also why the fill square at the corner of a selected column does not
copy cells here: drag it down and FrameWork opens the column's formula
instead, because writing the rule once is what filling a range was always
trying to approximate.

Select `Revenue`, `Cost` and `Profit` together—click the `Revenue` header,
then shift-click `Profit`—and use **Number format** on the **Format** tab to
make them Accounting, USD, no decimals. One change lands on all three columns.
Formatting changes how the numbers read, never what they are.

*Taught properly in: [Your first FrameWork workbook](../first-workbook/README.md).*

## 4. Declare the order

Click the ascending sort control in the `Month` header.

Checkpoint: rows run from `2026-01` to `2026-06`. Open **Wrangle** and find
**Sort** as the last step in the transformation chain.

This is the difference worth carrying out of the tour: the sort is not a way of
looking at the table, it is part of what the table *is*. Every table derived
from this one inherits it, and a row-relative calculation such as
`` `Revenue`.shift(1) `` is only accepted once an order has been declared—
otherwise "the previous row" would mean whichever row happened to arrive first.

*Taught properly in: [Month-over-month formulas by pointing](../formula-clicks/README.md).*

## 5. Scratchwork, and Scratchwork in its own window

Press **⌘J** anywhere. A `Scratchwork` block appears on the canvas with the
cursor on its first line. This is where one-off arithmetic goes—the back of the
envelope, kept in the document.

Type the name and the equals sign:

```text
Total revenue = 
```

Now, without leaving the line, click the `Revenue` column header in
`Monthly sales`. The reference is written for you. Add `.sum()` so the line
reads:

```text
Total revenue = `Monthly sales`.`Revenue`.sum()
```

Checkpoint: the gutter beside the line shows `839000`.

Now choose **Window → Scratchwork Window**. The same block opens as an ordinary
window of its own, on the same text, the same undo history and the same live
answers. Put it beside the canvas, click a column in the main window, and the
reference still lands in the Scratchwork line you are editing. **⌘J** raises
the window when it is already open.

Checkpoint: editing the line in either place changes it in both, and closing
the Scratchwork window leaves the block on the canvas with the line intact.

The answers are queries, not captured cells. Change a Revenue value in the
table and the total follows it; undo and it comes back.

*Taught properly in: [Your first FrameWork workbook](../first-workbook/README.md).*

## 6. Bring the budget over by dragging a header

`Budget` holds one budgeted revenue per month. Rather than writing a lookup,
match the two tables on their key.

1. In `Budget`, click the **Budget** header to select that column.
2. Drag it onto the **Month** header in `Monthly sales`.

The compact prompt should read as an equation:

```text
Match Monthly sales.Month to Budget.Month
```

Wait for the diagnostics, which are computed over the full dataset rather than
the rows on screen. They should report 6 matched, 0 missing, 0 duplicate keys.
Click **Mark Month as unique**, then **Bring columns over**. Rename the result
`Sales vs budget`.

In **Wrangle** on `Sales vs budget`, add a calculated column:

```text
Variance = `Revenue` - `Budget`
```

Format it as USD Accounting.

Checkpoint: `Sales vs budget` has six rows, January shows a Variance of
`-2000`, and June shows `8000`. Open Wrangle and read the first step: the key
relationship is a visible step, not a dialog that has disappeared.

FrameWork refuses to build the join until the lookup side declares its key
unique, because a duplicate key is how a lookup silently multiplies rows. If
the table has grown wider than the window, right-click a header and choose
**Pin columns through here** to freeze everything up to it while the rest
scrolls.

Add a second Scratchwork line:

```text
Total variance = `Sales vs budget`.`Variance`.sum()
```

Checkpoint: the answer is `21000`.

*Taught properly in: [Month-end close](../month-end-close/README.md) and
[Vectors, dates, and visual joins](../vectors-and-joins/README.md).*

## 7. Branch a tab and summarize

Use the **+** beside the `Sales vs budget` table tabs and choose **Table view**.
Rename the new tab `By region`. In **Wrangle**, add **Summarize**:

- Group by: Region
- Total Revenue: `` `Revenue`.sum() ``
- Total Budget: `` `Budget`.sum() ``
- Total Variance: `` `Variance`.sum() ``

Checkpoint:

| Region | Total Revenue | Total Budget | Total Variance |
|---|---:|---:|---:|
| East | 573000 | 560000 | 13000 |
| West | 266000 | 258000 | 8000 |

Switching back to the `Sales vs budget` tab still shows all six rows. A branch
is a separate table with its own visible chain reading the same source—not a
copy of the rows, and not a hidden filter that can disagree with what is
downstream of it. The summary reconciles to the whole table because it is
reading the whole table.

*Taught properly in: [Month-end close](../month-end-close/README.md).*

## 8. Ask "what if" without copying the model

Click **Container** in the left rail and rename the new group `Assumptions`.
Inside it, click **+ Value**, rename the value `Growth`, and give it:

```text
1.08
```

Back on `Monthly sales`, add a calculated column named `Forecast`, and format
it as USD Accounting like the others:

```text
(`Revenue` * `Growth`).round()
```

A column formula is an ordinary expression, so rounding to the dollar is a
method on the arithmetic rather than a wrapper function around the whole thing.

Checkpoint: the January Forecast is `127440`, and adding a third Scratchwork
line

```text
Forecast total = `Monthly sales`.`Forecast`.sum()
```

answers `906120.00`.

Now select the `Growth` value card. In **Selection** you will find a
**Scenarios** grid below its number. Type `Upside` into the **New scenario**
row and press **Return**, give it the value `1.15`, then add `Downside` with
`0.95`.

Use the **Scenario** menu in the bottom-right corner of the canvas to switch
between them. It appears only once a document has a scenario to switch to.

Checkpoint: `Forecast total` reads `906120.00` on Base, `964850.00` on Upside,
and `797050.00` on Downside. Leave it on **Base** before continuing.

Nothing was duplicated to get three answers. A scenario is a small set of
disagreements about named assumptions, and every column, result and Scratchwork
line that reads one moves when you switch. A blank cell in the Scenarios grid
means "same as the base", so a scenario stays as short as what actually differs.

## 9. Chart the table beside the table

Select `Sales vs budget`, use the **+** beside its table tabs, and choose
**Plot**. In the plot inspector choose:

- chart type: Bar;
- X: Month;
- Y: Revenue;
- Color: Region.

Rename it `Revenue by month`.

Checkpoint: six bars, split between East and West, sharing a card with the
table they describe. Change a Revenue value upstream and the bar moves.

The plot reads the table by identity, and it reads all of it—not the first
thousand rows, and not a copied range that has to be re-pointed when the data
grows.

*Taught properly in: [Your first FrameWork workbook](../first-workbook/README.md).*

## Finish line

Your workbook should now agree with `grand-tour-finished.fw`:

- 6 rows in `Monthly sales`, sorted January to June, with Profit and Forecast;
- `Sales vs budget` with six rows and a Variance column;
- `By region` with two rows reconciling to `21000` of variance;
- `Growth` at `1.08`, with an Upside and a Downside;
- Scratchwork answering `839000`, `21000`, and `906120.00` on Base;
- a `Revenue by month` chart on the analysis tab strip.

Two last things worth knowing:

- Undo travels with the model. Undo the join and the Variance column, the
  summary and the chart go with it; redo brings them all back. Try it.
- Open the **Project** panel in the left rail and choose **Export to Excel…**.
  The workbook that comes out holds the results as values, and the checkbox
  **Include a sheet explaining how each column was computed** adds one tab
  saying which of them were formulas and what they read. People who need a
  spreadsheet get a spreadsheet; you keep the model.

## Where to go next

The tour showed each of these once. In order, the lessons that teach them:

1. [Your first FrameWork workbook](../first-workbook/README.md) — the same
   ground at walking pace, plus formatting, a live markdown narrative, and
   plots. About 15 minutes.
2. [Importing an Excel workbook](../excel-import/README.md) — bringing real
   `.xlsx` sheets in. About 10 minutes.
3. [Month-over-month formulas by pointing](../formula-clicks/README.md) —
   `.shift(1)`, click-to-build, table-first autocomplete. About 15 minutes.
4. [Excel concepts in FrameWork](../excel-to-framework/README.md) — a
   translation table for the habits that do not carry over.
5. [Month-end close](../month-end-close/README.md) — the full package: joins,
   summaries, pivots, exceptions, control totals. About 30–45 minutes.
6. [Vectors, dates, and visual joins](../vectors-and-joins/README.md) —
   vectors, row-count-aware date series, and the Calculation Matrix. About 20
   minutes.

## Smoke-test notes

This lesson is the broadest manual smoke test in the set: it crosses paste,
find, calculated columns, declared order, Scratchwork and its window, joins,
branches, summaries, values, scenarios, plots and export. Capture the first
broken checkpoint with the template in the parent tutorial README, and say
whether the control could be found without the guide.

## Rebuilding the files

Both workbooks are generated through `Store::apply(Operation::...)`, the same
validation, history and persistence boundary the desktop and MCP use:

```bash
cargo run -p framework-core --example generate_tutorial_workbooks -- grand-tour
```

The generator reloads the answer key and proves the joined variance, the
regional summary, the three Scratchwork answers, and the forecast under each
scenario.
