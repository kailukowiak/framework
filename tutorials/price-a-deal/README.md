# Price a deal

About 20 minutes. Open **Price a deal — Start** from **Data library →
Tutorials and examples**. The finished workbook is a separate answer key.

This lesson uses the first financial function pack: loan payments,
interest and principal, and dated net present value. IRR solving remains
the next increment of [the finance build plan](../../docs/finance-build-plan.md).

## Files

- [Start](price-a-deal-start.fw) — dated cash flows and loan assumptions.
- [Finished](price-a-deal-finished.fw) — the working schedule and valuation.

## 1. Calculate the loan payment

The `Loan terms` block starts with:

```text
principal = 400000
rate = 0.06
term = 60
```

Add:

```text
monthly = rate / 12
payment = (-principal).finance.pmt(monthly, term)
```

Checkpoint: monthly is `0.005`; payment is `7733.1206`.
The functions follow Excel's argument order and signs: money paid out
is negative. Passing a negative principal makes the payment positive here.
The rate is per payment period. Optional `fv=0` sets the remaining balance;
`type=0` pays at period end and `type=1` at period start.

## 2. Build the amortisation schedule

Add a generated table named `Schedule`, formula `sequence(1, 61)`,
column `Period`. In **Wrangle**, sort by Period ascending, then add
each calculated column as its own step:

```text
Opening = recur(`Loan terms`.principal, previous() * (1 + `Loan terms`.monthly) - `Loan terms`.payment)
Interest = (-`Loan terms`.principal).finance.ipmt(`Loan terms`.monthly, `Period`, `Loan terms`.term)
Principal paid = (-`Loan terms`.principal).finance.ppmt(`Loan terms`.monthly, `Period`, `Loan terms`.term)
Closing = `Opening` - `Principal paid`
```

The recurrence carries the balance forward in the declared order. Its integer
seed now promotes to retain the fractional amounts in later rows.
Format the four money columns as USD Accounting.

Checkpoint: period 1 shows Opening `400,000`, Interest `2,000`,
Principal paid `5,733`, Closing `394,267`. Period 60 closes at `0`.

Add to `Loan terms`:

```text
total interest = `Schedule`.`Interest`.sum()
```

Checkpoint: `63987.2367`. Change the rate and watch the schedule
recompute; undo restores it.

## 3. Discount the deal's cash flows

Add a block named `Deal`:

```text
rate = 0.10
npv = `Cash flows`.`Amount`.finance.xnpv(rate, `Cash flows`.`Date`)
```

Checkpoint: `41581.08`. No helper columns are needed.
XNPV uses actual days divided by 365, with the first supplied date as time
zero. Later rows can be out of order, but no date may precede that first date.
Amounts and dates must match in length and contain no missing values.

For equally spaced flows, `npv(rate, values)` discounts the first flow by
one period, as Excel does. Add the initial investment separately if it occurs
today. Both functions require a finite scalar rate greater than -1.

## 4. Bracket the return with a rate scan

Add a `Rate scan` block:

```text
`NPV at 5%` = `Cash flows`.`Amount`.finance.xnpv(0.05, `Cash flows`.`Date`)
`NPV at 10%` = `Cash flows`.`Amount`.finance.xnpv(0.10, `Cash flows`.`Date`)
`NPV at 15%` = `Cash flows`.`Amount`.finance.xnpv(0.15, `Cash flows`.`Date`)
`NPV at 20%` = `Cash flows`.`Amount`.finance.xnpv(0.20, `Cash flows`.`Date`)
`NPV at 25%` = `Cash flows`.`Amount`.finance.xnpv(0.25, `Cash flows`.`Date`)
`NPV at 30%` = `Cash flows`.`Amount`.finance.xnpv(0.30, `Cash flows`.`Date`)
```

Checkpoint: `64121.94`, `41581.08`, `21809.34`, `4355.85`,
`-11141.06`, `-24974.32`. The return lies between 20% and 25%.

## 5. Next: solve the return

XIRR is not implemented yet. Phase 1b replaces the scan with:

```text
irr = xirr(`Cash flows`.`Amount`, `Cash flows`.`Date`)
```

Its acceptance target is approximately `21.35%`, with a convergence report
and a clear failure reason when no root can be found.

## Rebuilding and checking

```bash
cargo run -p framework-core --example generate_finance_tutorials price-a-deal
```

The generator writes both workbooks through the normal operations and checks
the reloaded answer key against independent numerical checkpoints.
Report a missing control or a different result using the parent tutorial's
smoke-test template.
