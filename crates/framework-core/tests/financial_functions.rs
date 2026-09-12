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
    for name in ["pv", "fv", "pmt", "ipmt", "ppmt", "nper", "npv", "xnpv"] {
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
}
