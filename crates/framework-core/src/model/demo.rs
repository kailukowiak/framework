//! The document a fresh install opens. Not a test fixture — `Store::load_or_demo`
//! falls back to it whenever there is nothing to load.

use crate::*;
use std::collections::BTreeMap;

impl Document {
    pub fn demo() -> Self {
        let tax_id = id();
        let block_id = id();
        let frame_id = id();
        let quantity_id = column_id("Quantity");
        let price_id = column_id("Unit price");
        let total_id = column_id("Total");

        let rows = [("3", "14.00"), ("5", "7.50"), ("2", "28.00")]
            .into_iter()
            .map(|(quantity, price)| Row {
                id: id(),
                cells: BTreeMap::from([
                    (
                        quantity_id.clone(),
                        Cell {
                            raw: quantity.into(),
                            ..Cell::default()
                        },
                    ),
                    (
                        price_id.clone(),
                        Cell {
                            raw: price.into(),
                            ..Cell::default()
                        },
                    ),
                    (total_id.clone(), Cell::default()),
                ]),
            })
            .collect();

        let total_formula = Formula {
            expression: Expr::Binary {
                operator: BinaryOperator::Multiply,
                left: Box::new(Expr::Binary {
                    operator: BinaryOperator::Multiply,
                    left: Box::new(Expr::Column {
                        column_id: quantity_id.clone(),
                    }),
                    right: Box::new(Expr::Column {
                        column_id: price_id.clone(),
                    }),
                }),
                right: Box::new(Expr::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expr::Integer { value: 1 }),
                    right: Box::new(Expr::Value {
                        object_id: tax_id.clone(),
                    }),
                }),
            },
        };

        let keyed_frame = |name: &str, grid: Vec<Vec<&str>>, x: f64, y: f64| {
            let (mut frame, mut view) = Self::build_frame(
                name.into(),
                grid.into_iter()
                    .map(|row| row.into_iter().map(str::to_string).collect())
                    .collect(),
                x,
                y,
            );
            frame.unique_keys.push(UniqueKeyConstraint {
                id: id(),
                column_ids: vec![frame.columns[0].id.clone()],
            });
            view.width = view.width.min(820.0);
            (frame, view)
        };
        let (sales, sales_view) = keyed_frame(
            "Transactions",
            vec![
                vec![
                    "Sale ID",
                    "Customer ID",
                    "Product ID",
                    "Rep ID",
                    "Sold on",
                    "Units",
                ],
                vec!["S-1001", "C-100", "P-10", "R-1", "2026-07-02", "3"],
                vec!["S-1002", "C-200", "P-20", "R-2", "2026-07-05", "5"],
                vec!["S-1003", "C-100", "P-30", "R-1", "2026-07-09", "2"],
                vec!["S-1004", "C-300", "P-10", "R-3", "2026-07-13", "7"],
                vec!["S-1005", "C-999", "P-40", "R-2", "2026-07-18", "4"],
                vec!["S-1006", "C-400", "P-999", "R-4", "2026-07-22", "1"],
            ],
            80.0,
            470.0,
        );
        let (customers, customers_view) = keyed_frame(
            "Customers",
            vec![
                vec!["Customer ID", "Customer", "Region code", "Segment"],
                vec!["C-100", "Northwind Studio", "AB-N", "Enterprise"],
                vec!["C-200", "Prairie Goods", "AB-S", "Growth"],
                vec!["C-300", "Coastal Labs", "BC-S", "Enterprise"],
                vec!["C-400", "Aurora Market", "AB-N", "Small business"],
                vec!["C-500", "Summit Supply", "BC-S", "Growth"],
            ],
            980.0,
            80.0,
        );
        let (products, products_view) = keyed_frame(
            "Products",
            vec![
                vec!["Product ID", "Product", "Category", "List price"],
                vec!["P-10", "Field notebook", "Stationery", "$14.00"],
                vec!["P-20", "Canvas tote", "Accessories", "$7.50"],
                vec!["P-30", "Desk lamp", "Home office", "$28.00"],
                vec!["P-40", "Travel mug", "Accessories", "$19.00"],
                vec!["P-50", "Cable organizer", "Home office", "$11.00"],
            ],
            980.0,
            410.0,
        );
        let (sales_reps, sales_reps_view) = keyed_frame(
            "Sales reps",
            vec![
                vec!["Rep ID", "Rep", "Region code"],
                vec!["R-1", "Maya Chen", "AB-N"],
                vec!["R-2", "Noah Williams", "AB-S"],
                vec!["R-3", "Sofia Patel", "BC-S"],
            ],
            80.0,
            830.0,
        );
        let (regions, regions_view) = keyed_frame(
            "Regions",
            vec![
                vec!["Region code", "Region", "Manager", "Quarter target"],
                vec!["AB-N", "Alberta North", "Riley Brooks", "$125000"],
                vec!["AB-S", "Alberta South", "Taylor Singh", "$110000"],
                vec!["BC-S", "British Columbia South", "Jordan Lee", "$145000"],
                vec!["SK-C", "Saskatchewan Central", "Morgan Reed", "$90000"],
            ],
            980.0,
            740.0,
        );

        Self {
            id: id(),
            name: "Commerce join playground".into(),
            revision: 0,
            frozen_values: std::collections::BTreeMap::new(),
            objects: vec![
                // The one assumption this playground makes, written where an
                // assumption goes: a line of a block. A card holding `5%` and
                // nothing else is what the block exists to replace, and the
                // column below reads the line by its id either way.
                DataObject::Block(BlockObject {
                    id: block_id.clone(),
                    name: "Assumptions".into(),
                    lines: vec![demo_tax_line(tax_id.clone())],
                }),
                DataObject::Frame(FrameObject {
                    comment: None,
                    id: frame_id.clone(),
                    name: "Orders".into(),
                    columns: vec![
                        Column {
                            id: quantity_id,
                            name: "Quantity".into(),
                            source_name: None,
                            data_type: DataType::Number,
                            categories: Vec::new(),
                            format: None,
                            formula: None,
                        },
                        Column {
                            id: price_id,
                            name: "Unit price".into(),
                            source_name: None,
                            data_type: DataType::Currency,
                            categories: Vec::new(),
                            format: None,
                            formula: None,
                        },
                        Column {
                            id: total_id.clone(),
                            name: "Total".into(),
                            source_name: None,
                            data_type: DataType::Currency,
                            categories: Vec::new(),
                            format: None,
                            formula: Some(total_formula),
                        },
                    ],
                    rows,
                    steps: Vec::new(),
                    display: FrameDisplay::default(),
                    base_columns: Vec::new(),
                    source_file: None,
                    artifact: None,
                    connector: None,
                    derivation: None,
                    generator: None,
                    entry_columns: Vec::new(),
                    materialization: None,
                    unique_keys: Vec::new(),
                    summaries: vec![Summary {
                        id: id(),
                        column_id: total_id,
                        operation: SummaryOperation::Sum,
                        label: "Total".into(),
                    }],
                }),
                DataObject::Frame(sales),
                DataObject::Frame(customers),
                DataObject::Frame(products),
                DataObject::Frame(sales_reps),
                DataObject::Frame(regions),
            ],
            views: vec![
                CanvasView {
                    id: id(),
                    object_id: block_id,
                    x: 80.0,
                    y: 90.0,
                    width: 260.0,
                    height: 140.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                CanvasView {
                    id: id(),
                    object_id: frame_id,
                    x: 360.0,
                    y: 90.0,
                    width: 600.0,
                    height: 320.0,
                    collapsed: false,
                    tab_object_ids: Vec::new(),
                },
                sales_view,
                customers_view,
                products_view,
                sales_reps_view,
                regions_view,
            ],
        }
    }
}

fn demo_tax_line(id: Id) -> BlockLine {
    BlockLine {
        id,
        name: "Tax rate".into(),
        named: true,
        name_quoted: false,
        source: "5%".into(),
        formula: Some(Formula {
            expression: Expr::Percentage { value: 0.05 },
        }),
        error: None,
    }
}
