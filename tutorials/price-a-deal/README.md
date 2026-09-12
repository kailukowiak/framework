# Price a deal

About 20 minutes. Open **Price a deal — Start** from **Data library →
Tutorials and examples**, or open the linked `.fw` files with **File → Open**.
The answer key is a separate workbook.

This is the first of three finance lessons. Each one builds a real job with
what FrameWork has today, shows exactly where that hurts, and then shows what
the same job becomes when the matching phase of
[the finance build plan](../../docs/finance-build-plan.md) lands. The
"today" half is a working lesson with checkpoints. The "after" half is the
acceptance target for that phase: when its formulas compute and its
checkpoints match, the phase is done.

## Why this lesson exists

The first question a finance person asks of any tool is whether it has
`XIRR`. Today FrameWork's answer is no: there is no `pmt`, `npv`, `irr` or
`xirr` in the catalog. Everything below can still be built, because the
engine has recurrence, aggregates and date arithmetic. But a loan payment is
a formula you have to know by heart, and an internal rate of return is a
number you find by guessing rates until the sign flips. That is the
Excel-in-1995 experience, and it is the reason phase 1 of the plan is the
function pack.

## Files

- [`price-a-deal-start.fw`](price-a-deal-start.fw) — the deal's cash flows
  and a `Loan terms` block holding principal, rate and term.
- [`price-a-deal-finished.fw`](price-a-deal-finished.fw) — the answer key,
  built with today's catalog.

## 1. Work out the loan payment by hand

The `Loan terms` block already holds the inputs, and one comment:

```text
principal = 400000.0
# 400000.0 rather than 400000: a recurrence keeps the type of its first value, and the schedule needs cents
rate = 0.06
term = 60
```

Add two lines below them:

```text
monthly = rate / 12
payment = principal * monthly / (1 - (1 + monthly) ** -term)
```

Checkpoint: `monthly` reads `0.005` and `payment` reads `7733.1206`.

That second line is `PMT` written out. It is correct, and you had to know
it. Nobody should have to know it.

## 2. Build the amortisation schedule with a recurrence

Add a generated table named `Schedule` whose formula is `sequence(1, 61)`
and whose column is `Period`. It has sixty rows, one per month.

Select `Schedule`, open **Wrangle**, and sort by Period ascending. Then add
these calculated columns, one step at a time, because each reads the one
before it:

```text
Opening = recur(`Loan terms`.principal, previous() * (1 + `Loan terms`.monthly) - `Loan terms`.payment)
Interest = `Opening` * `Loan terms`.monthly
Principal paid = `Loan terms`.payment - `Interest`
Closing = `Opening` - `Principal paid`
```

The first one is what **Calculate down rows…** writes for you: the opening
balance of each month is last month's opening balance grown by one month of
interest, less the payment. Format the four money columns as USD Accounting.

Checkpoint: period 1 shows Opening `400,000`, Interest `2,000`, Principal
paid `5,733`, Closing `394,267`. Period 60 closes at `0`.

Back in `Loan terms`, add:

```text
total interest = `Schedule`.`Interest`.sum()
```

Checkpoint: `total interest` reads `63987.2367`.

This works, and it is a fair amount of typing for something Excel does with
`IPMT` and `PPMT`. Note what did work: the recurrence is explicit about its
order, so the schedule cannot silently depend on how rows happen to be
sorted.

## 3. Discount the deal's cash flows

`Cash flows` holds six dated amounts: one outflow and five inflows on
irregular dates. Add a block named `Deal` with one line:

```text
rate = 0.10
```

Select `Cash flows`, open **Wrangle**, and add three calculated columns:

```text
Years = (`Date` - `Date`.min()).dt.total_days(False) / 365
Discount factor = 1 / (1 + `Deal`.rate) ** `Years`
Present value = `Amount` * `Discount factor`
```

Then add to `Deal`:

```text
npv = `Cash flows`.`Present value`.sum()
```

Checkpoint: `Years` runs `0`, `0.455`, `0.959`, `1.455`, `1.959`, `2.458`,
and `npv` reads `$41581.08`.

That is `XNPV` built from parts. Three columns and a line to answer one
question, and the day-count convention is a literal `365` you have to
remember to write.

## 4. Find the IRR by scanning rates

There is no solver, so do what a spreadsheet without one does: try rates.
Add a block named `Rate scan` and type the same formula six times:

```text
`NPV at 5%` = (`Cash flows`.`Amount` / (1 + 0.05) ** `Cash flows`.`Years`).sum()
`NPV at 10%` = (`Cash flows`.`Amount` / (1 + 0.10) ** `Cash flows`.`Years`).sum()
`NPV at 15%` = (`Cash flows`.`Amount` / (1 + 0.15) ** `Cash flows`.`Years`).sum()
`NPV at 20%` = (`Cash flows`.`Amount` / (1 + 0.20) ** `Cash flows`.`Years`).sum()
`NPV at 25%` = (`Cash flows`.`Amount` / (1 + 0.25) ** `Cash flows`.`Years`).sum()
`NPV at 30%` = (`Cash flows`.`Amount` / (1 + 0.30) ** `Cash flows`.`Years`).sum()
```

Checkpoint: `$64121.94`, `$41581.08`, `$21809.34`, `$4355.85`, `$-11141.06`,
`$-24974.32`. The sign changes between 20% and 25%, so the IRR is somewhere
near 21%.

To get more digits you add more lines and look again. That is the whole
demonstration: the answer exists, and the tool cannot find it for you.

A Calculation Matrix with the rates on one axis would be the tidier way to
write this, and it does not work here: a matrix body cannot read a table
that has calculated columns until that table is materialised, while a block
line can. Six copies of one formula is what you are left with, which is also
what Excel leaves you with.

## 5. What changes when the function pack lands

None of the following computes today. It is the acceptance target for
phase 1 of the finance plan, and the numbers are the checkpoints.

Section 1 becomes one line:

```text
payment = pmt(rate / 12, term, -principal)
```

Section 2's interest and principal columns become the closed forms, with the
recurrence kept only for the balance:

```text
Interest = ipmt(`Loan terms`.monthly, `Period`, `Loan terms`.term, -`Loan terms`.principal)
Principal paid = ppmt(`Loan terms`.monthly, `Period`, `Loan terms`.term, -`Loan terms`.principal)
```

Section 3 loses its three helper columns:

```text
npv = xnpv(rate, `Cash flows`.`Amount`, `Cash flows`.`Date`)
```

Section 4 goes away entirely:

```text
irr = xirr(`Cash flows`.`Amount`, `Cash flows`.`Date`)
```

Expected: `payment` `7733.12`, `total interest` `63987.24`, `npv` `41581.08`,
`irr` `21.35%`. The solver must report how it converged, and must refuse
with a reason on a series with no sign change rather than return a number.

Typing `PMT` or `XIRR` must find the lower-case function through the
catalog's Excel aliases, the way `sumif` already finds `.filter(...).sum()`.

## Smoke-test notes

If a named control is missing, a step cannot be completed, or a checkpoint
differs, record it with the template in the parent tutorial README rather
than working around it. Four things are expected to be awkward today and are
product feedback, not lesson errors:

- a recurrence keeps the type of its first value, so `principal = 400000`
  would silently truncate every balance in the schedule to whole dollars;
- a Calculation Matrix body cannot read a table with calculated columns
  until that table is materialised, while a block line can, which is why
  section 4 is a block;
- `.dt.total_days` and `.cum_sum` demand their optional argument be spelled
  out, so `.dt.total_days(False)` rather than `.dt.total_days()`;
- the day count in section 3 is a literal `365` you type rather than a
  convention you choose.

## Rebuilding the files

Both workbooks are generated through `Store::apply(Operation::...)`, the same
validation, history and persistence boundary the desktop and MCP use:

```bash
cargo run -p framework-core --example generate_finance_tutorials price-a-deal
```

The generator asserts every checkpoint above against the reloaded answer key.
The expected numbers were computed independently in Python before the
workbook existed, so a passing generator is evidence rather than a tautology.
