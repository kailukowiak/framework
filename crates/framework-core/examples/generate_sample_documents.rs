use framework_core::{
    CanvasView, Cell, CollaborationPaths, Column, DataObject, DataType, Document, FrameDisplay,
    FrameObject, Operation, Row, Store, TextObject, UniqueKeyConstraint, column_id,
    create_excel_range_artifact, inspect_excel_workbook,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn frame(name: &str, columns: &[(&str, DataType)], rows: Vec<Vec<String>>) -> FrameObject {
    let columns = columns
        .iter()
        .map(|(name, data_type)| Column {
            id: column_id(name),
            name: (*name).into(),
            source_name: None,
            data_type: *data_type,
            categories: Vec::new(),
            format: None,
            formula: None,
        })
        .collect::<Vec<_>>();
    let rows = rows
        .into_iter()
        .map(|values| Row {
            id: id(),
            cells: columns
                .iter()
                .zip(values)
                .map(|(column, raw)| {
                    (
                        column.id.clone(),
                        Cell {
                            raw,
                            ..Cell::default()
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        })
        .collect();
    FrameObject {
        comment: None,
        id: id(),
        name: name.into(),
        unique_keys: vec![UniqueKeyConstraint {
            id: id(),
            column_ids: vec![columns[0].id.clone()],
        }],
        columns,
        rows,
        steps: Vec::new(),
        display: FrameDisplay::default(),
        base_columns: Vec::new(),
        source_file: None,
        artifact: None,
        connector: None,
        derivation: None,
        generator: None,
        materialization: None,
        entry_columns: Vec::new(),
        summaries: Vec::new(),
    }
}

fn document(name: &str, note: &str, frames: Vec<FrameObject>) -> Document {
    let note_object = TextObject {
        id: id(),
        name: "About this dataset".into(),
        text: note.into(),
        segments: Vec::new(),
    };
    let mut objects = vec![DataObject::Text(note_object.clone())];
    let mut views = vec![CanvasView {
        id: id(),
        object_id: note_object.id,
        x: 80.0,
        y: 50.0,
        width: 1_330.0,
        height: 92.0,
        collapsed: false,
        tab_object_ids: Vec::new(),
    }];
    for (index, frame) in frames.into_iter().enumerate() {
        views.push(CanvasView {
            id: id(),
            object_id: frame.id.clone(),
            x: 80.0 + (index % 2) as f64 * 700.0,
            y: 175.0 + (index / 2) as f64 * 390.0,
            width: 650.0,
            height: 335.0,
            collapsed: false,
            tab_object_ids: Vec::new(),
        });
        objects.push(DataObject::Frame(frame));
    }
    Document {
        id: id(),
        name: name.into(),
        revision: 0,
        objects,
        views,
        frozen_values: Default::default(),
    }
}

fn anscombe() -> Document {
    let common_x = [10., 8., 13., 9., 11., 14., 6., 4., 12., 7., 5.];
    let values = [
        (
            "I",
            common_x,
            [
                8.04, 6.95, 7.58, 8.81, 8.33, 9.96, 7.24, 4.26, 10.84, 4.82, 5.68,
            ],
        ),
        (
            "II",
            common_x,
            [
                9.14, 8.14, 8.74, 8.77, 9.26, 8.10, 6.13, 3.10, 9.13, 7.26, 4.74,
            ],
        ),
        (
            "III",
            common_x,
            [
                7.46, 6.77, 12.74, 7.11, 7.81, 8.84, 6.08, 5.39, 8.15, 6.42, 5.73,
            ],
        ),
        (
            "IV",
            [8., 8., 8., 8., 8., 8., 8., 19., 8., 8., 8.],
            [
                6.58, 5.76, 7.71, 8.84, 8.47, 7.04, 5.25, 12.50, 5.56, 7.91, 6.89,
            ],
        ),
    ];
    let observations = values
        .into_iter()
        .flat_map(|(series, xs, ys)| {
            xs.into_iter()
                .zip(ys)
                .enumerate()
                .map(move |(index, (x, y))| {
                    vec![
                        format!("{series}-{:02}", index + 1),
                        series.into(),
                        x.to_string(),
                        y.to_string(),
                    ]
                })
        })
        .collect();
    document(
        "Anscombe's quartet",
        "Canonical 1973 dataset by Francis Anscombe. Four series share nearly identical summary statistics but look very different when plotted. Stored in long form with a Series lookup so it is also useful for join testing.",
        vec![
            frame(
                "Observations",
                &[
                    ("Observation ID", DataType::String),
                    ("Series", DataType::String),
                    ("X", DataType::Number),
                    ("Y", DataType::Number),
                ],
                observations,
            ),
            frame(
                "Series",
                &[
                    ("Series", DataType::String),
                    ("Visual pattern", DataType::String),
                ],
                vec![
                    vec!["I".into(), "roughly linear".into()],
                    vec!["II".into(), "curved".into()],
                    vec!["III".into(), "linear with an outlier".into()],
                    vec!["IV".into(), "vertical cluster with a leverage point".into()],
                ],
            ),
        ],
    )
}

fn ucb_admissions() -> Document {
    let counts = [
        ("A", 512, 313, 89, 19),
        ("B", 353, 207, 17, 8),
        ("C", 120, 205, 202, 391),
        ("D", 138, 279, 131, 244),
        ("E", 53, 138, 94, 299),
        ("F", 22, 351, 24, 317),
    ];
    let mut rows = Vec::new();
    for (department, admitted_male, rejected_male, admitted_female, rejected_female) in counts {
        for (gender, outcome, count) in [
            ("Male", "Admitted", admitted_male),
            ("Male", "Rejected", rejected_male),
            ("Female", "Admitted", admitted_female),
            ("Female", "Rejected", rejected_female),
        ] {
            rows.push(vec![
                format!("{department}-{gender}-{outcome}"),
                department.into(),
                gender.into(),
                outcome.into(),
                count.to_string(),
            ]);
        }
    }
    document(
        "UC Berkeley admissions 1973",
        "Canonical aggregated admissions dataset distributed with R as UCBAdmissions and commonly used to demonstrate Simpson's paradox. Department descriptions are convenience labels added for this sample.",
        vec![
            frame(
                "Admissions",
                &[
                    ("Row ID", DataType::String),
                    ("Department", DataType::String),
                    ("Gender", DataType::String),
                    ("Outcome", DataType::String),
                    ("Applicants", DataType::Number),
                ],
                rows,
            ),
            frame(
                "Departments",
                &[
                    ("Department", DataType::String),
                    ("Selectivity band", DataType::String),
                ],
                vec![
                    vec!["A".into(), "Very high".into()],
                    vec!["B".into(), "High".into()],
                    vec!["C".into(), "Moderate".into()],
                    vec!["D".into(), "Moderate".into()],
                    vec!["E".into(), "Low".into()],
                    vec!["F".into(), "Low".into()],
                ],
            ),
        ],
    )
}

#[derive(Clone)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0
    }

    fn index(&mut self, length: usize) -> usize {
        (self.next() as usize) % length
    }

    fn range(&mut self, start: u64, end: u64) -> u64 {
        start + self.next() % (end - start)
    }
}

fn sample_date(index: usize) -> String {
    format!("2026-{:02}-{:02}", 1 + (index / 28) % 8, 1 + index % 28)
}

fn subscription_analytics() -> Document {
    let plans = [
        ("PL-B", "Basic", 29),
        ("PL-G", "Growth", 79),
        ("PL-S", "Scale", 199),
    ];
    let mut rng = DeterministicRng(0x5aa5_2026);
    let accounts = (1..=48)
        .map(|index| {
            vec![
                format!("AC-{index:03}"),
                format!("Account {index:02}"),
                ["Self-serve", "Sales-assisted", "Partner"][rng.index(3)].into(),
                ["Canada", "United States", "United Kingdom", "Australia"][rng.index(4)].into(),
            ]
        })
        .collect::<Vec<_>>();
    let subscriptions = (1..=64)
        .map(|index| {
            let plan = plans[rng.index(plans.len())];
            vec![
                format!("SU-{index:04}"),
                format!("AC-{:03}", 1 + rng.index(48)),
                plan.0.into(),
                ["Active", "Active", "Active", "Paused", "Cancelled"][rng.index(5)].into(),
                plan.2.to_string(),
                sample_date(rng.index(190)),
            ]
        })
        .collect::<Vec<_>>();
    let payments = (1..=140)
        .map(|index| {
            vec![
                format!("PY-{index:04}"),
                format!("SU-{:04}", 1 + rng.index(68)),
                sample_date(rng.index(210)),
                ["29", "79", "199"][rng.index(3)].into(),
                ["Paid", "Paid", "Paid", "Failed"][rng.index(4)].into(),
            ]
        })
        .collect::<Vec<_>>();
    document(
        "Synthetic subscription analytics",
        "Deterministically generated SaaS accounts, plans, subscriptions, and payments. A few payments intentionally reference missing subscriptions so left and inner joins produce different results.",
        vec![
            frame(
                "Accounts",
                &[
                    ("Account ID", DataType::String),
                    ("Account", DataType::String),
                    ("Acquisition", DataType::String),
                    ("Country", DataType::String),
                ],
                accounts,
            ),
            frame(
                "Plans",
                &[
                    ("Plan ID", DataType::String),
                    ("Plan", DataType::String),
                    ("Monthly price", DataType::Currency),
                ],
                plans
                    .into_iter()
                    .map(|(id, name, price)| vec![id.into(), name.into(), price.to_string()])
                    .collect(),
            ),
            frame(
                "Subscriptions",
                &[
                    ("Subscription ID", DataType::String),
                    ("Account ID", DataType::String),
                    ("Plan ID", DataType::String),
                    ("Status", DataType::String),
                    ("MRR", DataType::Currency),
                    ("Started on", DataType::Date),
                ],
                subscriptions,
            ),
            frame(
                "Payments",
                &[
                    ("Payment ID", DataType::String),
                    ("Subscription ID", DataType::String),
                    ("Paid on", DataType::Date),
                    ("Amount", DataType::Currency),
                    ("Status", DataType::String),
                ],
                payments,
            ),
        ],
    )
}

fn support_operations() -> Document {
    let agents = [
        ("AG-1", "Maya Chen", "Billing"),
        ("AG-2", "Noah Williams", "Technical"),
        ("AG-3", "Sofia Patel", "Onboarding"),
        ("AG-4", "Jordan Lee", "Technical"),
    ];
    let mut rng = DeterministicRng(0x51a_2026);
    let customers = (1..=36)
        .map(|index| {
            vec![
                format!("CU-{index:03}"),
                format!("Customer {index:02}"),
                ["SMB", "Mid-market", "Enterprise"][rng.index(3)].into(),
            ]
        })
        .collect::<Vec<_>>();
    let tickets = (1..=96)
        .map(|index| {
            let priority = ["P1", "P2", "P3", "P4"][rng.index(4)];
            vec![
                format!("TK-{index:04}"),
                format!("CU-{:03}", 1 + rng.index(39)),
                agents[rng.index(agents.len())].0.into(),
                priority.into(),
                ["Open", "Pending", "Resolved", "Resolved", "Closed"][rng.index(5)].into(),
                sample_date(rng.index(210)),
                rng.range(1, 96).to_string(),
                rng.range(1, 6).to_string(),
            ]
        })
        .collect::<Vec<_>>();
    document(
        "Synthetic support operations",
        "Deterministically generated support tickets with customer, agent, and SLA lookup frames. Some tickets contain missing customer keys to make join diagnostics visible.",
        vec![
            frame(
                "Tickets",
                &[
                    ("Ticket ID", DataType::String),
                    ("Customer ID", DataType::String),
                    ("Agent ID", DataType::String),
                    ("Priority", DataType::String),
                    ("Status", DataType::String),
                    ("Opened on", DataType::Date),
                    ("Resolution hours", DataType::Number),
                    ("CSAT", DataType::Number),
                ],
                tickets,
            ),
            frame(
                "Customers",
                &[
                    ("Customer ID", DataType::String),
                    ("Customer", DataType::String),
                    ("Segment", DataType::String),
                ],
                customers,
            ),
            frame(
                "Agents",
                &[
                    ("Agent ID", DataType::String),
                    ("Agent", DataType::String),
                    ("Team", DataType::String),
                ],
                agents
                    .into_iter()
                    .map(|(agent_id, agent, team)| {
                        vec![agent_id.to_string(), agent.to_string(), team.to_string()]
                    })
                    .collect(),
            ),
            frame(
                "SLA policies",
                &[
                    ("Priority", DataType::String),
                    ("Target hours", DataType::Number),
                ],
                vec![
                    vec!["P1".into(), "4".into()],
                    vec!["P2".into(), "12".into()],
                    vec!["P3".into(), "36".into()],
                    vec!["P4".into(), "72".into()],
                ],
            ),
        ],
    )
}

fn write_sample(
    root: &Path,
    relative: &str,
    document: Document,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(relative);
    Store::new(document).save(&path)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Keep the workbook and the FrameWork result beside one another in the
/// sample source, but make the opened document self-contained. Each range
/// travels through the same cached-value → CSV normalization → Parquet path
/// as the desktop command; this is not a hand-built imitation of an import.
fn write_excel_import_sample(
    root: &Path,
    workbook_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("synthetic/excel-import-workbook.fw");
    let workbook = inspect_excel_workbook(workbook_path)?;
    if workbook.sheets.len() != 2 || workbook.tables.len() != 3 {
        return Err(std::io::Error::other(format!(
            "Excel import demo must contain 2 sheets and 3 tables, found {} and {}",
            workbook.sheets.len(),
            workbook.tables.len()
        ))
        .into());
    }
    let mut store = Store::new(document(
        "Excel import workbook",
        "Imported from one two-sheet XLSX workbook. Operations contains the Inventory and Suppliers Excel Tables; Sales contains Orders. The frames hold cached values only, including the Revenue formula results — no Excel formula or formatting entered the FrameWork document.",
        Vec::new(),
    ));
    let data_directory = CollaborationPaths::for_document(&path, store.document_id())?
        .root
        .join("data");
    for (name, sheet, range, x, y) in [
        ("Inventory", "Operations", "A4:F10", 80.0, 175.0),
        ("Suppliers", "Operations", "H4:L8", 760.0, 175.0),
        ("Orders", "Sales", "B5:I25", 80.0, 565.0),
    ] {
        let (artifact, _) =
            create_excel_range_artifact(workbook_path, &data_directory, sheet, range, true)?;
        store.apply(Operation::ImportFrameFromArtifact {
            name: name.into(),
            artifact,
            connector: None,
            x,
            y,
        })?;
    }
    store.save(&path)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let root = workspace_root.join(".framework-samples");
    // Generated artifact-backed samples mint document ids, so their Parquet
    // directories cannot be overwritten in place. This library is entirely
    // regenerated by this command; clear its prior sidecars so every e2e
    // preparation does not leave another unreachable copy behind.
    let synthetic_artifacts = root.join("synthetic/.framework");
    if synthetic_artifacts.exists() {
        fs::remove_dir_all(synthetic_artifacts)?;
    }
    fs::create_dir_all(root.join("canonical"))?;
    fs::create_dir_all(root.join("synthetic"))?;

    println!("FrameWork sample library: {}", root.display());
    println!("Open these from Data → Sample FrameWork documents in the desktop app.\n");

    write_sample(&root, "canonical/anscombe-quartet.fw", anscombe())?;
    write_sample(&root, "canonical/ucb-admissions-1973.fw", ucb_admissions())?;
    write_sample(
        &root,
        "synthetic/commerce-join-playground.fw",
        Document::demo(),
    )?;
    write_sample(
        &root,
        "synthetic/subscription-analytics.fw",
        subscription_analytics(),
    )?;
    write_sample(
        &root,
        "synthetic/support-operations.fw",
        support_operations(),
    )?;
    write_excel_import_sample(
        &root,
        &workspace_root.join("examples/datasets/excel-import-demo.xlsx"),
    )?;

    fs::write(
        root.join("README.md"),
        "# Local FrameWork sample library\n\nGenerated with `cargo run -p framework-core --example generate_sample_documents`.\n\n- `canonical/` contains well-known published datasets, reshaped only to make relationships explicit.\n- `synthetic/` contains deterministic, fictional business data, including a three-table Excel import example.\n",
    )?;
    Ok(())
}
