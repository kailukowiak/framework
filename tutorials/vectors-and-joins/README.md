# Vectors, Calculation Matrix, dates, and visual joins

This final tutorial builds one small launch model with four spreadsheet-shaped
gestures: continue a visible date pattern down a table, write four standalone
vectors, run a calculation across every combination of them, and drag lookup
columns onto a matching key. The saved result remains a FrameWork model rather
than a pasted answer.

This same guide is rendered as the **Tutorial walkthrough** markdown card on
the left side of both the Start workbook and the Answer key.

It demonstrates:

- an Excel-like two-cell gesture that becomes a row-count-aware date formula;
- why a standalone vector has its own length while a table-bound vector uses
  `frame.len()`;
- writing a one-dimensional vector as one compact variable;
- using Calculation Matrix as a visual nested `for each` loop;
- selecting lookup columns and dragging them onto a destination key;
- full-dataset match and duplicate diagnostics before a join is created;
- live propagation when the source table gains a row.

## Files

- [`vectors-and-joins-start.fw`](vectors-and-joins-start.fw) — launch inputs, a
  linked plan, product catalog, and an empty checks block.
- [`vectors-and-joins-finished.fw`](vectors-and-joins-finished.fw) — the answer
  key with the date series, four variables, Calculation Matrix, validated join,
  and live checks.

## Before you begin

Allow about 25 minutes. In **Data Library**, choose **Create tutorials** if the
lesson is not present, open the answer key for one minute, then make every
change in the Start workbook. Repository contributors can instead open the two
linked `.fw` files with **File → Open**.

The canvas begins with four useful objects:

- `Launch inputs` owns the six rows you can edit;
- `Launch plan` is a live table made from those inputs;
- `Product catalog` has one row per SKU but is not marked unique yet;
- `Checks` is an empty Scratchwork block for the final control totals.

That source/plan split is deliberate. A calculated table reads its rows from
upstream. Adding a row to `Launch inputs` near the end will therefore prove that
the date formula and the join follow changing data rather than a fixed preview.

## 1. Continue the date pattern down the live table

`Launch plan` shows `2026-09-01` and `2026-10-01` in its first two **Launch
month** cells. The remaining cells are blank.

1. Select those first two date cells.
2. Right-click the selection and choose **Fill series down frame…**.
3. Keep the starting date `2026-09-01` and change of `1` month.
4. Choose **Line — ascending** for the row order.
5. Click **Fill column**.

Checkpoint: the six dates run monthly from `2026-09-01` through `2027-02-01`.
Open **Wrangle** and find a Sort followed by this calculated-column formula:

```text
sequence(2026-09-01, periods=frame.len(), step=1mo)
```

This is the important distinction:

- a vector such as `Base, Upside, Downside` owns exactly three values;
- a vector calculated in a table can ask for `frame.len()` values.

The fill dialog writes the second form by default. It also records the sort,
because a row-wise series without an order would attach dates to accidental
positions.

## 2. Make four standalone vectors

Click **Variable** in the left rail and replace the starter name and formula:

```text
Scenario = ["Base", "Upside", "Downside"]
```

Click **Variable** again and enter:

```text
Multiplier = [1, 1.15, 0.85]
```

Add two more variables:

```text
Quarter = ["Q1", "Q2", "Q3", "Q4"]
Base revenue = [100, 110, 120, 130]
```

Checkpoint: `Scenario` and `Multiplier` each have **3 values**; `Quarter` and
`Base revenue` each have **4 values**. Every one is a simple one-dimensional
vector: one name, one formula, and an intentional number of values. None is a
tiny table, and they do not need a container around them.

## 3. Build a Calculation Matrix

1. Add a blank **Calculation Matrix** and name it `Scenario × Quarter`.
2. Drag `Scenario` and `Multiplier` into **Rows**.
3. Drag `Quarter` and `Base revenue` into **Columns**.
4. Enter this body formula:

```text
`Base revenue` * `Multiplier`
```

Checkpoint: the matrix has three row tuples, four column tuples, and twelve
answers:

| Scenario | Multiplier | Q1 · 100 | Q2 · 110 | Q3 · 120 | Q4 · 130 |
|---|---:|---:|---:|---:|---:|
| Base | 1 | 100 | 110 | 120 | 130 |
| Upside | 1.15 | 115 | 126.5 | 138 | 149.5 |
| Downside | 0.85 | 85 | 93.5 | 102 | 110.5 |

Calculation Matrix is the visual answer to a nested loop:

```text
for each (Scenario, Multiplier) row:
    for each (Quarter, Base revenue) column:
        Base revenue * Multiplier
```

Rows and columns are independent axes, so every row tuple meets every column
tuple. Within one axis, fields are zipped: `Scenario` and `Multiplier` describe
the same three loop items, while `Quarter` and `Base revenue` describe the same
four. They do not create four Cartesian layers. Nothing is appended below or
copied into twelve separate formulas. Shorten both column vectors to two values;
the matrix should immediately become 3 × 2. Undo those edits before continuing.

## 4. Bring catalog columns over with a visual join

In `Product catalog`, select the adjacent headers **Product**, **Region**, and
**Unit price**. Drag that selected header run onto the **SKU** header in
`Launch plan`.

The compact lookup prompt should read as an equation:

```text
Match Launch plan.SKU to Product catalog.SKU
```

Wait for the full-dataset diagnostics. They should show:

- 6 matched;
- 0 missing;
- 0 duplicate keys.

Click **Mark SKU as unique**, then **Bring columns over**. Rename the result
`Scheduled launches`.

Checkpoint: all six Launch plan rows remain, and Product, Region, and Unit
price appear beside them. The lookup table shows a key marker on SKU. Open
Wrangle on the result: its first step should state the key relationship rather
than hiding it in a dialog that has disappeared.

If the prompt offers a wrong key, if the counts only reflect visible rows, or
if the join can be created without a unique lookup key, record that as a smoke
test failure.

## 5. Add one joined calculation and two live checks

On `Scheduled launches`, add a calculated column in Wrangle:

```text
Revenue = `Units` * `Unit price`
```

Format **Unit price** and **Revenue** as USD Accounting. In `Checks`, enter:

```text
Rows scheduled = `Scheduled launches`.`Line`.len()
Revenue scheduled = `Scheduled launches`.`Revenue`.sum()
```

Checkpoint:

- Rows scheduled is `6`;
- Revenue scheduled is `137600`;
- the first Aurora row has Revenue `30000`.

## 6. Prove that the model scales with its source

Add this seventh row to `Launch inputs`:

| Line | SKU | Units | Launch month |
|---:|---|---:|---|
| 7 | C-300 | 50 | blank |

Do not type a date into the new row.

Checkpoint:

- `Launch plan` grows to seven rows and calculates `2027-03-01`;
- `Scheduled launches` grows to seven rows and brings over Cedar, North, and
  the `320` unit price;
- Rows scheduled becomes `7`;
- Revenue scheduled becomes `153600`.

Undo and redo the source-row addition. The date, joined values, row count, and
revenue total should travel together. Nothing should require reapplying the
vector or recreating the join.

## Finish line

You have now used four related but different ideas:

1. **A standalone vector** is one named formula with an intentional length.
2. **A table-bound vector formula** produces one value per current row by using
   `frame.len()` and a declared order.
3. **A Calculation Matrix** evaluates every combination of its independent row
   and column axes.
4. **A join** matches by key, not by row position, and refuses an unsafe lookup
   side until its uniqueness is explicit.

Those distinctions are the guardrails. The gestures stay Excel-simple, while
the Wrangle steps say what will remain true when the data changes.

## Smoke-test notes

This lesson is expected to work as a manual product smoke test. Capture the
first broken checkpoint with the template in the parent tutorial README.
The matching agent smoke scenario is `tools/mcp-smoke/scenarios/vectors-joins`;
it builds and verifies the tabular promises without pretending MCP can perform
the tutorial's pointer gestures.

## Rebuilding the files

The workbooks are generated through the same validated operation boundary used
by the desktop and MCP:

```bash
cargo run -p framework-core --example generate_tutorial_workbooks -- vectors-and-joins
```

The generator reloads the answer key and proves the date range, Calculation
Matrix, joined revenue, and upstream-row growth.
