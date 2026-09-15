use framework_core::*;

pub(super) fn evaluate(source: &str) -> ComputedCell {
    let mut store = Store::new(Document::blank("Finance"));
    store
        .apply(Operation::AddBlock {
            name: "Checks".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let object_id = store.document().objects[0].id().to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: object_id,
            source: source.into(),
            editing: None,
        })
        .unwrap();
    store
        .view()
        .computed_blocks
        .values()
        .next()
        .unwrap()
        .lines
        .last()
        .unwrap()
        .cell
        .clone()
}

pub(super) fn close(source: &str, expected: f64) {
    let cell = evaluate(source);
    assert!(cell.error.is_none(), "{source}: {:?}", cell.error);
    let actual = cell.value.unwrap();
    assert!(
        (actual - expected).abs() < 1e-7 * expected.abs().max(1.0),
        "{source}: {actual} != {expected}"
    );
}

#[test]
fn published_and_hand_calculated_financial_examples() {
    // Independent published examples, not output recorded from this engine:
    // numpy.org/numpy-financial/latest/{pv,fv,pmt,nper}.html and Microsoft's
    // IPMT/XNPV help. The small zero-rate cases are direct cash conservation.
    // Microsoft rounds DDB to cents; the tenth-year and first-day cases below
    // carry the full precision behind that rounding, verified by hand against
    // the published MIN formula.
    for (source, expected) in [
        ("pmt(0.075/12, 12*15, 200000)", -1854.0247200054619),
        ("pv(0.05/12, 10*12, -100, 15692.93)", -100.00067131625819),
        ("fv(0.05/12, 10*12, -100, -100)", 15692.928894),
        ("nper(0.07/12, -150, 8000)", 64.07334877),
        ("IPMT(0.1/12, 1, 3*12, 8000)", -66.6666666667),
        ("pmt(0, 10, 1000)", -100.0),
        ("pv(0, 10, -100, -50)", 1050.0),
        ("fv(0, 10, -100, -50)", 1050.0),
        ("nper(0, -100, 1000, -50)", 9.5),
        ("ipmt(0, 2, 10, 1000)", 0.0),
        ("ppmt(0, 2, 10, 1000)", -100.0),
        ("pmt(0.1, 2, 100, type=1)", -52.38095238095238),
        ("ipmt(0.1, 1, 2, 100, 0, 1)", 0.0),
        ("ipmt(0.1, 2, 2, 100, 0, 1)", -4.761904761904762),
        ("ppmt(0.1, 2, 2, 100, 0, 1)", -47.61904761904762),
        ("pmt(-0.1, 2, 100)", -42.63157894736842),
        ("pmt(0.000000000001, 10, 1000)", -100.00000000055),
        ("npv(0.1, [110, 121])", 200.0),
        ("npv(0, [-100, 40, 80])", 20.0),
        (
            "xnpv(0.1, [-100, 110], [date(2025,1,1), date(2026,1,1)])",
            0.0,
        ),
        (
            "xnpv(0.09, [-10000, 2750, 4250, 3250, 2750], [date(2008,1,1), date(2008,3,1), date(2008,10,30), date(2009,2,15), date(2009,4,1)])",
            2086.64760203154,
        ),
        (
            "irr([-70000,12000,15000,18000,21000,26000])",
            0.0866309480365,
        ),
        ("irr([-100,90])", -0.1),
        ("irr([-100,100])", 0.0),
        (
            "xirr([-10000,2750,4250,3250,2750], [date(2008,1,1), date(2008,3,1), date(2008,10,30), date(2009,2,15), date(2009,4,1)])",
            0.373362535,
        ),
        ("xirr([-100,110], [date(2025,1,1),date(2026,1,1)])", 0.1),
        // Microsoft EFFECT/NOMINAL/SLN/DB/DDB examples and the RATE loan
        // from Microsoft's RATE help (monthly rate ~0.77%, annual 9.24%).
        ("effect(0.0525, 4)", 0.0535427),
        ("nominal(0.053543, 4)", 0.05250032),
        ("sln(30000, 7500, 10)", 2250.0),
        ("sln(10000, 2000, 5)", 1600.0),
        ("db(1000000, 100000, 6, 1, 7)", 186083.33),
        ("db(1000000, 100000, 6, 2, 7)", 259639.42),
        ("db(1000000, 100000, 6, 3, 7)", 176814.44),
        ("db(1000000, 100000, 6, 4, 7)", 120410.64),
        ("db(1000000, 100000, 6, 5, 7)", 81999.64),
        ("db(1000000, 100000, 6, 6, 7)", 55841.76),
        ("db(1000000, 100000, 6, 7, 7)", 15845.10),
        ("db(10000, 1000, 5, 1)", 3690.0),
        ("ddb(2400, 300, 10, 1, 2)", 480.0),
        ("ddb(2400, 300, 10, 2, 1.5)", 306.0),
        ("ddb(2400, 300, 10, 10)", 22.1225472),
        ("ddb(2400, 300, 10*12, 1, 2)", 40.0),
        ("ddb(2400, 300, 10*365, 1)", 1.315068493150685),
        ("ddb(10000, 1000, 5, 1)", 4000.0),
        ("ddb(10000, 1000, 5, 5)", 296.0),
        ("rate(48, -200, 8000)", 0.00770147248823337),
        ("rate(12*15, -1854.0247200054619, 200000)", 0.00625),
        ("rate(60, -93.22, 5000) * 12", 0.04502156849021323),
        (
            "mirr([-120000,39000,30000,21000,37000,46000], 0.1, 0.12)",
            0.1260941303659051,
        ),
        (
            "mirr([-120000,39000,30000,21000], 0.1, 0.12)",
            -0.0480446552499809,
        ),
        (
            "mirr([-1000,400,400,400,400], 0.1, 0.12)",
            0.17586295137979602,
        ),
    ] {
        close(source, expected);
    }
}

#[test]
fn financial_errors_are_visible_and_nulls_are_not_zero() {
    for formula in [
        "pmt(0.1, 0, 100)",
        "pmt(-1, 10, 100)",
        "pmt(0.1, 10, 100, type=2)",
        "ipmt(0.1, 0, 10, 100)",
        "ppmt(0.1, 11, 10, 100)",
        "ipmt(0.1, 1.5, 10, 100)",
        "nper(0, 0, 100)",
        "nper(0.1, 0, 100, 100)",
        "npv(0.1, [10, None])",
        "npv(-1, [10, 20])",
        "xnpv(0.1, [10,20], [date(2026,1,1),date(2025,1,1)])",
        "xnpv(0.1, [10,20], [date(2026,1,1)])",
        "xnpv(0.1, [10,20], [1,2])",
        "irr([10,20])",
        "irr([-100,200,-150])",
        "irr([-100,200,-100.000000000001])",
        "irr([-100,110], guess=-1)",
        "irr([-100,None,120])",
        "xirr([-100,110], [date(2026,1,1),date(2025,1,1)])",
        "xirr([-100,110], [date(2025,1,1)])",
        "xirr([-100,110], [1,2])",
        "xirr([-100,100], [date(2025,1,1),date(2025,1,1)])",
        "irr([-1e300,1e-300])",
        "effect(0, 4)",
        "effect(0.05, 0)",
        "effect(0.05, 0.5)",
        "nominal(0, 4)",
        "nominal(0.05, 0)",
        "sln(10000, 1000, 0)",
        "db(0, 0, 5, 1)",
        "db(10000, 20000, 5, 1)",
        "db(10000, 1000, 0, 1)",
        "db(10000, 1000, 5, 0)",
        "db(10000, 1000, 5, 6)",
        "db(10000, 1000, 5, 1, 0)",
        "db(10000, 1000, 5, 1, 13)",
        "ddb(10000, 1000, 5, 0)",
        "ddb(10000, 1000, 5, 6)",
        "ddb(10000, 1000, 0, 1)",
        "ddb(10000, 20000, 5, 1)",
        "ddb(10000, 1000, 5, 1, -1)",
        "rate(0, -100, 1000)",
        "rate(10, -100, 1000, type=2)",
        "rate(10, -100, 1000, guess=-1)",
        "rate(10, 100, 100)",
        "mirr([10,20], 0.1, 0.1)",
        "mirr([-100,110], -1, 0.1)",
        "mirr([-100,110], 0.1, -2)",
        "mirr([-100,None,110], 0.1, 0.1)",
        "mirr([-100,110], 0.1)",
    ] {
        let cell = evaluate(formula);
        assert!(
            cell.error.is_some(),
            "{formula} unexpectedly returned {:?}",
            cell.value
        );
    }
    let cell = evaluate("pmt(0.1, 10, None)");
    assert!(cell.error.is_none(), "{:?}", cell.error);
    assert!(cell.value.is_none());
    for blank in [
        "effect(None, 4)",
        "sln(None, 0, 1)",
        "db(None, 0, 1, 1)",
        "ddb(1, 0, 1, None)",
        "rate(None, -100, 1000)",
    ] {
        let cell = evaluate(blank);
        assert!(cell.error.is_none(), "{blank}: {:?}", cell.error);
        assert!(cell.value.is_none(), "{blank}");
    }
}

#[test]
fn financial_catalog_and_uppercase_calls_are_available() {
    let catalog = formula_function_catalog();
    let document = Document::demo();
    let frame_id = document
        .objects
        .iter()
        .find_map(|o| {
            if let DataObject::Frame(f) = o {
                Some(f.id.clone())
            } else {
                None
            }
        })
        .unwrap();
    for name in [
        "pv", "fv", "pmt", "ipmt", "ppmt", "nper", "npv", "xnpv", "irr", "xirr", "effect",
        "nominal", "sln", "db", "ddb", "rate", "mirr",
    ] {
        let entry = catalog.iter().find(|f| f.name == name).unwrap();
        assert!(entry.aliases.contains(&name.to_uppercase()));
        let completion = complete_formula(&document, &frame_id, &name.to_uppercase(), name.len());
        assert!(
            completion.suggestions.iter().any(|s| s.id == entry.id),
            "{name} missing from completion"
        );
    }
    close("PMT(0, 10, 100)", -10.0);
    close("NPV(0, [10,20])", 30.0);
    close("IRR([-100,110])", 0.1);
    close("EFFECT(0.0525, 4)", 0.0535427);
    close(
        "MIRR([-1000,400,400,400,400], 0.1, 0.12)",
        0.17586295137979602,
    );
}

#[test]
fn return_solver_selects_multiple_roots_and_is_scale_invariant() {
    close("irr([-100,230,-132], guess=0.09)", 0.1);
    close("irr([-100,230,-132], guess=0.21)", 0.2);
    close("irr([-100,200,-100])", 0.0);
    close("irr([-100000000,230000000,-132000000], guess=0.21)", 0.2);
    close(
        "xirr([-100,100,-100,110], [date(2025,1,1),date(2025,1,1),date(2025,1,1),date(2026,1,1)])",
        0.1,
    );
    let expected = (0.1_f64.ln_1p() * 365.0 / 73049.0).exp_m1();
    close(
        "xirr([-100,110], [date(2000,1,1),date(2200,1,1)])",
        expected,
    );
}
