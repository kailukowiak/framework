# Demo runbook — Cascadia Outfitters accounting analysis

A scripted walkthrough over the generated demo dataset: import at scale, tour the core
functionality (Wrangle filters, calculated columns, plots, branches, joins, formats),
and "discover" the nine seeded findings live. Written against current `main`
(anti/semi joins, file connectors, paged tables, column formats, keyboard nav all landed).

**Data**: `demo-data/` at the repo root. Regenerate any time with
`uv run examples/generate_accounting_demo.py` (seeded — refs and amounts below are stable).
If a full 1.18M-line import feels slow on the demo machine, `general_ledger_2025q4.csv`
(151k lines) runs every act except the FY2024 comparisons.

**Setup**: open a new canvas → Add data → **Open file…** → `demo-data/general_ledger.parquet`.
The table lands as a file connector (inspector shows the source; **Refresh snapshot** re-reads it).
Repeat later for `budget_2025.csv`, `bank_statement_operating.csv`, `ar_invoices.csv`,
`chart_of_accounts.csv` as each act needs them.

---

## Act 1 — Scale and first look

1. Point at the row count under the imported ledger: **1,179,288 journal lines** — roughly
   an Excel workbook's worth of pain, scrolling smoothly in a virtualized view.
2. Scroll hard; note the header stays and paging keeps up (Polars-backed pages, not DOM rows).
3. Talking point: this is a *file connector* — the source file is the artifact; Refresh
   re-reads it while every table, formula, and lineage cord keeps its identity.

## Act 2 — Trial balance, and the books don't balance

1. From the ledger, create a **grouped summary**: group by `period`, aggregating
   `` `debit`.sum() `` and `` `credit`.sum() ``. Instant 24-row trial balance skeleton.
2. Now the finding. Group the ledger by `je_id` with one aggregate:

   ```
   (`debit`.fill_null(0) - `credit`.fill_null(0)).sum()
   ```

   Name it `imbalance`.
3. Create a child of that result and add a data filter: `` `imbalance` != 0 ``.
4. **Expected result — 3 rows**: the JEs for `BILL-300000` (+0.01), `BILL-300001` (−0.01),
   `BILL-300002` (+100.00). Out of 304,392 entries, three don't balance, found in seconds.
   In Excel this is a pivot over a million rows plus manual scanning.

## Act 3 — Calculated column + an analysis branch: the cutoff catch

1. On the ledger, **Add calculated column** `posted_year`:

   ```
   `posted_date`.dt.year()
   ```

2. Create a table tab named **Cutoff catch** and add both predicates as a **Filter rows**
   step in Wrangle:

   ```
   `posted_year` == 2026
   ```
   ```
   `period` == "2025-12"
   ```

3. **Expected result — ~400 rows**: invoices *posted-dated January 2026* but booked into
   *period 2025-12*. Revenue pulled across year-end — a cutoff error, caught by comparing a
   derived column against a data column. Note for the audience: this is a real analysis
   branch. The ledger remains complete, while anything derived from Cutoff catch inherits
   the predicates visibly through lineage.
4. Bonus column while you're here — fiscal year from the period string:

   ```
   when(`period` >= "2025").then(2025).otherwise(2024)
   ```

   (Zero-padded `YYYY-MM` sorts lexicographically, so string comparison is exact here.)

## Act 4 — Budget vs. actuals with exception views

1. Import `budget_2025.csv`.
2. From the ledger, derive FY2025 P&L actuals: filter `` `period` >= "2025" `` and
   `` `account_code` >= "4000" ``, then group by `account_code`, `department`, `period`
   with `` (`credit`.fill_null(0) - `debit`.fill_null(0)).sum() `` named `actual`.
3. Add the same explicit `Key` column to each table with
   `` concat_str([`account_code`, `department`, `period`], "-") ``, then **Join** actuals
   to budget on Key. The dialog will ask for the unique key on the lookup side — click
   **Mark unique**. A joined result remains an ordinary Wrangle input, so the next columns
   are added directly to it.
4. Add columns `` `actual` - `budget_amount` `` (variance) and
   `` (`actual` - `budget_amount`) / `budget_amount` `` (variance %).
5. **Formats moment**: set variance to **accounting** style, scale **thousands** — parens
   negatives, zero-dash, and a "$K" badge in the header. Set variance % to **percent**.
6. Exceptions, both directions, each one anti-join:
   - Budget rows with no actuals → the **RETAIL** department: budgeted $4,500/mo all year,
     spent nothing — it doesn't exist in the GL.
   - Actuals rows with no budget line → the ~3% of combinations finance never budgeted.
   The join dialog labels these "rows without a match"; each renders as its own exception
   table with lineage cords back to both parents.

## Act 5 — Bank reconciliation

1. Import `bank_statement_operating.csv`. Derive the GL cash side: filter the ledger to
   `` `account_code` == "1010" ``.
2. **Inner join** bank↔GL on `reference` = `source_ref`, then add
   `` `amount` + `credit`.fill_null(0) - `debit`.fill_null(0) `` as `difference`
   (bank outflows are negative), then branch and filter to `` `difference` != 0 ``.
   - **Expected**: `CHQ-6584` — GL says 5,728.65, bank cleared 5,674.65.
     Difference **54.00, divisible by 9** — the classic transposition tell. Accountants
     will finish that sentence for you.
3. Anti-join GL→bank: outstanding cheques and year-end deposits in transit.
4. Anti-join bank→GL: the **SERVICE CHG ADJ, 347.50 on 2025-08-14** that nobody ever
   booked, plus service fees for **2024-03 and 2025-07** (GL books fees a month in arrears
   and skipped those two entirely).
5. Duplicate payment: filter bank to `` `description` == "CHEQUE" ``, group by `amount`
   with `` `reference`.count() `` , child-filter count > 1 →
   **5,273.04 appears twice**: `CHQ-5090` and `CHQ-5090-DUP`, same vendor, four days apart,
   both cleared. Make a filtered ledger tab using
   `` `memo`.str.contains("CHQ-5090") `` to show both payments against the same bill.

## Act 6 — The auditor's filter

Branch the ledger to **Auditor filter**, then add one Filter rows step matching **all**:

```
`source` == "Manual"
```
```
`debit` % 1000 == 0
```
```
`created_by` == "jsmith"
```

**Expected — JE-MGMT-1/2/3**: round $50k/$75k/$100k revenue accruals posted 2025-12-31,
memo "Revenue accrual - management adjustment." Let the room react.

## Act 7 — AR aging, with the plot

1. Import `ar_invoices.csv`. In a block named **Assumptions**, add
   `As of = 2025-12-31`. Change that line later and watch everything reflow.
2. Calculated column `age_bucket` on invoices:

   ```
   when(`status` == "Paid").then("Paid")
   .when(`due_date` <= `Assumptions`.`As of`.dt.offset_by("-90d")).then("90+ days")
   .when(`due_date` <= `Assumptions`.`As of`.dt.offset_by("-60d")).then("61-90 days")
   .when(`due_date` <= `Assumptions`.`As of`.dt.offset_by("-30d")).then("31-60 days")
   .when(`due_date` <= `Assumptions`.`As of`).then("1-30 days")
   .otherwise("Current")
   ```

3. Group by `age_bucket`, aggregating `` `total`.sum() `` and `` `invoice_no`.count() ``
   (**6,227 open invoices** total). Accounting format, $K scale.
4. **Plot moment**: on the bucket summary, choose **Generate plot** — a Vega-Lite bar chart
   scaffolds automatically (buckets nominal, totals quantitative). Do the same on the Act 2
   trial balance for revenue-by-month: the seasonality (summer peak, December bump) is
   visible immediately, and the plot re-renders when upstream data changes.
5. Tie-out for the accountants: open AR total vs. the GL 1100 balance — two summaries,
   side by side on the canvas.

---

## Answer key (all nine seeded findings)

| # | Finding | Where it surfaces |
|---|---------|-------------------|
| 1–3 | `BILL-300000/1/2` out of balance +0.01 / −0.01 / +100.00 | Act 2 |
| 4 | ~400 cutoff invoices (posted 2026-01, period 2025-12) | Act 3 |
| 5 | RETAIL dept budgeted, never spent | Act 4 |
| 6 | `CHQ-6584` transposition, 54.00 diff (÷9) | Act 5 |
| 7 | `CHQ-5090` / `-DUP` duplicate payment, 5,273.04 ×2 | Act 5 |
| 8 | SERVICE CHG ADJ 347.50 + unbooked fees 2024-03, 2025-07 | Act 5 |
| 9 | JE-MGMT-1/2/3 round-dollar year-end accruals by jsmith | Act 6 |

## Functionality demonstrated

Import at scale (parquet connector, refresh) · visible Wrangle filters and branches ·
calculated columns (dates, conditionals, string contains) · grouped aggregates · derived
children of results · joins with enforced unique keys · **anti/semi exception views** ·
accounting/percent formats with $K scaling · block lines as live assumptions ·
Vega-Lite plots · undo across all of it · lineage cords tracing every result to its sources.

## Good optional moments

- Click a header sort and then open Wrangle: the sort is declared lineage, which is why
  `.shift(1)` remains meaningful instead of changing under an invisible view order.
- Type `` `period`.str. `` and show the string-only completion list and argument help.
- In Scratchwork, use `` `ledger`.`debit`.filter(`source` == "Manual").sum() `` to show
  the SUMIF idea as one composable expression rather than a parallel function family.
