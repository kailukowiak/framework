# /// script
# requires-python = ">=3.11"
# dependencies = ["polars>=1.0", "numpy>=1.26"]
# ///
"""Generate the Cascadia Outfitters Ltd. demo accounting dataset.

A seeded, reproducible ~1.2M-line general ledger for fiscal 2024-2025 plus the
supporting tables (chart of accounts, budget, bank statement, AR invoices)
needed to demo reconciliation, budget-vs-actuals, and aging workflows.

Deliberately planted findings are listed by the script at the end of a run and
documented in the generated README.md.

Usage: uv run examples/generate_accounting_demo.py [--out demo-data]
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
import polars as pl

RNG = np.random.default_rng(42)

PERIODS = [f"{y}-{m:02d}" for y in (2024, 2025) for m in range(1, 13)]
MONTH_START = np.array([np.datetime64(p, "D") for p in PERIODS])
MONTH_DAYS = np.array(
    [int(((np.datetime64(p, "M") + np.timedelta64(1, "M")).astype("datetime64[D]")
          - np.datetime64(p, "D")) / np.timedelta64(1, "D")) for p in PERIODS]
)
# Outdoor retail seasonality: summer peak, December bump.
SALES_WEIGHTS = np.array([0.55, 0.6, 0.8, 1.0, 1.3, 1.5, 1.5, 1.4, 1.0, 0.8, 0.9, 1.4] * 2, dtype=float)
SALES_WEIGHTS[12:] *= 1.08  # modest growth in FY2025
SALES_WEIGHTS /= SALES_WEIGHTS.sum()

DEPTS = ["SALES", "RENT", "WHSE", "MKTG", "ADMIN", "FIN", "IT", "OPS"]

# (code, name, type, subtype) — normal balance implied by type.
ACCOUNTS = [
    ("1010", "Cash - Operating (CAD)", "Asset", "Cash"),
    ("1020", "Cash - Savings", "Asset", "Cash"),
    ("1100", "Accounts Receivable", "Asset", "Receivable"),
    ("1150", "GST/HST Recoverable", "Asset", "Tax"),
    ("1200", "Inventory", "Asset", "Inventory"),
    ("1300", "Prepaid Insurance", "Asset", "Prepaid"),
    ("1510", "Warehouse Equipment", "Asset", "Fixed asset"),
    ("1511", "Accum. Dep. - Warehouse Equipment", "Asset", "Contra"),
    ("1520", "Vehicles", "Asset", "Fixed asset"),
    ("1521", "Accum. Dep. - Vehicles", "Asset", "Contra"),
    ("1530", "Rental Fleet Gear", "Asset", "Fixed asset"),
    ("1531", "Accum. Dep. - Rental Fleet", "Asset", "Contra"),
    ("1540", "Computer Hardware", "Asset", "Fixed asset"),
    ("1541", "Accum. Dep. - Computer Hardware", "Asset", "Contra"),
    ("2000", "Accounts Payable", "Liability", "Payable"),
    ("2100", "Accrued Liabilities", "Liability", "Accrual"),
    ("2300", "GST/HST Payable", "Liability", "Tax"),
    ("2400", "Payroll Withholdings Payable", "Liability", "Payroll"),
    ("2500", "Term Loan", "Liability", "Debt"),
    ("3000", "Common Shares", "Equity", "Capital"),
    ("3900", "Retained Earnings", "Equity", "Retained"),
    ("4010", "Equipment Sales", "Revenue", "Sales"),
    ("4020", "Apparel Sales", "Revenue", "Sales"),
    ("4030", "Rental Revenue", "Revenue", "Rentals"),
    ("4040", "Service & Repairs", "Revenue", "Service"),
    ("5010", "Cost of Goods Sold", "Expense", "COGS"),
    ("6010", "Salaries & Wages", "Expense", "Payroll"),
    ("6020", "Employer Payroll Taxes", "Expense", "Payroll"),
    ("6100", "Rent", "Expense", "Facilities"),
    ("6110", "Utilities", "Expense", "Facilities"),
    ("6200", "Marketing & Advertising", "Expense", "Marketing"),
    ("6300", "Insurance", "Expense", "G&A"),
    ("6400", "Software & IT Services", "Expense", "IT"),
    ("6500", "Travel", "Expense", "G&A"),
    ("6510", "Meals & Entertainment", "Expense", "G&A"),
    ("6600", "Professional Fees", "Expense", "G&A"),
    ("6610", "Office Supplies", "Expense", "G&A"),
    ("6620", "Freight & Delivery", "Expense", "Logistics"),
    ("6700", "Repairs & Maintenance", "Expense", "Facilities"),
    ("6800", "Bank Charges", "Expense", "Finance"),
    ("6810", "Interest Expense", "Expense", "Finance"),
    ("6900", "Depreciation Expense", "Expense", "Depreciation"),
    ("7010", "Interest Income", "Revenue", "Other income"),
]
ACCOUNT_NAME = {a[0]: a[1] for a in ACCOUNTS}

FIRST = ["Summit", "Alpine", "Cascade", "Glacier", "Ridge", "Trailhead", "Boreal", "Chinook", "Tamarack", "Kootenay",
         "Foothills", "Larch", "Prairie", "Basecamp", "Northface", "Wapta", "Bow Valley", "Ptarmigan", "Cirrus", "Stony Creek"]
SECOND_CUST = ["Outfitters", "Adventures", "Expeditions", "Sports", "Tours", "Guides", "Lodge", "Rentals", "Collective", "Co-op"]
SECOND_VEND = ["Supply Co", "Distributing", "Industries", "Logistics", "Wholesale", "Manufacturing", "Imports", "Textiles", "Freight", "Services"]


def _names(seconds: list[str], n: int) -> np.ndarray:
    combos = np.array([f"{f} {s}" for f in FIRST for s in seconds])
    return RNG.choice(combos, size=n, replace=True)


def _month_choice(n: int, weights: np.ndarray) -> np.ndarray:
    return RNG.choice(len(PERIODS), size=n, p=weights / weights.sum())


def _dates_in_month(midx: np.ndarray) -> np.ndarray:
    day = (RNG.random(midx.size) * MONTH_DAYS[midx]).astype(int)
    return MONTH_START[midx] + day


def _lognormal_cents(n: int, mean: float, sigma: float, lo: float, hi: float) -> np.ndarray:
    dollars = np.clip(RNG.lognormal(mean, sigma, n), lo, hi)
    return np.round(dollars * 100).astype(np.int64)


class Ledger:
    """Accumulates journal lines as parallel lists of numpy arrays."""

    COLS = ["source_ref", "posted_date", "period", "account_code", "department",
            "memo", "debit_cents", "credit_cents", "source", "created_by"]

    def __init__(self) -> None:
        self.parts: list[dict[str, np.ndarray]] = []

    def add(self, source_ref, posted_date, period, account_code, department,
            memo, debit_cents, credit_cents, source, created_by) -> None:
        n = len(source_ref)
        part = {
            "source_ref": np.asarray(source_ref, dtype=object),
            "posted_date": np.asarray(posted_date, dtype="datetime64[D]"),
            "period": np.asarray(period, dtype=object),
            "account_code": np.asarray(account_code, dtype=object),
            "department": np.asarray(department, dtype=object),
            "memo": np.asarray(memo, dtype=object),
            "debit_cents": np.asarray(debit_cents, dtype=np.int64),
            "credit_cents": np.asarray(credit_cents, dtype=np.int64),
            "source": np.full(n, source, dtype=object) if isinstance(source, str) else np.asarray(source, dtype=object),
            "created_by": np.full(n, created_by, dtype=object) if isinstance(created_by, str) else np.asarray(created_by, dtype=object),
        }
        assert all(len(part[c]) == n for c in self.COLS)
        self.parts.append(part)

    def frame(self) -> pl.DataFrame:
        data = {c: np.concatenate([p[c] for p in self.parts]) for c in self.COLS}
        return pl.DataFrame({
            **{c: data[c].tolist() for c in self.COLS if c not in ("posted_date", "debit_cents", "credit_cents")},
            "posted_date": data["posted_date"],
            "debit_cents": data["debit_cents"],
            "credit_cents": data["credit_cents"],
        })


def build_sales(led: Ledger, n_inv: int):
    """AR invoices: DR 1100 gross / CR revenue net / CR 2300 GST, plus perpetual COGS."""
    midx = _month_choice(n_inv, SALES_WEIGHTS)
    dates = _dates_in_month(midx)
    period = np.array(PERIODS, dtype=object)[midx]
    net = _lognormal_cents(n_inv, 7.1, 0.85, 40, 30000)
    gst = np.round(net * 0.05).astype(np.int64)
    gross = net + gst
    inv_no = np.array([f"INV-{100000 + i}" for i in range(n_inv)], dtype=object)
    cust = _names(SECOND_CUST, n_inv)
    rev_acct = RNG.choice(["4010", "4020", "4030", "4040"], size=n_inv, p=[0.52, 0.26, 0.13, 0.09])
    rev_dept = np.where(rev_acct == "4030", "RENT", "SALES")
    memo = np.array([f"Invoice {i} - {c}" for i, c in zip(inv_no, cust)], dtype=object)
    zeros = np.zeros(n_inv, dtype=np.int64)
    blank = np.full(n_inv, None, dtype=object)

    led.add(inv_no, dates, period, np.full(n_inv, "1100", object), blank, memo, gross, zeros, "AR", "system.ar")
    led.add(inv_no, dates, period, rev_acct, rev_dept, memo, zeros, net, "AR", "system.ar")
    led.add(inv_no, dates, period, np.full(n_inv, "2300", object), blank, memo, zeros, gst, "AR", "system.ar")

    cogs = np.round(net * RNG.uniform(0.38, 0.58, n_inv)).astype(np.int64)
    cogs_memo = np.array([f"COGS {i}" for i in inv_no], dtype=object)
    led.add(inv_no, dates, period, np.full(n_inv, "5010", object), rev_dept, cogs_memo, cogs, zeros, "AR", "system.ar")
    led.add(inv_no, dates, period, np.full(n_inv, "1200", object), blank, cogs_memo, zeros, cogs, "AR", "system.ar")

    return {"inv_no": inv_no, "date": dates, "customer": cust, "net": net, "gst": gst, "gross": gross}


def build_receipts(led: Ledger, inv):
    """Customer payments grouped into deposit batches (one-to-many rec story)."""
    n = inv["inv_no"].size
    lag = np.round(RNG.gamma(2.2, 11, n) + 3).astype(int)
    pay_date = inv["date"] + lag
    end = np.datetime64("2025-12-31")
    paid = pay_date <= end  # invoices "paid" after year end stay open -> AR aging
    idx = np.where(paid)[0]
    idx = idx[np.argsort(pay_date[idx].astype(int), kind="stable")]

    batch_sizes = RNG.integers(1, 9, size=idx.size)  # 1..8 invoices per deposit
    cuts = np.cumsum(batch_sizes)
    cuts = cuts[cuts < idx.size]
    batch_id_per_inv = np.zeros(idx.size, dtype=int)
    batch_id_per_inv[cuts] = 1
    batch_id_per_inv = np.cumsum(batch_id_per_inv)
    n_batches = batch_id_per_inv[-1] + 1
    dep_no = np.array([f"DEP-{200000 + b}" for b in range(n_batches)], dtype=object)

    inv_dep = dep_no[batch_id_per_inv]
    # deposit date = latest payment date in the batch
    dep_date = np.zeros(n_batches, dtype="datetime64[D]")
    np.maximum.at(dep_date.view(np.int64), batch_id_per_inv, pay_date[idx].view(np.int64))
    inv_dep_date = dep_date[batch_id_per_inv]
    dep_amount = np.zeros(n_batches, dtype=np.int64)
    np.add.at(dep_amount, batch_id_per_inv, inv["gross"][idx])

    dep_period = np.array([str(d)[:7] for d in dep_date], dtype=object)
    inv_period = np.array([str(d)[:7] for d in inv_dep_date], dtype=object)
    zeros_i = np.zeros(idx.size, dtype=np.int64)
    blank_i = np.full(idx.size, None, dtype=object)
    memo_i = np.array([f"Payment {i} on {d}" for i, d in zip(inv["inv_no"][idx], inv_dep)], dtype=object)
    led.add(inv_dep, inv_dep_date, inv_period, np.full(idx.size, "1100", object), blank_i, memo_i,
            zeros_i, inv["gross"][idx], "Bank", "system.ar")

    zeros_b = np.zeros(n_batches, dtype=np.int64)
    blank_b = np.full(n_batches, None, dtype=object)
    memo_b = np.array([f"Deposit {d}" for d in dep_no], dtype=object)
    led.add(dep_no, dep_date, dep_period, np.full(n_batches, "1010", object), blank_b, memo_b,
            dep_amount, zeros_b, "Bank", "system.ar")

    return {"dep_no": dep_no, "dep_date": dep_date, "dep_amount": dep_amount,
            "paid_mask": paid, "inv_dep": inv_dep, "inv_pay_date": inv_dep_date, "paid_idx": idx}


def build_ap(led: Ledger, n_bills: int):
    """Vendor bills and payments. Returns cheque data for the bank statement."""
    weights = np.ones(len(PERIODS))
    weights[10] *= 1.3
    weights[22] *= 1.3  # pre-season stocking
    midx = _month_choice(n_bills, weights)
    dates = _dates_in_month(midx)
    period = np.array(PERIODS, dtype=object)[midx]
    exp_accts = ["1200", "6110", "6200", "6400", "6500", "6510", "6600", "6610", "6620", "6700"]
    exp_p = np.array([0.34, 0.05, 0.13, 0.08, 0.05, 0.03, 0.06, 0.07, 0.13, 0.06])
    acct = RNG.choice(exp_accts, size=n_bills, p=exp_p / exp_p.sum())
    dept_map = {"1200": None, "6110": "OPS", "6200": "MKTG", "6400": "IT", "6500": "SALES", "6510": "SALES",
                "6600": "FIN", "6610": "ADMIN", "6620": "WHSE", "6700": "OPS"}
    dept = np.array([dept_map[a] for a in acct], dtype=object)
    net = _lognormal_cents(n_bills, 6.4, 1.0, 25, 80000)
    net = np.where(acct == "1200", net * 3, net).astype(np.int64)  # inventory buys are bigger
    gst = np.round(net * 0.05).astype(np.int64)
    gross = net + gst
    bill_no = np.array([f"BILL-{300000 + i}" for i in range(n_bills)], dtype=object)
    vend = _names(SECOND_VEND, n_bills)
    memo = np.array([f"Bill {b} - {v}" for b, v in zip(bill_no, vend)], dtype=object)
    zeros = np.zeros(n_bills, dtype=np.int64)
    blank = np.full(n_bills, None, dtype=object)

    led.add(bill_no, dates, period, acct, dept, memo, net, zeros, "AP", "system.ap")
    led.add(bill_no, dates, period, np.full(n_bills, "1150", object), blank, memo, gst, zeros, "AP", "system.ap")
    led.add(bill_no, dates, period, np.full(n_bills, "2000", object), blank, memo, zeros, gross, "AP", "system.ap")

    paid = RNG.random(n_bills) < 0.94
    lag = np.clip(np.round(RNG.normal(32, 9, n_bills)), 5, 90).astype(int)
    pay_date = dates + lag
    paid &= pay_date <= np.datetime64("2025-12-31")
    idx = np.where(paid)[0]
    is_chq = RNG.random(idx.size) < 0.6
    pay_no = np.array([f"CHQ-{5000 + i}" if c else f"EFT-{40000 + i}" for i, c in enumerate(is_chq)], dtype=object)
    pperiod = np.array([str(d)[:7] for d in pay_date[idx]], dtype=object)
    pz = np.zeros(idx.size, dtype=np.int64)
    pb = np.full(idx.size, None, dtype=object)
    pmemo = np.array([f"Payment {p} for {b} - {v}" for p, b, v in zip(pay_no, bill_no[idx], vend[idx])], dtype=object)
    led.add(pay_no, pay_date[idx], pperiod, np.full(idx.size, "2000", object), pb, pmemo, gross[idx], pz, "AP", "system.ap")
    led.add(pay_no, pay_date[idx], pperiod, np.full(idx.size, "1010", object), pb, pmemo, pz, gross[idx], "AP", "system.ap")

    return {"pay_no": pay_no, "pay_date": pay_date[idx], "pay_amount": gross[idx],
            "is_chq": is_chq, "vendor": vend[idx]}


def build_payroll(led: Ledger):
    """Biweekly payroll runs across departments."""
    run_dates = np.arange(np.datetime64("2024-01-05"), np.datetime64("2026-01-01"), np.timedelta64(14, "D"))
    base = {"SALES": 92000, "RENT": 31000, "WHSE": 54000, "MKTG": 28000,
            "ADMIN": 24000, "FIN": 33000, "IT": 30000, "OPS": 41000}  # dollars per run
    for r, d in enumerate(run_dates):
        ref = f"PR-{str(d)[:10]}"
        period = str(d)[:7]
        gross = {k: int(round(v * RNG.normal(1.0, 0.03) * 100)) for k, v in base.items()}
        total = sum(gross.values())
        wh = int(round(total * 0.24))
        er_tax = int(round(total * 0.072))
        net = total - wh
        n = len(base)
        led.add([ref] * n, [d] * n, [period] * n, ["6010"] * n, list(base.keys()),
                [f"Payroll {ref} gross wages"] * n, list(gross.values()), [0] * n, "Payroll", "system.payroll")
        led.add([ref] * 4, [d] * 4, [period] * 4,
                ["6020", "2400", "2400", "1010"], [None, None, None, None],
                [f"Payroll {ref} employer taxes", f"Payroll {ref} withholdings",
                 f"Payroll {ref} employer remittance accrual", f"Payroll {ref} net funding"],
                [er_tax, 0, 0, 0], [0, wh, er_tax, net], "Payroll", "system.payroll")
    return {"run_dates": run_dates}


def build_recurring(led: Ledger):
    """Monthly rent, depreciation, insurance, loan interest, remittances."""
    dep_classes = [("1511", 412500), ("1521", 287500), ("1531", 651042), ("1541", 138333)]  # cents/month
    for i, p in enumerate(PERIODS):
        mend = MONTH_START[i] + MONTH_DAYS[i] - 1
        led.add([f"RENT-{p}"] * 2, [MONTH_START[i]] * 2, [p] * 2, ["6100", "1010"], ["OPS", None],
                [f"Monthly rent {p}"] * 2, [3450000, 0], [0, 3450000], "Recurring", "system.close")
        n = len(dep_classes)
        led.add([f"DEPR-{p}"] * (n + 1), [mend] * (n + 1), [p] * (n + 1),
                ["6900"] + [c for c, _ in dep_classes], [None] * (n + 1),
                [f"Depreciation {p}"] * (n + 1),
                [sum(a for _, a in dep_classes)] + [0] * n, [0] + [a for _, a in dep_classes],
                "Recurring", "system.close")
        led.add([f"INS-{p}"] * 2, [mend] * 2, [p] * 2, ["6300", "1300"], ["ADMIN", None],
                [f"Insurance amortization {p}"] * 2, [187500, 0], [0, 187500], "Recurring", "system.close")
        loan_int = int(round((2500000 - i * 52000) * 0.062 / 12))
        led.add([f"LOAN-{p}"] * 3, [mend] * 3, [p] * 3, ["6810", "2500", "1010"], [None, None, None],
                [f"Term loan payment {p}"] * 3, [loan_int, 5200000 // 100, 0], [0, 0, loan_int + 52000], "Recurring", "system.close")


def build_bank_statement(receipts, ap, payroll_runs, exclude: set[str]):
    """Build the bank statement for account 1010 with seeded rec differences.

    Refs in `exclude` are seeded findings whose bank lines are appended manually
    in apply_findings; they must not also clear through the normal path.
    """
    rows: list[tuple] = []  # (date, desc, ref, amount_cents)

    dep_lag = RNG.integers(0, 3, receipts["dep_no"].size)
    for d, dt, amt, lag in zip(receipts["dep_no"], receipts["dep_date"], receipts["dep_amount"], dep_lag):
        cleared = dt + int(lag)
        if cleared <= np.datetime64("2025-12-31"):
            rows.append((cleared, "CUSTOMER DEPOSIT", str(d), int(amt)))
        # deposits recorded in GL late December that clear in January = deposits in transit

    never_clear = RNG.random(ap["pay_no"].size) < 0.015  # outstanding cheques
    clear_lag = RNG.integers(1, 13, ap["pay_no"].size)
    for p, dt, amt, chq, lag, nc, v in zip(ap["pay_no"], ap["pay_date"], ap["pay_amount"],
                                           ap["is_chq"], clear_lag, never_clear, ap["vendor"]):
        if (nc and chq) or str(p) in exclude:
            continue
        cleared = dt + (int(lag) if chq else int(lag % 2))
        if cleared <= np.datetime64("2025-12-31"):
            desc = "CHEQUE" if chq else f"EFT {v.upper()}"
            rows.append((cleared, desc, str(p), -int(amt)))

    for d in payroll_runs["run_dates"]:
        pass  # payroll nets are funded via provider EFT below

    # payroll provider EFTs mirror the GL net funding amounts closely; rebuild from base logic is
    # complex, so pull them from the ledger later instead (see main()).

    for i, p in enumerate(PERIODS):
        mend = MONTH_START[i] + MONTH_DAYS[i] - 1
        rows.append((mend, "SERVICE CHARGE", f"SVC-{p}", -12500))
        rows.append((mend, "INTEREST", f"INT-{p}", int(4100 + RNG.integers(0, 2500))))
        rows.append((MONTH_START[i] + 2, "PREAUTH RENT - BOW VALLEY PROPERTIES", f"RENT-{p}", -3450000))

    return rows


def apply_findings(gl: pl.DataFrame, bank_rows: list[tuple], seeded: dict) -> tuple[pl.DataFrame, list[str]]:
    """Plant the documented anomalies. Returns modified GL and the findings list."""
    findings: list[str] = []

    # 1. Three unbalanced journal entries (import/validation demo).
    targets = (gl.filter(pl.col("source") == "AP").select("source_ref").unique().sort("source_ref").head(3)
               .get_column("source_ref").to_list())
    bumps = [1, -1, 10000]  # +$0.01, -$0.01, +$100.00
    for ref, bump in zip(targets, bumps):
        mask = (pl.col("source_ref") == ref) & (pl.col("debit_cents") > 0)
        first_debit = gl.filter(mask).head(1)
        gl = gl.with_columns(
            pl.when((pl.col("source_ref") == ref) & (pl.col("memo") == first_debit["memo"][0]) &
                    (pl.col("account_code") == first_debit["account_code"][0]))
            .then(pl.col("debit_cents") + bump).otherwise(pl.col("debit_cents")).alias("debit_cents"))
        findings.append(f"Unbalanced JE: {ref} is out of balance by {bump / 100:+.2f}")

    # 2. Duplicate vendor payment, both clearing the bank.
    dup_ref, dup_amt, dup_date = seeded["dup_ref"], seeded["dup_amt"], seeded["dup_date"]
    dup_lines = gl.filter(pl.col("source_ref") == dup_ref)
    dup_new = dup_lines.with_columns(
        (pl.col("source_ref") + "-DUP").alias("source_ref"),
        (pl.col("posted_date") + pl.duration(days=4)).alias("posted_date"),
        (pl.col("memo") + " (reissued)").alias("memo"))
    gl = pl.concat([gl, dup_new])
    bank_rows.append((dup_date + np.timedelta64(6, "D"), "CHEQUE", f"{dup_ref}", -dup_amt))
    bank_rows.append((dup_date + np.timedelta64(9, "D"), "CHEQUE", f"{dup_ref}-DUP", -dup_amt))
    findings.append(f"Duplicate payment: {dup_ref} and {dup_ref}-DUP, same vendor, {dup_amt / 100:,.2f} each, both cleared the bank")

    # 3. Round-dollar year-end revenue accruals by a named user (audit smell).
    ye = np.array(["2025-12-31", "2025-12-31"], dtype="datetime64[D]")
    for i, amt_d in enumerate([50000, 75000, 100000]):
        ref = f"JE-MGMT-{i + 1}"
        gl = pl.concat([gl, pl.DataFrame({
            "source_ref": [ref, ref], "period": ["2025-12"] * 2,
            "account_code": ["1100", "4010"], "department": [None, "SALES"],
            "memo": ["Revenue accrual - management adjustment"] * 2,
            "source": ["Manual"] * 2, "created_by": ["jsmith"] * 2,
            "posted_date": ye,
            "debit_cents": [amt_d * 100, 0], "credit_cents": [0, amt_d * 100],
        }).select(gl.columns)])
    findings.append("Round-dollar management revenue accruals: JE-MGMT-1/2/3 ($50k/$75k/$100k) posted 2025-12-31 by jsmith")

    # 4. Cutoff errors: January 2026 sales posted into period 2025-12.
    cutoff = gl.filter((pl.col("source") == "AR") & (pl.col("period") == "2025-12") &
                       pl.col("source_ref").str.starts_with("INV-")).select("source_ref").unique().head(400)
    cutoff_refs = cutoff.get_column("source_ref")
    gl = gl.with_columns(
        pl.when(pl.col("source_ref").is_in(cutoff_refs.implode()))
        .then(pl.col("posted_date") + pl.duration(days=14)).otherwise(pl.col("posted_date")).alias("posted_date"))
    findings.append("Cutoff: ~400 invoices have January 2026 posted dates but sit in period 2025-12")

    # 5. Transposition error: GL cheque amount differs from bank by 54.00 (divisible by 9).
    tr_ref, tr_amt, tr_date = seeded["tr_ref"], seeded["tr_amt"], seeded["tr_date"]
    bank_rows.append((tr_date + np.timedelta64(3, "D"), "CHEQUE", str(tr_ref), -(tr_amt - 5400)))
    findings.append(f"Transposition: {tr_ref} recorded in GL {tr_amt / 100:,.2f} but cleared bank at {(tr_amt - 5400) / 100:,.2f} (difference divisible by 9)")

    # 6. Bank-only items: an unexplained service adjustment; two months of fees never booked to GL.
    bank_rows.append((np.datetime64("2025-08-14"), "SERVICE CHG ADJ", "SVC-ADJ-1", -34750))
    findings.append("Bank-only: SERVICE CHG ADJ 347.50 on 2025-08-14 never recorded in the GL")
    findings.append("Bank fees for 2024-03 and 2025-07 (125.00/mo) were never booked to 6800 (GL books fees one month in arrears)")

    return gl, findings


def book_bank_fees(led: Ledger):
    """GL books bank service charges one month in arrears; two months are skipped entirely."""
    skip = {"2024-03", "2025-07"}
    for i, p in enumerate(PERIODS):
        if p in skip or i == len(PERIODS) - 1:
            continue
        nxt = PERIODS[i + 1]
        d = MONTH_START[i + 1] + 4
        led.add([f"BANKFEE-{p}"] * 2, [d] * 2, [nxt] * 2, ["6800", "1010"], ["FIN", None],
                [f"Bank service charges for {p}"] * 2, [12500, 0], [0, 12500], "Recurring", "system.close")
        # interest income booked quarterly as a lump; keep GL/bank timing difference


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="demo-data")
    parser.add_argument("--invoices", type=int, default=132000)
    parser.add_argument("--bills", type=int, default=76000)
    args = parser.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    led = Ledger()
    inv = build_sales(led, args.invoices)
    receipts = build_receipts(led, inv)
    ap = build_ap(led, args.bills)
    payroll = build_payroll(led)
    build_recurring(led)
    book_bank_fees(led)

    gl = led.frame()

    # Pick material cheques (paid well before year end) for the seeded findings.
    cand = np.where(ap["is_chq"] & (ap["pay_amount"] > 200_000) & (ap["pay_amount"] < 800_000)
                    & (ap["pay_date"] < np.datetime64("2025-11-01")))[0]
    dup_i, tr_i = cand[10], cand[200]
    seeded = {"dup_ref": str(ap["pay_no"][dup_i]), "dup_amt": int(ap["pay_amount"][dup_i]),
              "dup_date": ap["pay_date"][dup_i],
              "tr_ref": str(ap["pay_no"][tr_i]), "tr_amt": int(ap["pay_amount"][tr_i]),
              "tr_date": ap["pay_date"][tr_i]}
    bank_rows = build_bank_statement(receipts, ap, payroll, exclude={seeded["dup_ref"], seeded["tr_ref"]})

    # payroll provider EFTs on the statement mirror GL net funding exactly
    pr_net = (gl.filter(pl.col("source_ref").str.starts_with("PR-") & (pl.col("account_code") == "1010"))
              .select("source_ref", "posted_date", "credit_cents"))
    for ref, d, amt in pr_net.iter_rows():
        bank_rows.append((np.datetime64(str(d)) + 1, "EFT NORTHPAY PAYROLL SERVICES", str(ref), -int(amt)))

    gl, findings = apply_findings(gl, bank_rows, seeded)

    # Stable JE ids ordered by first posted date, line numbers within each JE.
    first_dates = gl.group_by("source_ref").agg(pl.col("posted_date").min().alias("d"))
    first_dates = first_dates.sort(["d", "source_ref"]).with_row_index("je_seq")
    gl = (gl.join(first_dates.select("source_ref", "je_seq"), on="source_ref", how="left")
          .sort(["je_seq", "posted_date"])
          .with_columns((pl.col("je_seq") + 1).map_elements(lambda s: f"JE-{s:07d}", return_dtype=pl.String).alias("je_id"))
          .with_columns(pl.int_range(pl.len()).over("je_id").alias("line_no") + 1)
          .with_columns([
              pl.when(pl.col("debit_cents") != 0).then(pl.col("debit_cents") / 100).alias("debit"),
              pl.when(pl.col("credit_cents") != 0).then(pl.col("credit_cents") / 100).alias("credit"),
              pl.col("account_code").replace_strict(ACCOUNT_NAME, default="?").alias("account_name"),
              pl.lit("CAD").alias("currency"),
          ])
          .select(["je_id", "line_no", "posted_date", "period", "account_code", "account_name",
                   "department", "memo", "debit", "credit", "currency", "source", "source_ref", "created_by"]))

    # ---- Supporting tables ----
    coa = pl.DataFrame([{"account_code": c, "account_name": n, "account_type": t, "subtype": s,
                         "normal_balance": "Debit" if t in ("Asset", "Expense") else "Credit"}
                        for c, n, t, s in ACCOUNTS])

    depts = pl.DataFrame({"department": DEPTS,
                          "department_name": ["Retail Sales", "Rentals & Guiding", "Warehouse", "Marketing",
                                              "Administration", "Finance", "Information Technology", "Operations"]})

    paid_date = np.full(inv["inv_no"].size, None, dtype=object)
    dep_ref = np.full(inv["inv_no"].size, None, dtype=object)
    paid_date[receipts["paid_idx"]] = [str(d) for d in receipts["inv_pay_date"]]
    dep_ref[receipts["paid_idx"]] = receipts["inv_dep"]
    invoices = pl.DataFrame({
        "invoice_no": inv["inv_no"].tolist(), "invoice_date": inv["date"],
        "due_date": inv["date"] + 30, "customer": inv["customer"].tolist(),
        "net": (inv["net"] / 100), "gst": (inv["gst"] / 100), "total": (inv["gross"] / 100),
        "paid_date": paid_date.tolist(), "deposit_ref": dep_ref.tolist(),
    }).with_columns(pl.when(pl.col("paid_date").is_null()).then(pl.lit("Open")).otherwise(pl.lit("Paid")).alias("status"))

    # Budget 2025: grown 2024 actuals with noise; seeded gaps for anti-join demos.
    actuals_2024 = (gl.filter(pl.col("period").str.starts_with("2024") &
                              pl.col("account_code").is_in([a[0] for a in ACCOUNTS if a[2] in ("Revenue", "Expense")]))
                    .with_columns((pl.col("credit").fill_null(0) - pl.col("debit").fill_null(0)).alias("signed"))
                    .group_by(["account_code", "department", "period"])
                    .agg(pl.col("signed").sum()))
    month_of = pl.col("period").str.slice(5, 2)
    # Polars hashes are unsigned. Cast before centering the bucket around zero;
    # subtracting 50 from UInt64 would wrap half the values into enormous budgets.
    budget_noise = ((pl.col("signed").hash(seed=1) % 100).cast(pl.Int64) - 50)
    budget = (actuals_2024
              .with_columns(("2025-" + month_of).alias("period"))
              .with_columns((pl.col("signed").abs() * (1.06 + 0.08 * budget_noise / 100))
                            .round(0).alias("budget_amount"))
              .drop("signed")
              .filter(pl.col("budget_amount") > 0))
    budget = budget.filter(pl.col("account_code").hash(seed=7) % 33 != 0)  # ~3% of lines missing
    retail = pl.DataFrame({"account_code": ["6200"] * 12, "department": ["RETAIL"] * 12,
                           "period": [f"2025-{m:02d}" for m in range(1, 13)],
                           "budget_amount": [4500.0] * 12})  # budgeted dept that has no actuals
    budget = pl.concat([budget, retail]).sort(["account_code", "department", "period"])

    bank = (pl.DataFrame({"date": np.array([r[0] for r in bank_rows], dtype="datetime64[D]"),
                          "description": [r[1] for r in bank_rows],
                          "reference": [r[2] for r in bank_rows], "amount": [r[3] / 100 for r in bank_rows]})
            .sort(["date", "reference"])
            .with_columns((250000.0 + pl.col("amount").cum_sum()).round(2).alias("balance")))

    # ---- Self-checks ----
    total_dr = gl["debit"].fill_null(0).sum()
    total_cr = gl["credit"].fill_null(0).sum()
    imbalance = (gl.group_by("je_id")
                 .agg((pl.col("debit").fill_null(0).sum() - pl.col("credit").fill_null(0).sum()).round(2).alias("diff"))
                 .filter(pl.col("diff") != 0))
    assert imbalance.height == 3, f"expected exactly 3 seeded unbalanced JEs, found {imbalance.height}"
    max_actual = actuals_2024["signed"].abs().max()
    max_budget = budget["budget_amount"].max()
    assert budget["budget_amount"].is_finite().all(), "budget contains non-finite amounts"
    assert max_budget <= max_actual * 1.11 + 1, f"implausible budget amount {max_budget:,.2f}"

    # ---- Write ----
    gl_csv = gl.with_columns(pl.col("posted_date").dt.to_string("%Y-%m-%d"))
    gl_csv.write_csv(out / "general_ledger.csv", float_precision=2)
    gl.write_parquet(out / "general_ledger.parquet")
    q4 = gl_csv.filter(pl.col("period").is_in(["2025-10", "2025-11", "2025-12"]))
    q4.write_csv(out / "general_ledger_2025q4.csv", float_precision=2)
    coa.write_csv(out / "chart_of_accounts.csv")
    depts.write_csv(out / "departments.csv")
    invoices.with_columns(pl.col("invoice_date").dt.to_string("%Y-%m-%d"),
                          pl.col("due_date").dt.to_string("%Y-%m-%d")).write_csv(out / "ar_invoices.csv", float_precision=2)
    budget.write_csv(out / "budget_2025.csv", float_precision=2)
    budget.write_parquet(out / "budget_2025.parquet")
    bank.with_columns(pl.col("date").dt.to_string("%Y-%m-%d")).write_csv(out / "bank_statement_operating.csv", float_precision=2)

    print(f"GL lines: {gl.height:,}  (JEs: {gl['je_id'].n_unique():,})")
    print(f"Q4-2025 slice: {q4.height:,} lines")
    print(f"Bank statement lines: {bank.height:,}")
    print(f"AR invoices: {invoices.height:,}  Budget lines: {budget.height:,}")
    print(f"Total debits {total_dr:,.2f} vs credits {total_cr:,.2f} (diff {total_dr - total_cr:+,.2f} = seeded imbalances)")
    print("\nSeeded findings:")
    for f in findings:
        print(f"  - {f}")


if __name__ == "__main__":
    main()
