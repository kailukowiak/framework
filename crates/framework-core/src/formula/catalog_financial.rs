use super::FormulaFunctionDefinition;

pub(crate) const FUNCTIONS: &[FormulaFunctionDefinition] = &[
    formula_function!(
        "root.pv",
        "pv",
        ["PV", "present value"],
        "Financial",
        "pv(rate, nper, pmt, fv=0, type=0)",
        "Present value of equal periodic payments. Outflows are negative; type=0 pays at period end, type=1 at its start. Rate is per period and must exceed -1; nper must be positive.",
        3,
        5
    ),
    formula_function!(
        "root.fv",
        "fv",
        ["FV", "future value"],
        "Financial",
        "fv(rate, nper, pmt, pv=0, type=0)",
        "Future value of equal periodic payments. Outflows are negative; type=0 pays at period end, type=1 at its start. Rate is per period and must exceed -1; nper must be positive.",
        3,
        5
    ),
    formula_function!(
        "root.pmt",
        "pmt",
        ["PMT", "loan payment"],
        "Financial",
        "pmt(rate, nper, pv, fv=0, type=0)",
        "Equal periodic payment, including principal and interest. Outflows are negative; type=0 pays at period end, type=1 at its start. Rate must exceed -1; nper must be positive.",
        3,
        5
    ),
    formula_function!(
        "root.ipmt",
        "ipmt",
        ["IPMT", "interest payment"],
        "Financial",
        "ipmt(rate, per, nper, pv, fv=0, type=0)",
        "Interest part of a payment, with Excel signs and argument order. per is an integer from 1 through nper. type=1 pays at period start, with no first-period interest.",
        4,
        6
    ),
    formula_function!(
        "root.ppmt",
        "ppmt",
        ["PPMT", "principal payment"],
        "Financial",
        "ppmt(rate, per, nper, pv, fv=0, type=0)",
        "Principal part of a payment: pmt minus ipmt. per is an integer from 1 through nper. type=0 pays at period end; type=1 at its start.",
        4,
        6
    ),
    formula_function!(
        "root.nper",
        "nper",
        ["NPER", "number of payments"],
        "Financial",
        "nper(rate, pmt, pv, fv=0, type=0)",
        "Number of periods for equal payments. Uses Excel signs and argument order; rate must exceed -1. type=0 pays at period end; type=1 at its start. Undefined solutions error.",
        3,
        5
    ),
    formula_function!(
        "root.npv",
        "npv",
        ["NPV", "net present value"],
        "Financial",
        "npv(rate, values)",
        "Discount a nonempty cash-flow column in supplied order. Like Excel, the first flow is one period away; add a time-zero investment separately. Missing flows error. Prefer xnpv for dated flows.",
        2,
        2
    ),
    formula_function!(
        "root.xnpv",
        "xnpv",
        ["XNPV", "dated net present value"],
        "Financial",
        "xnpv(rate, values, dates)",
        "Discount matching cash-flow and Date columns on an actual/365 basis. The first date is time zero; earlier dates and missing values error. Rate is scalar, finite, and greater than -1.",
        3,
        3
    ),
    formula_function!(
        "root.irr",
        "irr",
        ["IRR", "internal rate of return"],
        "Financial",
        "irr(values, guess=0.1)",
        "Periodic internal rate of return for a nonempty cash-flow column in supplied order, with the first flow at time zero. Flows must contain both signs. Searches a bounded rate domain and chooses the discovered root nearest guess in log-rate space.",
        1,
        2
    ),
    formula_function!(
        "root.xirr",
        "xirr",
        ["XIRR", "dated internal rate of return"],
        "Financial",
        "xirr(values, dates, guess=0.1)",
        "Dated internal rate of return on an actual/365 basis, using the first supplied date as time zero. Flows must contain both signs. Searches a bounded rate domain and chooses the discovered root nearest guess in log-rate space.",
        2,
        3
    ),
    formula_function!(
        "root.effect",
        "effect",
        ["EFFECT", "effective rate", "effective annual rate"],
        "Financial",
        "effect(nominal_rate, npery)",
        "Effective annual rate from a nominal rate and compounding periods per year. Npery is truncated to an integer; nominal_rate must be positive and npery at least 1.",
        2,
        2
    ),
    formula_function!(
        "root.nominal",
        "nominal",
        ["NOMINAL", "nominal rate"],
        "Financial",
        "nominal(effect_rate, npery)",
        "Nominal annual rate from an effective rate and compounding periods per year. Npery is truncated to an integer; effect_rate must be positive and npery at least 1.",
        2,
        2
    ),
    formula_function!(
        "root.sln",
        "sln",
        [
            "SLN",
            "straight line depreciation",
            "straight-line depreciation"
        ],
        "Financial",
        "sln(cost, salvage, life)",
        "Straight-line depreciation for one period: (cost - salvage) / life. Life must be nonzero.",
        3,
        3
    ),
    formula_function!(
        "root.db",
        "db",
        [
            "DB",
            "declining balance depreciation",
            "fixed declining balance"
        ],
        "Financial",
        "db(cost, salvage, life, period, month=12)",
        "Fixed-declining-balance depreciation for one period. Rate is rounded to three decimals; first and last periods are prorated by month. Cost must be positive, salvage between 0 and cost, life at least 1, period within the schedule, month 1 through 12.",
        4,
        5
    ),
    formula_function!(
        "root.ddb",
        "ddb",
        ["DDB", "double declining depreciation", "declining balance"],
        "Financial",
        "ddb(cost, salvage, life, period, factor=2)",
        "Declining-balance depreciation for one period, capped so book value never drops below salvage. Life and period are truncated to integers; period must be within 1 and life, factor nonnegative.",
        4,
        5
    ),
    formula_function!(
        "root.rate",
        "rate",
        ["RATE", "interest rate", "periodic rate"],
        "Financial",
        "rate(nper, pmt, pv, fv=0, type=0, guess=0.1)",
        "Periodic interest rate of an annuity, solved with the bounded root finder. Nper must be positive, type 0 or 1, guess finite and greater than -1. Refuses with a reason when no root is found.",
        3,
        6
    ),
    formula_function!(
        "root.mirr",
        "mirr",
        ["MIRR", "modified internal rate of return"],
        "Financial",
        "mirr(values, finance_rate, reinvest_rate)",
        "Modified internal rate of return: negatives discounted at finance_rate, positives compounded at reinvest_rate. Flows must contain both signs; rates must be finite scalars greater than -1.",
        3,
        3
    ),
    formula_function!(
        "root.period_index",
        "period_index",
        ["PERIOD_INDEX", "period number", "fiscal period index"],
        "Financial",
        "period_index(date, fy_start=1)",
        "Monthly period number since year zero for a date, counting fiscal months from fy_start (1 is January). Offsets between two indexes are whole periods without date math.",
        1,
        2
    ),
    formula_function!(
        "root.prior",
        "prior",
        ["PRIOR", "previous period", "prior period"],
        "Financial",
        "prior(expr, n=1, fy_start=1)",
        "The value expr held n periods before each row's own period, joined on the frame's declared period column — the period before, never the row above. Needs a declared period; a missing earlier period reads blank.",
        1,
        3
    ),
    formula_function!(
        "root.fiscal_year",
        "fiscal_year",
        ["FISCAL_YEAR", "fiscal year", "financial year"],
        "Financial",
        "fiscal_year(date, fy_start=1)",
        "The fiscal year a date falls in, where the year starts in month fy_start (1 is January). With a February start, January 2025 is fiscal 2024.",
        1,
        2
    ),
    formula_function!(
        "root.fiscal_quarter",
        "fiscal_quarter",
        ["FISCAL_QUARTER", "fiscal quarter", "financial quarter"],
        "Financial",
        "fiscal_quarter(date, fy_start=1)",
        "The fiscal quarter from 1 to 4 a date falls in, counting three-month quarters from fy_start. Needs no period declaration; it reads the date, not the frame.",
        1,
        2
    ),
    formula_function!(
        "root.fiscal_period",
        "fiscal_period",
        ["FISCAL_PERIOD", "fiscal period", "fiscal month"],
        "Financial",
        "fiscal_period(date, fy_start=1)",
        "The fiscal month from 1 to 12 a date falls in, counting from fy_start. This is the one-date reading of the numbering period_index counts.",
        1,
        2
    ),
    formula_function!(
        "root.period_start",
        "period_start",
        ["PERIOD_START", "period start", "month start"],
        "Financial",
        "period_start(date)",
        "The first day of the calendar month holding a date. Pair with period_end to bound a period without spelling month lengths.",
        1,
        1
    ),
    formula_function!(
        "root.period_end",
        "period_end",
        ["PERIOD_END", "period end", "month end"],
        "Financial",
        "period_end(date)",
        "The last day of the calendar month holding a date: February 2024 ends on the 29th. The month-end reading of EDATE(date, 0).",
        1,
        1
    ),
    formula_function!(
        "root.add_periods",
        "add_periods",
        ["EDATE", "add months", "shift months"],
        "Financial",
        "add_periods(date, n)",
        "The date n calendar months from a date, with end-of-month clamping: January 31 plus one month is February 28. The count may be a column; a missing count reads blank. EDATE is the Excel name for this exact arithmetic.",
        2,
        2
    ),
];
