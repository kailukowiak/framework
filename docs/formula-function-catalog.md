# Native Polars formulas

FrameWork formulas are native Polars expressions with one ergonomic substitution: exact backtick references replace `pl.col(...)`. The calculated-column name supplies the final alias.

```text
`Weight` / (`Height` ** 2)
sum_horizontal([`Q1`, `Q2`, `Q3`]) * `Multiplier`
`Birthdate`.dt.year()
`Amount`.rolling_mean(window_size=30, min_periods=5)
`Amount`.sum().over(`Category`)
`Amount`.filter(`Region` == "West").sum()
`Date text`.str.to_date().dt.month()
when(`Amount` > 100).then("High").otherwise("Low")
when(`Stat`).then("Stat").when(`Weekend`).then("Reg Holiday").otherwise("Work")
`Amount`.normalize()
`Amount`.normalize(center=0)
```

The Rust parser resolves backtick references to stable object and column IDs, then compiles the syntax tree to Polars 0.55 `Expr` objects. Formula text is not Python and is never passed to `eval`. Polars owns expression typing, null behavior, broadcasting, aggregation, window semantics, and execution errors.

## Syntax

- Exact references: `` `Column` ``, `` `Canvas Value` ``, and `` `Table`.`Column` ``.
- Literals: numbers, strings, `True`, `False`, `None`, and `null`.
- Operators: `+`, `-`, `*`, `/`, `//`, `%`, `**`, comparisons, `&`, `|`, and `~`.
- Structure: normal precedence, parentheses, lists, positional arguments, keyword arguments, and method chains.
- The calculated-column name is the alias. Calling `.alias(...)` inside a formula is rejected.

## Data types and casting

Formulas cast between column types with `.cast("type")`.

**Accounting** is a Decimal128 column at a declared scale — the exact type
for a ledger, where sums must foot and nothing is silently rounded.
`.cast("accounting")` casts to scale 2; `.cast("accounting", 4)` casts to a
chosen scale. Arithmetic between an accounting amount and another value
stays exact: an integer participates as-is, a written number brings its own
place count (`0.0825` is four places, `1.5` is one), and a float column is
cast to nine places before combining. The result takes the wider of the two
scales; division takes the amount's own scale.

`Currency` (the f64 modelling type) and `Accounting` never combine on their
own — an amount and a price need an explicit cast on one side first:
`.cast("accounting")` brings money onto the ledger, `.cast("currency")`
brings an amount back to a modelling number. `.cast("currency")` is only
accepted on an accounting input.

`.mean()`, `.median()` and quantiles of an accounting column return a plain
Number; `.sum()`, `.min()` and `.max()` stay accounting. Joining an
accounting value into text prints its exact digits at its scale: `"Owed " +
Amount` reads `Owed 12.50`, not a rounded or reformatted number.

Wherever an amount is rounded to fewer places — a typed cell with too many, a cast to a smaller scale, a product or quotient landing on the wider operand's scale — the rounding is half to even, the same rule Polars applies, so a typed `0.125` and a computed copy of it both read `0.12`. A per-call rounding mode for tax or interest rules is not offered yet.

## Exposed expression surface

The catalog returned by the core and MCP is also used for autocomplete.

| Family | Current functions and methods |
| --- | --- |
| Horizontal | `sum_horizontal`, `mean_horizontal`, `min_horizontal`, `max_horizontal` |
| Financial | `pv`, `fv`, `pmt`, `ipmt`, `ppmt`, `nper`, `rate`, `npv`, `xnpv`, `irr`, `xirr`, `mirr`, `effect`, `nominal`, `sln`, `db`, `ddb`, `period_index`, `prior`, `fiscal_year`, `fiscal_quarter`, `fiscal_period`, `period_start`, `period_end`, `add_periods`, `ytd`, `ttm`, `same_period_last_year`, `fiscal_week`, `workday`, `networkdays` (uppercase Excel names also work) |
| Generators / row order | `sequence(stop)`, `sequence(start, stop, step)`, `table.len()`; `recur(first, next, restart_by=[columns])` with `previous()` inside `next` |
| Conditional/null | `when().then()` — chained as many times as you like — `.otherwise()`, `coalesce`, `.is_null`, `.is_not_null`, `.fill_null`, `.filter(predicate)` → `.sum()` / `.mean()` / `.count()` |
| Numeric | `.abs`, `.sign`, `.round`, `.round_sig_figs`, `.truncate`, `.floor`, `.ceil`, `.sqrt`, `.cbrt`, `.pow`, `.exp`, `.log`, `.log1p`, `.normalize`, `.clip`, `.clip_min`, `.clip_max`, `.floor_div` |
| Trigonometry | `.sin`, `.cos`, `.tan`, `.cot`, `.arcsin`, `.arccos`, `.arctan`, `.arctan2`, `.sinh`, `.cosh`, `.tanh`, `.arcsinh`, `.arccosh`, `.arctanh`, `.degrees`, `.radians` |
| Aggregation/window | `.sum`, `.mean`, `.quantile`, `.min`, `.max`, `.count`, `.len`, `.null_count`, `.shift`, `.over`, `.head`, `.tail` |
| Rolling | `.rolling_mean`, `.rolling_sum`, `.rolling_min`, `.rolling_max` |
| Dates | `date`, `.dt.year`, `.dt.iso_year`, `.dt.quarter`, `.dt.month`, `.dt.week`, `.dt.weekday`, `.dt.ordinal_day`, `.dt.is_leap_year`, `.dt.days_in_month`, `.dt.date`, `.dt.month_start`, `.dt.month_end`, `.dt.offset_by` |
| Strings | `.str.to_uppercase`, `.str.to_lowercase`, `.str.to_date()` (strict `YYYY-MM-DD`), `.str.contains` |

`sequence` is an advanced Scratchwork formula. Its stop is excluded:
`sequence(1, 8, 2)` produces `1, 3, 5, 7`, while
`sequence(2026-01-01, 2027-01-01, 1mo)` produces a monthly date spine. It is
a typed list rather than a row number, so it can be inspected, copied, or
folded with `.sum()` and friends without being silently matched to table rows.
Inside a sorted Wrangle chain, `sequence(1, table.len() + 1)` is the explicit
exception: it fills the current table with `1…N` in the order declared before
that calculated-column step. FrameWork refuses it when no sort says what row
position means.

`recur` is the explicitly sequential exception to the otherwise vectorized
column engine. Inside a sorted Wrangle chain,
``recur(`Opening`, previous() + `Change`, restart_by=[`Account`])`` evaluates
top to bottom and carries a separate previous result for each account. It is
normally authored through the visual *Calculate down rows* step rather than
typed as a wrapper. `previous()` is rejected outside that context, and the
first-row expression cannot use it.

The seed and next-row expression determine a common recurrence type before
evaluation. An integer seed can therefore grow fractional results without
rounding each step; integer-only recurrences remain integers.

## Financial functions

Use `finance.pmt(rate, nper, pv)` or the receiver form
`principal.finance.pmt(rate, nper)`. All financial functions support both forms;
the original unqualified calls remain compatible. Names are case-insensitive.

| Receiver call | Receiver supplies |
| --- | --- |
| `payment.finance.pv(rate, nper, fv=0, type=0)` | payment |
| `principal.finance.fv(rate, nper, pmt, type=0)` | present value |
| `principal.finance.pmt(rate, nper, fv=0, type=0)` | present value |
| `principal.finance.ipmt(rate, per, nper, fv=0, type=0)` | present value |
| `principal.finance.ppmt(rate, per, nper, fv=0, type=0)` | present value |
| `principal.finance.nper(rate, pmt, fv=0, type=0)` | present value |
| `flows.finance.npv(rate)` | cash flows |
| `flows.finance.xnpv(rate, dates)` | cash flows |
| `flows.finance.irr(guess=0.1)` | cash flows |
| `flows.finance.xirr(dates, guess=0.1)` | cash flows |
| `flows.finance.mirr(finance_rate, reinvest_rate)` | cash flows |
| `nominal_rate.finance.effect(npery)` | nominal rate |
| `effect_rate.finance.nominal(npery)` | effective rate |
| `cost.finance.sln(salvage, life)` | initial cost |
| `cost.finance.db(salvage, life, period, month=12)` | initial cost |
| `cost.finance.ddb(salvage, life, period, factor=2)` | initial cost |
| `principal.finance.rate(nper, pmt, fv=0, type=0, guess=0.1)` | present value |
| `date.finance.period_index(fy_start=None, calendar=None)` | date |
| `value.finance.prior(n=1, fy_start=None, calendar=None)` | value |
| `date.finance.fiscal_year(fy_start=None, calendar=None)` | date |
| `date.finance.fiscal_quarter(fy_start=None, calendar=None)` | date |
| `date.finance.fiscal_period(fy_start=None, calendar=None)` | date |
| `date.finance.period_start()` | date |
| `date.finance.period_end()` | date |
| `date.finance.add_periods(n)` | date |
| `value.finance.ytd(fy_start=None, calendar=None)` | value |
| `value.finance.ttm(calendar=None)` | value |
| `value.finance.same_period_last_year(calendar=None)` | value |
| `date.finance.fiscal_week(calendar=None)` | date |
| `date.finance.workday(n, calendar=None)` | date |
| `start_date.finance.networkdays(end_date, calendar=None)` | start date |

Use parentheses around a negative receiver: `(-principal).finance.pmt(rate, nper)`.
The namespace form keeps the following original argument order.

`pv(rate, nper, pmt, fv=0, type=0)`, `fv(rate, nper, pmt, pv=0, type=0)`,
`pmt(rate, nper, pv, fv=0, type=0)` and `nper(rate, pmt, pv, fv=0, type=0)`
use Excel's argument order and cash-flow signs. Rate is per period, outflows
are negative, and type is 0 for end-of-period payments or 1 for beginning.
`ipmt(rate, per, nper, pv, fv=0, type=0)` and `ppmt` split a payment into
interest and principal; per must be an integer from 1 through nper.
Rates must exceed -1 and known nper must be positive. Zero rates are supported;
undefined or non-finite results error. Missing inputs propagate as blank.

`npv(rate, values)` accepts one cash-flow column or list in supplied order.
Its first flow is one period away, matching Excel; add time-zero flows separately.
`xnpv(rate, values, dates)` uses actual days / 365 from the first supplied date.
Dates must be Date values, none earlier than the first. Both functions reject
empty or missing flows and require a finite scalar rate greater than -1;
XNPV also checks matching lengths. Neither requires freezing a live frame.
For an imported text date column, pass `dates.str.to_date()` explicitly.
Unlike Excel's variadic NPV, pass all flows as one column or list; missing
flows error rather than being silently skipped.

`irr(values, guess=0.1)` returns a rate per supplied period, with the first
flow at time zero. `xirr(values, dates, guess=0.1)` returns an annual rate on
an actual/365 basis, using the same first-date convention as XNPV. Both accept
a column or list and require finite, nonmissing flows with both signs; the
guess must be a finite scalar greater than -1. XIRR requires matching Date
values, with none earlier than the first. Live and derived columns are supported.

Return solving searches `ln(1 + rate)` from -18 through 18 in 4,096 intervals,
also sampling zero and the guess when inside that range. Sign-change brackets
are refined by bisection; an exact sampled zero is accepted only when nearby
probes distinguish it from a flat zero curve. This is a bounded search, not
an exhaustive polynomial solver: closely spaced roots and unsampled tangent
roots can be missed, and rates outside the domain are not searched. `rate`
reuses the same bounded search per row over the annuity equation, so a loan
table can solve its own periodic rate beside its payments.

When several roots are discovered, the nearest to the guess in log-rate space
is selected, with the lower rate winning an exact tie. The default guess is
0.1. The answer does **not** establish uniqueness; inspect an NPV rate scan
when flows change sign repeatedly. Indeterminate curves, failure to find a
root, and failure to converge are errors. Convergence details are internal
for now; the tutorial checks the returned rate with a visible XNPV residual.

`mirr(values, finance_rate, reinvest_rate)` discounts negatives at the
finance rate and compounds positives at the reinvestment rate, then annualizes
over the flow count. Flows must contain both signs; both rates are finite
scalars greater than -1.

`effect(nominal_rate, npery)` and `nominal(effect_rate, npery)` convert
between nominal and effective annual rates. Npery is truncated to an integer
as in Excel; the rate must be positive and npery at least 1.

`sln(cost, salvage, life)` is straight-line depreciation per period.
`db(cost, salvage, life, period, month=12)` is fixed-declining-balance with
the rate rounded to three decimals and first/last periods prorated by month,
matching Excel including the extra period when the first year is partial.
`ddb(cost, salvage, life, period, factor=2)` caps each period so book value
never drops below salvage. All three broadcast down a Period column, so a
schedule frame reconciles: each column sums to cost minus salvage.

`period_index(date, fy_start=None, calendar=None)` numbers a date's fiscal
period from year zero, twelve to a year: fiscal months counted from the
year start, or retail blocks read from a week-pattern calendar's week
table. Offsets between two indexes are whole periods with no date math.
`prior(expr, n=1, fy_start=None, calendar=None)` reads what `expr` held `n`
periods before each row's own period, joined on the frame's declared period
column within its partitions
— the period before, never the row above. Declaring the column is a frame
property (`SetFramePeriod`): the column must hold dates with no missing
values, unique within each partition. A frame without one gets an error
naming the frame and the fix when a step using `prior` is saved; `prior`
anywhere else (Scratchwork, a filter, a summary) is refused the same way.
A missing earlier period — the first row, or a deleted month — reads blank
rather than failing, which is what separates "no such period" from "the row
above". `n` and `fy_start` must be whole numbers written in the formula or
held by a named value.

Every period-relative call — `period_index`, `prior`, `ytd`, `ttm`,
`same_period_last_year` — takes the same optional `calendar`, and a call
naming none reads the document default. That default is what supplies the
year start when no `fy_start` is written, so a February-start workbook
counts its periods from February everywhere rather than only where
`fy_start=2` was remembered, and a retail workbook's windows count the same
blocks its `fiscal_period` reports.

`fiscal_year(date, fy_start=None, calendar=None)` numbers a date's fiscal
year by the calendar year it ends in — the way company accounts name
theirs — so with
a February start, January 2025 is fiscal 2025 and February opens fiscal
2026. A calendar can instead number years by the year they start in, which
is how the NRF labels its retail calendars. `fiscal_quarter(date,
fy_start=None, calendar=None)` and `fiscal_period(date, fy_start=None,
calendar=None)` count three-month quarters and months from `fy_start`
without any labelling question. `period_start(date, calendar=None)` and
`period_end(date, calendar=None)` bound the calendar month holding a date —
February 2024 ends on the 29th.
`add_periods(date, n)` shifts by `n` calendar months with end-of-month
clamping, so January 31 plus one month is February 28; this is Excel's EDATE
arithmetic exactly, and `EDATE` works as an alias. The count may be a column;
a missing count reads blank. These read the date itself, so they need no
period declaration and work in Scratchwork; like `period_index`, `fy_start`
must be a whole month number from 1 to 12 written in the formula or held by
a named value.

`ytd(expr, fy_start=None, calendar=None)` sums `expr` over the fiscal year
so far, `ttm(expr, calendar=None)` sums the twelve periods ending here, and
`same_period_last_year(expr, calendar=None)` reads the value twelve periods
ago — all joined on the frame's declared period column within its
partitions, never shifted by row position. `ytd` resets when the fiscal
year turns in its calendar; twelve indexes back is the same date last year
under any year start, so `ttm` and `same_period_last_year` take no year
start — but which twelve periods exist is a calendar question, so all three
take a `calendar`. Windows sum the periods present — a deleted month
removes its value from every window holding it rather than failing — and
only a window with no readable value at all reads blank. Like `prior`, all
three need the declaration (a frame without one gets an error naming the
frame and the fix when the column is saved), work in calculated columns on
both authoring surfaces, and are refused in Scratchwork, filters and
summaries.

A fiscal calendar names the month its year starts on, the week pattern
its twelve periods follow (calendar months, or 4-4-5, 4-5-4, 5-4-4 blocks
per quarter), the rule ending its year, which year number the year carries,
and the days the business counts — weekend days plus dated holidays.
Calendars live on the document with one default; the fiscal functions take
an optional `calendar` naming one per call, and a call that names none
reads the default. An explicit `fy_start` wins over a calendar-month
calendar's year start, and a missing calendar is refused naming what is
available. A retail week calendar refuses `fy_start` outright rather than
parsing and dropping it: its year opens the day after the previous year's
end rule fires, so there is no month boundary for a month number to move. `fiscal_year`,
`fiscal_quarter`, `fiscal_period`, `period_start` and `period_end` read a
week-pattern calendar through its week table: quarters stay thirteen
weeks, a 53rd week extends the last period, and period bounds are block
bounds. `fiscal_week(date)` numbers the date's week from 1, in seven-day
blocks from the year's start date — a year starting on a Sunday has
Sunday-to-Saturday weeks, matching the published NRF calendar for every
week of fiscal 2023, 2024 and 2025 including the 53rd week of 2023.
`workday(date, n)` shifts by business days skipping the calendar's weekend
and holidays, the start date uncounted; `networkdays(start, end)` counts
the business days between inclusively, negated when the end precedes the
start. Both take the date itself, need no declaration, and read blank on
missing inputs.

The loan functions compile to composed Polars expressions. Discounted totals
use a native aggregate over the evaluated series so length, date and missing
value checks happen before any aggregation can conceal them. Reference tests
use [NumPy Financial's examples](https://numpy.org/numpy-financial/latest/)
and [Microsoft's XNPV example](https://support.microsoft.com/en-us/excel/functions/xnpv-function).
Return tests also use [NumPy Financial's IRR examples](https://numpy.org/numpy-financial/latest/irr.html)
and [Microsoft's XIRR example](https://support.microsoft.com/en-us/excel/functions/xirr-function).

Unsupported methods fail visibly instead of falling back to another evaluator. The catalog is a discoverability surface, not a separate formula language: names, arguments, and behavior are intended to track Polars.

## Generated expression surface

The table above is the hand-written core. Beyond it, [formula-function-catalog.generated.md](formula-function-catalog.generated.md) lists a much wider, code-generated surface covering most of the remaining Polars 0.55.2 `Expr` methods and namespaces (root functions, and the `str`/`dt`/`list`/`arr`/`struct`/`cat` namespaces). It is produced by `tools/generate_expr_bindings.py` from the vendored `polars-plan` source and compiled by `crates/framework-core/src/generated_expr_bindings.rs`; both files carry regeneration instructions in their headers. Methods that take closures/UDFs, IO/serialization/meta/plugin methods, and `alias` are excluded by rule; anything else the generator can't bind with certainty (an options-struct or enum argument, for example) is left out and recorded with a reason in `tools/expr_bindings_spec.json` rather than silently misbound.

## Variable controls

Named Scratchwork lines and compact variables can declare an input control in their formula:

```text
growth = slider(start=0, stop=0.2, step=0.01, value=0.05)
region = dropdown(["North", "South"], value="North")
cutoff = date_input(value=date(2026, 12, 31))
```

Each constructor returns the selected scalar, so other formulas and semantic filters reference the variable normally. The UI edits only its `value` argument through ordinary history; there is no separate widget state in the document. Sliders commit on release (or after a keyboard adjustment), and exact numeric/date fields commit on blur or Enter. One slider gesture is one undoable edit.

Bounds, step, options and initial selection must be literals. Slider bounds are inclusive; step must be positive. The default selection is `start`, and the exact numeric field allows in-range values between steps. Dropdown options must be distinct, nonempty and all one scalar type; its default is the first option. Date inputs require a literal date. Invalid definitions and out-of-domain edits report errors, not silent clamping. Frozen variables do not expose an editable control.

## Safety boundary

FrameWork constructs typed Polars expressions from a parsed AST. It does not expose imports, attribute access outside known expression namespaces, filesystem or network I/O, Python callbacks, UDFs, plugins, NumPy execution, or arbitrary code evaluation. Expensive operations can later receive cost limits at the compiler/executor boundary without changing saved formula syntax.

Whole-table and shape-changing operations—filtering table rows, sorting, joins, group-by output, pivots, and reshaping—belong to derived-table `LazyFrame` plans rather than calculated columns. `.filter(predicate)` is narrower: it filters one expression so an aggregate can answer a conditional total, average, count, minimum, or maximum without changing the table itself. Nested list/array/struct dtypes and additional Polars namespaces can be added directly to the compiler as FrameWork gains display and editing support for those result types.
