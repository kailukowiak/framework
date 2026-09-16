# Scenarios, sensitivity and goal seek

About 25 minutes. Open **Scenarios, sensitivity and goal seek — Start** from
**Data library → Tutorials and examples**, or open the linked `.fw` files
with **File → Open**. The answer key is a separate workbook.

This is the third finance lesson. It builds the job the hard way first, with
`under` and `solve` not yet in view, so the pain they remove is visible
before they show up. Phase 3 of
[the finance build plan](../../docs/finance-build-plan.md), evaluate under
overrides, has landed, and section 5 below replaces sections 2-4's hand work
with it.

## Why this lesson exists

Scenarios already work: the tour switches a whole model between Base,
Upside and Downside with one menu. What a finance person does next is put
the three answers side by side, vary two assumptions in a grid, and ask
what price hits a target. Every one of those is "compute the model as if
this assumption were different, without changing it". Today FrameWork can
only change it. So the comparison is a table you fill in by hand, the grid
is the model retyped as a formula, and the goal seek is a scan you read by
eye. This lesson does all three the hard way first, in sections 2-4, so
section 5's `under` and `solve` read as the obvious fix rather than magic.

## Files

- [`scenarios-and-sensitivity-start.fw`](scenarios-and-sensitivity-start.fw)
  — an `Assumptions` group with `Price`, `Annual units`, `Unit cost` and
  `Fixed costs`; `Upside` and `Downside` scenarios already defined; and a
  `Plan` table of twelve months with seasonality weights.
- [`scenarios-and-sensitivity-finished.fw`](scenarios-and-sensitivity-finished.fw)
  — the answer key, built with today's catalog.

## 1. Build the model

Select `Plan`, open **Wrangle**, and add three calculated columns, one step
at a time:

```text
Units = `Annual units` * `Weight` / 100
Revenue = `Units` * `Price`
Cost = `Units` * `Unit cost`
```

Format Revenue and Cost as USD Accounting.

Add a block named `Model` with:

```text
revenue = `Plan`.`Revenue`.sum()
gross = revenue - `Plan`.`Cost`.sum()
ebitda = gross - `Fixed costs`
margin = ebitda / revenue
```

Checkpoint: `960000.00`, `400000.00`, `150000.00`, `0.1562`.

The model is small on purpose. What matters is that part of it lives in a
table: the price is applied month by month, inside `Plan`, not in the block.

## 2. Compare scenarios by hand

Use the **Scenario** menu in the bottom-right corner of the canvas. Switch
to `Upside`, read the four lines of `Model`, and write them down. Switch to
`Downside` and do it again. Switch back to `Base`.

Checkpoint, which you now have on paper and nowhere in the document:

| | Base | Upside | Downside |
|---|---:|---:|---:|
| revenue | 960,000 | 1,187,500 | 747,500 |
| gross | 400,000 | 522,500 | 292,500 |
| ebitda | 150,000 | 272,500 | 42,500 |

That table is what gets presented. The document can produce each column
but cannot hold all three at once, because it can only be in one scenario
at a time.

## 3. A sensitivity grid, by retyping the model

Click **Variable** in the left rail and enter:

```text
Price axis = [100, 110, 120, 130, 140]
```

Click **Variable** again:

```text
Units axis = [6000, 7000, 8000, 9000, 10000]
```

Add a blank **Calculation Matrix** named `EBITDA by price and units`. Drag
`Price axis` into **Rows** and `Units axis` into **Columns**. Enter this body:

```text
(`Price axis` - `Unit cost`) * `Units axis` - `Fixed costs`
```

Checkpoint: the centre cell, price 120 by 8,000 units, reads `150000.00`, the
same as `Model.ebitda`. The corners read `-70000.00`, `50000.00`,
`170000.00` and `450000.00`.

Now read the body formula. It is not the model. It is the model retyped as
a closed form, and it agrees with `Model` only because the seasonality
weights sum to 100 so the months can be ignored. The matrix cannot ask
`Model.ebitda` what it would be at a different price, because the price is
applied inside `Plan`, and the matrix has no way to reach into `Plan` with a
different value. The moment the model has a per-month price list, a cost
step, or anything that does not collapse to one line, this grid cannot be
built at all.

## 4. Goal seek, by scanning

What price makes EBITDA 300,000 at the base volume? Click **Variable** and
enter:

```text
Price scan = sequence(100, 145, 5)
```

Add a blank **Calculation Matrix** named `EBITDA by price`. Drag
`Price scan` into **Rows**. For **Columns**, add a variable
`Answer = ["EBITDA"]` and drag it in. Enter the body:

```text
(`Price scan` - `Unit cost`) * `Annual units` - `Fixed costs`
```

Checkpoint: the column climbs from `-10000.00` at 100 to `310000.00` at 140. The
target falls between 135 and 140. To get closer you narrow the scan and
look again.

## 5. The easy way

Everything sections 2-4 did by hand — reading a scenario, retyping the
model into a grid, scanning for a target — is `under` and `solve` doing it
in place, against the model itself.

Add two lines to `Model`:

```text
upside ebitda = under(`Upside`, ebitda)
downside ebitda = under(`Downside`, ebitda)
```

Checkpoint: `272500.00` and `42500.00`, with the document still on Base —
no scenario switch, section 2's table gone.

Open the section-3 matrix card, `EBITDA by price and units`. On the Rows
axis, use the "Rows · Row input" selector to bind `Price axis` to `Price`;
on the Columns axis, use "Columns · Column input" to bind `Units axis` to
`Annual units`. Replace the body with:

```text
`Model`.ebitda
```

Checkpoint: the same 25 numbers as before (centre `150000.00`, corners
`-70000.00`, `50000.00`, `170000.00`, `450000.00`), now computed through
`Plan` rather than retyped around it, and the card says `25 evaluations`.
Change a seasonality weight in `Plan` — the grid follows, which the retyped
version in section 3 could not do.

Add one more line to `Model`:

```text
target price = solve(`Model`.ebitda == 300000, by=`Price`, within=[100, 200])
```

Checkpoint: `138.75`. The gutter shows the iterations and residual, and an
**Apply** action sets `Price` to the answer as an ordinary, undoable edit.
Ask for a bracket that contains no crossing, `within=[100, 120]`, and it is
refused with a reason rather than a number.

The rule under all three: evaluating under an override must not activate a
scenario, materialise anything, or change the document. Property test:
for every scenario, `under(scenario, x)` equals activating that scenario and
reading `x`.

## Smoke-test notes

If a named control is missing, a step cannot be completed, or a checkpoint
differs, record it with the template in the parent tutorial README rather
than working around it. Expected awkwardness that is product feedback, not
a lesson error: the hand-filled table in section 2, and the retyped model in
sections 3 and 4 — that is the contrast section 5 exists to make.

## Rebuilding the files

```bash
cargo run -p framework-core --example generate_finance_tutorials scenarios-and-sensitivity
```

The generator asserts every checkpoint above against the reloaded answer
key, including switching scenarios and reading `Model` the way the reader
does. The expected numbers were computed independently in Python before the
workbook existed.
