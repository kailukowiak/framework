use super::*;

const LOAN_TERMS: &str = "principal = 400000\nrate = 0.06\nterm = 60";

pub(super) fn generate_price_a_deal(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("Price a deal"));
    add_walkthrough(
        &mut store,
        include_str!("../../../../tutorials/price-a-deal/README.md"),
    )?;
    store.apply(Operation::AddFrame {
        name: "Cash flows".into(),
        grid: grid(&[
            &["Date", "Amount"],
            &["2026-01-15", "-250000"],
            &["2026-06-30", "40000"],
            &["2026-12-31", "60000"],
            &["2027-06-30", "70000"],
            &["2027-12-31", "80000"],
            &["2028-06-30", "90000"],
        ]),
        x: 720.0,
        y: 70.0,
    })?;
    set_column_type(&mut store, "Cash flows", "Amount", DataType::Currency)?;
    format_columns(&mut store, "Cash flows", &["Amount"], money_format())?;
    let flows_id = frame(&store, "Cash flows").id;
    let loan_y = 70.0 + view_height(&store, &flows_id) + 40.0;
    add_block(&mut store, "Loan terms", 720.0, loan_y, LOAN_TERMS)?;
    let start = output.join("price-a-deal-start.fw");
    store.save(&start)?;

    // Section 1: the payment from the financial catalog.
    let with_payment = format!(
        "{LOAN_TERMS}\nmonthly = rate / 12\npayment = (-principal).finance.pmt(monthly, term)"
    );
    set_block(&mut store, "Loan terms", &with_payment)?;

    // Section 2: the schedule as a recurrence over a generated spine.
    store.apply(Operation::AddGeneratorFrame {
        name: "Schedule".into(),
        formula: "sequence(1, 61)".into(),
        column_name: Some("Period".into()),
        x: 1410.0,
        y: 70.0,
    })?;
    let schedule = frame(&store, "Schedule");
    store.apply(Operation::SetFramePipeline {
        frame_id: schedule.id.clone(),
        steps: vec![
            sort_by(&schedule, "Period"),
            with_columns(&[(
                "Opening",
                "recur(`Loan terms`.principal, previous() * (1 + `Loan terms`.monthly) - `Loan terms`.payment)",
            )]),
            with_columns(&[("Interest", "(-`Loan terms`.principal).finance.ipmt(`Loan terms`.monthly, `Period`, `Loan terms`.term)")]),
            with_columns(&[("Principal paid", "(-`Loan terms`.principal).finance.ppmt(`Loan terms`.monthly, `Period`, `Loan terms`.term)")]),
            with_columns(&[("Closing", "`Opening` - `Principal paid`")]),
        ],
    })?;
    format_columns(
        &mut store,
        "Schedule",
        &["Opening", "Interest", "Principal paid", "Closing"],
        money_format(),
    )?;
    set_block(
        &mut store,
        "Loan terms",
        &format!("{with_payment}\ntotal interest = `Schedule`.`Interest`.sum()"),
    )?;

    // Keep the rate scan as an independent way to inspect the solved return,
    // and feed that return back to XNPV so the visible residual checks it.
    let deal_y = loan_y + view_height(&store, &object_id_named(&store, "Loan terms")) + 40.0;
    add_block(
        &mut store,
        "Deal",
        720.0,
        deal_y,
        "rate = 0.10\nnpv = `Cash flows`.`Amount`.finance.xnpv(rate, `Cash flows`.`Date`)\nirr = `Cash flows`.`Amount`.finance.xirr(`Cash flows`.`Date`)\nresidual = `Cash flows`.`Amount`.finance.xnpv(irr, `Cash flows`.`Date`)",
    )?;

    let scan_y = deal_y + view_height(&store, &object_id_named(&store, "Deal")) + 40.0;
    let scan = [5, 10, 15, 20, 25, 30]
        .iter()
        .map(|rate| {
            format!(
                "`NPV at {rate}%` = `Cash flows`.`Amount`.finance.xnpv(0.{rate:02}, `Cash flows`.`Date`)"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    add_block(&mut store, "Rate scan", 720.0, scan_y, &scan)?;

    let finished = output.join("price-a-deal-finished.fw");
    store.save(&finished)?;

    let reloaded = Store::load(&finished)?;
    dump(
        &reloaded,
        &["Cash flows", "Schedule"],
        &["Loan terms", "Deal", "Rate scan"],
        &[],
    );
    assert_block_close(
        &reloaded,
        "Loan terms",
        &[400000.0, 0.06, 60.0, 0.005, 7733.12, 63987.24],
    );
    let schedule = page(&reloaded, "Schedule");
    assert_eq!(schedule.total_rows, 60);
    assert_cell_close(&schedule, "1", "Opening", 400000.0);
    assert_cell_close(&schedule, "1", "Interest", 2000.0);
    assert_cell_close(&schedule, "1", "Principal paid", 5733.12);
    assert_cell_close(&schedule, "1", "Closing", 394266.88);
    assert_cell_close(&schedule, "2", "Opening", 394266.88);
    assert_cell_close(&schedule, "60", "Closing", 0.0);
    // Independent 60-digit Decimal bisection of the dated cash-flow equation
    // gives 0.21347541804480068637; do not derive this target from the solver.
    assert_block_close(
        &reloaded,
        "Deal",
        &[0.10, 41581.08, 0.2134754180448007, 0.0],
    );
    assert_block_close(
        &reloaded,
        "Rate scan",
        &[64121.94, 41581.08, 21809.34, 4355.85, -11141.06, -24974.32],
    );
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}
