Build a small launch-planning model in FrameWork. Do the work through the
FrameWork MCP server and leave the finished model in the open document.

Hard requirements:

1. Create an editable source table with six launch rows. It needs a stable
   numeric Line column, a SKU column, a Units column, and a Launch month
   column. The first two dates are 2026-09-01 and 2026-10-01.
2. Create a live frame from that source. In the live frame, sort by Line and
   calculate Launch month as a monthly sequence beginning 2026-09-01 whose
   number of periods is `frame.len()`. Do not hard-code six generated dates.
3. Create an editable product lookup with one unique row for each of these
   SKUs and values:
   - A-100, Aurora, West, 250
   - B-200, Boreal, East, 180
   - C-300, Cedar, North, 320
   - D-400, Delta, South, 210
   Explicitly declare SKU as its unique key.
4. Left join the live launch frame to the product lookup by SKU. Keep every
   launch row and bring over Product, Region, and Unit price. Add Revenue as
   Units times Unit price.
5. Make a separate two-column Scenarios table from two equal-length vectors:
   Scenario = Base, Upside, Downside and Multiplier = 1, 1.15, 0.85. It must
   retain a generated-vector rule and a paired-vector/zip step rather than
   becoming a hand-entered two-column table.
6. Recreate the tutorial's Calculation Matrix workflow with four standalone
   Variables (not hand-entered tables):
   - Scenario = Base, Upside, Downside
   - Multiplier = 1, 1.15, 0.85
   - Quarter = Q1, Q2, Q3, Q4
   - Base revenue = 100, 110, 120, 130
   Build a Calculation Matrix named Scenario × Quarter. Zip Scenario and
   Multiplier on Rows, zip Quarter and Base revenue on Columns, and use this
   exact body formula so the result exercises chained text formatting:

   (`Base revenue` * `Multiplier`).round(2).cast("string") + " {}".format(`Quarter`)

   It must produce three row tuples, four column tuples, and twelve live
   answers.
7. Leave the editable launch source at six rows. The verifier will append a
   seventh C-300 row and expects the monthly date calculation and joined
   result to grow live.

Use semantic names where named tools support them. Inspect the operation
catalog when you need a pipeline, linked frame, vector-pairing step, join,
Variable, or Calculation Matrix. Do not import helper files and do not paste a
precomputed answer table.
