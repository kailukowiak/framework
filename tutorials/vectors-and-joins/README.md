# Vectors, dates, and visual joins

This final tutorial builds one small launch model three ways people already
understand from a spreadsheet: continue a visible date pattern down a table,
put two vectors beside one another, and drag lookup columns onto a matching key.
The saved result remains a FrameWork model rather than a pasted answer.

This same guide is rendered as the **Tutorial walkthrough** markdown card on
the left side of both the Start workbook and the Answer key.

It demonstrates:

- an Excel-like two-cell gesture that becomes a row-count-aware date formula;
- why a free-standing vector has its own length while a table-bound vector uses
  `frame.len()`;
- creating a one-column table by dragging a vector to empty canvas;
- pairing a second, equal-length vector on the table's right edge;
- selecting lookup columns and dragging them onto a destination key;
- full-dataset match and duplicate diagnostics before a join is created;
- live propagation when the source table gains a row.

## Files

- [`vectors-and-joins-start.fw`](vectors-and-joins-start.fw) — launch inputs, a
  linked plan, product catalog, empty vector container, and empty checks block.
- [`vectors-and-joins-finished.fw`](vectors-and-joins-finished.fw) — the answer
  key with the date series, two-vector table, validated join, and live checks.

## Before you begin

Allow about 20 minutes. In **Data Library**, choose **Create tutorials** if the
lesson is not present, open the answer key for one minute, then make every
change in the Start workbook. Repository contributors can instead open the two
linked `.fw` files with **File → Open**.

The canvas begins with four useful objects:

- `Launch inputs` owns the six rows you can edit;
- `Launch plan` is a live table made from those inputs;
- `Product catalog` has one row per SKU but is not marked unique yet;
- `Scenario vectors` is an empty container where the two vectors will live.

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

## 2. Make two literal vectors

In `Scenario vectors`, click **Vector** and create this first vector:

- Name: `Scenario`
- Values:

```text
Base, Upside, Downside
```

Click **Vector** again and create the second vector:

- Name: `Multiplier`
- Values:

```text
1, 1.15, 0.85
```

Checkpoint: both vector cards say **3 values**. Scenario is text and Multiplier
is numeric. The values wrap across each card; pasted spreadsheet columns,
`[1, 1.15, 0.85]`, NumPy `array(...)`, and R `c(...)` input work too.

## 3. Create a table from the two vectors

1. Drag the **3 values** footer on `Scenario` to empty canvas.
2. Rename the new one-column table `Scenarios`.
3. Drag the **3 values** footer on `Multiplier` to the `+` edge immediately to
   the right of the Scenario column.

Checkpoint:

| Scenario | Multiplier |
|---|---:|
| Base | 1 |
| Upside | 1.15 |
| Downside | 0.85 |

Open **Wrangle** on `Scenarios`. The second gesture is recorded as one compact
paired-vector step. It is not three copied cell formulas.

Try undo and redo once. Then temporarily add a fourth value to only one source
vector. The table should report a length mismatch instead of shifting values
into the wrong rows. Undo that edit before continuing.

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
Rows scheduled = `Scheduled launches`.len()
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

You have now used three related but different ideas:

1. **A literal vector** is a small named collection with an intentional length.
2. **A table-bound vector formula** produces one value per current row by using
   `frame.len()` and a declared order.
3. **A join** matches by key, not by row position, and refuses an unsafe lookup
   side until its uniqueness is explicit.

Those distinctions are the guardrails. The gestures stay Excel-simple, while
the Wrangle steps say what will remain true when the data changes.

## Smoke-test notes

This lesson is expected to work as a manual product smoke test. Capture the
first broken checkpoint with the template in the parent tutorial README.
The matching agent smoke scenario is `tools/mcp-smoke/scenarios/vectors-joins`;
it builds and verifies the same structural promises without pretending MCP can
perform pointer gestures.

## Rebuilding the files

The workbooks are generated through the same validated operation boundary used
by the desktop and MCP:

```bash
cargo run -p framework-core --example generate_tutorial_workbooks -- vectors-and-joins
```

The generator reloads the answer key and proves the date range, paired vector,
joined revenue, and upstream-row growth.
