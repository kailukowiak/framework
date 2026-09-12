use super::financial_functions::{close, evaluate};
use framework_core::*;

#[test]
fn finance_namespace_and_receivers_share_the_financial_functions() {
    for (formula, expected) in [
        ("finance.pmt(0, 10, 100)", -10.0),
        ("finance.pv(0, 10, -100)", 1000.0),
        ("finance.fv(0, 10, -100)", 1000.0),
        ("finance.nper(0, -100, 1000)", 10.0),
        ("finance.ipmt(0.1, 1, 10, 1000)", -100.0),
        ("finance.ppmt(0, 1, 10, 1000)", -100.0),
        ("finance.npv(0.1, [110, 121])", 200.0),
        ("finance.npv(rate=0.1, values=[110, 121])", 200.0),
        (
            "[-100,110].finance.xnpv(rate=0.1, dates=[date(2025,1,1),date(2026,1,1)]).round(2)",
            0.0,
        ),
        (
            "finance.xnpv(0.1, [-100, 110], [date(2025,1,1),date(2026,1,1)])",
            0.0,
        ),
        ("principal = 100\nprincipal.finance.pmt(0, 10)", -10.0),
        ("payment = -100\npayment.finance.pv(0, 10)", 1000.0),
        (
            "principal = -100\nprincipal.finance.fv(0, 10, -100)",
            1100.0,
        ),
        ("principal = 1000\nprincipal.finance.nper(0, -100)", 10.0),
        (
            "principal = 1000\nprincipal.finance.ipmt(0.1, 1, 10)",
            -100.0,
        ),
        ("principal = 1000\nprincipal.finance.ppmt(0, 1, 10)", -100.0),
        ("[110,121].finance.npv(0.1)", 200.0),
        (
            "[-100,110].finance.xnpv(0.1, [date(2025,1,1),date(2026,1,1)]).round(2)",
            0.0,
        ),
        (
            "principal = 100\nprincipal.FINANCE.PMT(rate=0, nper=10, fv=50, type=1)",
            -15.0,
        ),
        ("FINANCE.PMT(rate=0, nper=10, pv=100)", -10.0),
    ] {
        close(formula, expected);
    }
}

#[test]
fn receiver_binding_refuses_duplicates_and_keeps_argument_validation() {
    for source in [
        "principal = 100\nprincipal.finance.pmt(0, 10, pv=200)",
        "principal = 100\nprincipal.finance.pmt(0)",
        "principal = 100\nprincipal.finance.pmt(0, 10, 0, 0, 0)",
        "principal = 100\nprincipal.finance.fv(0, 10)",
        "finance.irr([1, 2])",
        "[1, 2].finance.irr()",
    ] {
        assert!(evaluate(source).error.is_some(), "{source}");
    }
}

#[test]
fn namespace_completion_has_receiver_specific_signatures() {
    let store = Store::new(Document::demo());
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|o| {
            if let DataObject::Frame(f) = o {
                Some(f)
            } else {
                None
            }
        })
        .unwrap();
    let complete = |source: &str| store.complete_formula(&frame.id, source, source.chars().count());
    for (source, expected) in [
        ("fin", "namespace.finance"),
        ("finance.PM", "namespace.finance.pmt"),
        ("(100).", "namespace.finance"),
        ("(100).finance.PM", "finance.pmt"),
        ("(100).FINANCE.PM", "finance.pmt"),
    ] {
        assert!(
            complete(source)
                .suggestions
                .iter()
                .any(|s| s.id == expected),
            "{source}"
        );
    }
    assert!(
        !complete("\"text\".")
            .suggestions
            .iter()
            .any(|s| s.id == "namespace.finance")
    );
    for (source, expected) in [
        ("finance.pmt(0, ", "namespace.finance.pmt"),
        ("(100).finance.pmt(0, ", "finance.pmt"),
    ] {
        let result = complete(source);
        assert_eq!(
            result.active_function_id.as_deref(),
            Some(expected),
            "{source}"
        );
        assert_eq!(result.active_argument, Some(1));
    }
    let catalog = formula_function_catalog();
    let method = catalog.iter().find(|f| f.id == "finance.pmt").unwrap();
    assert_eq!(method.signature, ".finance.pmt(rate, nper, fv=0, type=0)");
    assert_eq!(method.minimum_arguments, 2);
    assert_eq!(method.maximum_arguments, 4);
    let fv = catalog.iter().find(|f| f.id == "finance.fv").unwrap();
    assert_eq!(fv.minimum_arguments, 3);
    assert_eq!(
        fv.arguments
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["rate", "nper", "pmt", "type"]
    );
}
