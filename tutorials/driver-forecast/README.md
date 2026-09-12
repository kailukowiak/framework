# Driver-based forecast

About 30 minutes. Open **Driver-based forecast — Start** from **Data library
→ Tutorials and examples**, or open the linked `.fw` files with **File →
Open**. The answer key is a separate workbook.

This is the second finance lesson. Like the others it builds the job with
today's FrameWork, shows where that hurts, and then shows what it becomes
when phase 2 of [the finance build plan](../../docs/finance-build-plan.md),
the period-aware time spine, lands.

## Why this lesson exists

Nearly every finance model is a monthly series with period-relative logic:
last month, same month last year, year to date, and a line where actuals
stop and the forecast starts. Excel has no idea what a period is, so every
model rebuilds that idea from column headers, and every `SUMIFS` over dates
is the idea rebuilt again.

FrameWork already refuses the worst of the spreadsheet habit: a calculation
that silently depends on row order must declare that order. But today the
only prior-period tool is a positional `.shift`, and a fiscal year that does
not start in January is arithmetic you write by hand. This lesson makes both
of those visible.

## Files

- [`driver-forecast-start.fw`](driver-forecast-start.fw) — eighteen months
  of actual revenue typed as currency, and an `Assumptions` group holding
  `Close date` and `Growth`. It also carries the two calendars section 7
  needs — `Company` as the default, `NRF 4-5-4` for the retail check —
  and `Actuals`' declared period.
- [`driver-forecast-finished.fw`](driver-forecast-finished.fw) — the answer
  key, built with today's catalog.

The company's fiscal year starts in February, so FY2026 runs February 2025
to January 2026. That is deliberate: it is the ordinary case in retail and
government, and it is where hand-written calendar arithmetic goes wrong.

## 1. Prior month on the actuals, and why it is fragile

Select `Actuals`, open **Wrangle**, and sort by Month ascending. Add two
calculated columns:

```text
Prior month = `Revenue`.shift(1)
Change = `Revenue` / `Prior month` - 1
```

Format Change as a percentage with one decimal.

Checkpoint: March 2025 shows Prior month `100,000` and Change `4.0%`.
February 2025, the first row, has no prior month.

Now do the thing this lesson is about. Right-click the November 2025 row and
delete it. Look at December 2025: its Prior month now reads `119,000`, which
is October. Nothing warned you. The column is right about position and wrong
about time. Undo the deletion before continuing.

`.shift(1)` means "the row above". A finance person means "the period
before". Those agree only while no month is missing, and a month goes
missing the first time an export is late.

## 2. A spine that cannot have gaps

The workaround is to build the calendar yourself. Add a generated table
named `Months` whose formula is:

```text
sequence(2025-02-01, 2027-02-01, 1mo)
```

with the column named `Month`. It has twenty-four rows, February 2025 to
January 2027: two complete fiscal years.

Right-click `Actuals`, mark `Month` unique, then right-click `Months` and
choose **Join another table**:

- Starting key: Month
- Lookup table: Actuals
- Lookup key: Month
- Keep: every Months row
- Result name: Forecast
- Output columns: Month, and Revenue renamed `Actual`

Checkpoint: `Forecast` has 24 rows. The last six, August 2026 onward, have
an empty Actual.

## 3. Actuals until the close date, drivers after it

Select `Forecast`, open **Wrangle**, sort by Month ascending, and add this
column with **Calculate down rows…**, or type it:

```text
Revenue = recur(`Actual`, when(`Month` <= `Close date`).then(`Actual`).otherwise(previous() * (1 + `Growth`)))
```

Format it as USD Accounting.

Checkpoint: July 2026 reads `146,000`, the last actual. August 2026 reads
`148,920`, and January 2027 reads `164,420`.

Change `Growth` in the Assumptions group to `0.03` and watch the six forecast
months move while the actuals stay put. Put it back to `0.02`.

This part is good, and it is the reason the lesson uses FrameWork rather
than a spreadsheet: the cutover is one assumption, the forecast is one
formula, and neither is a range you drag.

## 4. Fiscal year and quarter by hand

Add two calculated columns:

```text
Fiscal year = when(`Month`.dt.month() >= 2).then(`Month`.dt.year() + 1).otherwise(`Month`.dt.year())
Fiscal quarter = ((`Month`.dt.month() + 10) % 12) // 3 + 1
```

Checkpoint: January 2026 is Fiscal year `2026`, quarter `4`. February 2026
is Fiscal year `2027`, quarter `1`.

Read the quarter formula again. It is correct for a February year start. It
would be wrong for any other, it says nothing about 4-4-5 weeks or a
53-week year, and there is no way to tell from the formula which company it
belongs to.

## 5. Period-relative columns, positionally

Add, in one step:

```text
Prior month = `Revenue`.shift(1)
YTD = `Revenue`.cum_sum(False).over(`Fiscal year`)
Last year = `Revenue`.shift(12)
```

and then, in a step after it:

```text
YoY = `Revenue` / `Last year` - 1
```

Format the money columns as USD Accounting and YoY as a percentage.

Checkpoint: January 2026 YTD reads `1,475,000`, the FY2026 total. July 2026
YTD reads `767,000`. August 2026 YoY reads `16.3%`, and January 2027 YoY
reads `8.2%`.

These are right only because section 2 built a spine with no gaps and
section 3 sorted it. `shift(12)` does not know it means "a year ago"; it
means "twelve rows up". Nothing in the document records that the two are
supposed to be the same thing.

## 6. Summarise by fiscal quarter

Branch `Forecast` to a tab named `By quarter` and add **Summarize** in
Wrangle:

- Group by: Fiscal year, Fiscal quarter
- Revenue: `` `Revenue`.sum() ``

Checkpoint:

| Fiscal year | Quarter | Revenue |
|---|---|---:|
| 2026 | 1 | 314,000 |
| 2026 | 2 | 374,000 |
| 2026 | 3 | 369,000 |
| 2026 | 4 | 418,000 |
| 2027 | 1 | 351,000 |
| 2027 | 2 | 416,000 |
| 2027 | 3 | 455,755 |
| 2027 | 4 | 483,651 |

Add a block named `Checks` with:

```text
fy2026 = `Forecast`.`Revenue`.filter(`Forecast`.`Fiscal year` == 2026).sum()
fy2027 = `Forecast`.`Revenue`.filter(`Forecast`.`Fiscal year` == 2027).sum()
forecast months = `Forecast`.`Actual`.null_count()
```

Checkpoint: `1475000.00`, `1706405.3738`, `6`.

## 7. The time spine lands

The start file already carries what this section needs: a `Company`
calendar (February, calendar months, years numbered by the calendar year
they end in) set as the default, an `NRF 4-5-4` calendar for the retail
check at the end, and `Actuals`' declared period. Right-click `Actuals`
and open the frame menu: the Period column reads `Month`. That choice is
what lets the formulas below read the period before instead of the row
above. Creating a calendar is an MCP step for now — the start file carries
both — while declaring is one choice in the frame menu.

Section 1's fragile column becomes:

```text
Prior month = prior(`Revenue`)
```

`Change` stays exactly as it is. Now repeat the deletion from section 1:
delete November 2025 and look at December. Its Prior month reads empty,
not October, because the period before December is November and November
is gone. That is the test that matters. Undo the deletion before
continuing.

Select `Forecast` and declare `Month` as its period from the frame menu.
Section 4 becomes:

```text
Fiscal year = fiscal_year(`Month`)
Fiscal quarter = fiscal_quarter(`Month`)
```

No year start is written anywhere: the bare calls read the default
Company calendar. January 2026 is still Fiscal year `2026`, quarter `4`;
February 2026 is still `2027`, quarter `1`. The formula now says which
company it belongs to, because the calendar is a named object on the
document rather than arithmetic in the column.

Section 5 becomes, in one step:

```text
Prior month = prior(`Revenue`)
YTD = ytd(`Revenue`)
Last year = same_period_last_year(`Revenue`)
```

and a trailing-twelve-months column is one more word:

```text
TTM = ttm(`Revenue`)
```

Checkpoint: January 2027 TTM reads `1,706,405.37`, the FY2027 total —
twelve months ending here, whatever months are missing. Every checkpoint
in sections 3 to 6 is unchanged: January 2026 YTD still reads `1,475,000`,
August 2026 YoY still reads `16.3%`. `prior` and its siblings refuse to
run on a frame with no period declaration, naming the frame and the fix,
without refusing any other operation on it.

Finally the retail check. Add one column:

```text
Retail week = fiscal_week(`Month`, calendar="NRF 4-5-4")
```

Checkpoint: February 1 2025 is week `52` — the last week of the 53-week
fiscal 2023 — and January 1 2026 is week `48`. The NRF 4-5-4 calendar puts
every week of fiscal 2023, 2024 and 2025 where the published calendar
puts it, including the 53rd week of 2023 stretching January 2024 to five
weeks.

## Smoke-test notes

If a named control is missing, a step cannot be completed, or a checkpoint
differs, record it with the template in the parent tutorial README rather
than working around it. Expected awkwardness that is product feedback, not
a lesson error:

- creating a calendar is an MCP step for now; the start file carries both
  calendars section 7 needs, and declaring a period is one choice in the
  frame menu;
- a generated table and a join are still needed before the forecast has
  somewhere to put its months;
- a recurrence keeps the type of its first value, which is why the start
  workbook types Revenue as currency: seeded from an integer column, section
  3 would truncate every forecast month to whole dollars;
- `.cum_sum` demands its optional argument be spelled out, so
  `.cum_sum(False)` rather than `.cum_sum()`.

## Rebuilding the files

```bash
cargo run -p framework-core --example generate_finance_tutorials driver-forecast
```

The generator asserts every checkpoint above against the reloaded answer key.
The expected numbers were computed independently in Python before the
workbook existed.
