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
];
