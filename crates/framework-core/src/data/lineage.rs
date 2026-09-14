//! Rows for the "How it was computed" sheet: an optional, opt-in tab
//! appended to an exported workbook that says, for every column and every
//! named value it hands over, where the number came from.
//!
//! This is deliberately built from the same rendered text the interface
//! already shows — `ComputedFrame::formulas`, `ComputedFrame::steps`, a
//! result's `ComputedResult::formula` — rather than a second formula
//! renderer. Two renderers of the same expression tree are two chances to
//! disagree about what a person actually typed.

use crate::*;
use std::collections::HashMap;
use std::collections::HashSet;

/// One row of the sheet. `type_name` and `reads` are blank (rendered as an
/// empty cell) rather than an empty string in the workbook when they do not
/// apply — a Wrangle step has no type of its own, and plenty of formulas
/// name nothing else on the canvas.
pub(crate) struct LineageRow {
    pub(crate) frame: String,
    pub(crate) column: String,
    pub(crate) type_name: String,
    pub(crate) computed_as: String,
    pub(crate) reads: String,
}

pub(crate) fn header_row() -> Vec<String> {
    vec![
        "Frame".into(),
        "Column".into(),
        "Type".into(),
        "Computed as".into(),
        "Reads".into(),
    ]
}

/// The app's own words for a column's type — the same labels the frame
/// inspector's type picker shows, so the sheet reads like the product
/// rather than like the enum's Rust spelling.
pub(crate) fn data_type_label(data_type: DataType) -> &'static str {
    match data_type {
        DataType::String => "Text",
        DataType::Categorical => "Categorical",
        DataType::Integer => "Integer",
        DataType::Number => "Number",
        DataType::Currency => "Currency",
        DataType::Accounting => "Accounting",
        DataType::Percentage => "Percentage",
        DataType::Boolean => "Boolean",
        DataType::Date => "Date",
    }
}

/// Where a step's own output columns land, keyed by column id — everything
/// that is neither a direct column formula nor a still-visible `withColumns`
/// output (both of which `ComputedFrame::formulas` already covers) but is
/// still something a step produced rather than something typed in: a
/// `summarize` aggregate, a pivot's value column, an unpivot's name/value
/// pair, an expansion's carried field, a join's lookup output, a union's
/// stacked column, or a zipped vector.
enum StepOrigin {
    /// Produced by `chain_steps[_0]`, in lockstep with `computed.steps[_0]`.
    Chain(usize),
    /// Produced by the derivation's join, which is not one of `chain_steps`
    /// when it is the derivation's sole input rather than a step alongside
    /// others — see [`frame_lineage_rows`].
    Join,
}

fn step_origins(frame: &FrameObject, chain_steps: &[FrameStep]) -> HashMap<Id, StepOrigin> {
    let mut origins = HashMap::new();
    for (index, step) in chain_steps.iter().enumerate() {
        match step {
            FrameStep::Summarize {
                group_keys,
                aggregates,
                ..
            } => {
                for item in group_keys.iter().chain(aggregates.iter()) {
                    origins.insert(item.output_column_id.clone(), StepOrigin::Chain(index));
                }
            }
            FrameStep::Pivot { outputs, .. } => {
                for output in outputs {
                    origins.insert(output.output_column_id.clone(), StepOrigin::Chain(index));
                }
            }
            FrameStep::Unpivot {
                name_column_id,
                value_column_id,
                ..
            } => {
                origins.insert(name_column_id.clone(), StepOrigin::Chain(index));
                origins.insert(value_column_id.clone(), StepOrigin::Chain(index));
            }
            FrameStep::Expand { outputs, .. } => {
                for output in outputs {
                    origins.insert(output.output_column_id.clone(), StepOrigin::Chain(index));
                }
            }
            FrameStep::Join { join } => {
                for output in &join.outputs {
                    origins.insert(output.output_column_id.clone(), StepOrigin::Chain(index));
                }
            }
            FrameStep::Union { mapping, .. } => {
                for column in mapping {
                    origins.insert(column.column_id.clone(), StepOrigin::Chain(index));
                }
            }
            FrameStep::ZipVector {
                output_column_id, ..
            } => {
                origins.insert(output_column_id.clone(), StepOrigin::Chain(index));
            }
            _ => {}
        }
    }
    // A join held flat on the derivation, rather than as a step of its own,
    // never appears in `chain_steps` — see the prefix-stripping in
    // `frame_lineage_rows`. Its outputs still need somewhere to point.
    if let Some(join) = frame.derivation.as_ref().and_then(|d| d.join.as_ref()) {
        for output in &join.outputs {
            origins
                .entry(output.output_column_id.clone())
                .or_insert(StepOrigin::Join);
        }
    }
    origins
}

/// The column's name as this frame currently spells it, or as its own input
/// schema did, or its bare id when neither knows it — the id can outlive its
/// name when a later step drops the column before naming it.
fn column_name(frame: &FrameObject, column_id: &str) -> String {
    frame
        .columns
        .iter()
        .find(|column| column.id == column_id)
        .or_else(|| {
            frame
                .input_columns()
                .iter()
                .find(|column| column.id == column_id)
        })
        .map(|column| column.name.clone())
        .unwrap_or_else(|| column_id.to_string())
}

fn frame_name(document: &Document, frame_id: &str) -> String {
    document
        .frame(frame_id)
        .map(|frame| frame.name.clone())
        .unwrap_or_else(|_| frame_id.to_string())
}

fn foreign_column_name(document: &Document, frame_id: &str, column_id: &str) -> String {
    document
        .frame(frame_id)
        .ok()
        .and_then(|frame| {
            frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .map(|column| column.name.clone())
        })
        .unwrap_or_else(|| column_id.to_string())
}

fn pivot_aggregate_label(aggregate: PivotAggregate) -> &'static str {
    match aggregate {
        PivotAggregate::Sum => "sum",
        PivotAggregate::Count => "count",
        PivotAggregate::Mean => "mean",
        PivotAggregate::Min => "min",
        PivotAggregate::Max => "max",
        PivotAggregate::First => "first",
        PivotAggregate::None => "none",
    }
}

fn broadcast_operator_label(operator: BroadcastOperator) -> &'static str {
    match operator {
        BroadcastOperator::Multiply => "multiply",
        BroadcastOperator::Divide => "divide",
        BroadcastOperator::Add => "add",
        BroadcastOperator::Subtract => "subtract",
    }
}

fn join_description(frame: &FrameObject, document: &Document, join: &FrameJoin) -> String {
    format!(
        "join {} with \"{}\" on [{}] = [{}]",
        join.join_type.label(),
        frame_name(document, &join.lookup_frame_id),
        join.primary_key_column_ids
            .iter()
            .map(|id| column_name(frame, id))
            .collect::<Vec<_>>()
            .join(", "),
        join.lookup_key_column_ids
            .iter()
            .map(|id| foreign_column_name(document, &join.lookup_frame_id, id))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// `name = formula` for each entry, joined with `sep` — the shape both a
/// `withColumns` step and a `summarize` step's own key/aggregate lists
/// need, so it exists once rather than four times over.
fn named_formula_list(
    frame: &FrameObject,
    items: &[RenderedDerivedExpression],
    sep: &str,
) -> String {
    items
        .iter()
        .map(|item| {
            format!(
                "{} = {}",
                column_name(frame, &item.output_column_id),
                item.formula
            )
        })
        .collect::<Vec<_>>()
        .join(sep)
}

/// Column names, comma-joined — every step that lists columns by id
/// (`select`, `sort`, `broadcast`) rather than naming a formula for each.
fn column_name_list<'a>(frame: &FrameObject, ids: impl Iterator<Item = &'a Id>) -> String {
    ids.map(|id| column_name(frame, id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One line of the chain, in the same words the app already has for it:
/// each rendered formula it carries, spelled out rather than pointed at.
fn describe_rendered_step(
    frame: &FrameObject,
    document: &Document,
    step: &RenderedFrameStep,
) -> String {
    match step {
        RenderedFrameStep::Filter {
            predicates,
            match_all,
        } => {
            let joiner = if *match_all { " and " } else { " or " };
            format!("filter({})", predicates.join(joiner))
        }
        RenderedFrameStep::WithColumns { columns } => named_formula_list(frame, columns, "; "),
        RenderedFrameStep::Select { column_ids } => {
            format!("select({})", column_name_list(frame, column_ids.iter()))
        }
        RenderedFrameStep::Summarize {
            group_keys,
            aggregates,
            ..
        } => format!(
            "summarize(group by [{}], aggregates [{}])",
            named_formula_list(frame, group_keys, ", "),
            named_formula_list(frame, aggregates, ", "),
        ),
        RenderedFrameStep::Join { join } => join_description(frame, document, join),
        RenderedFrameStep::Sort { keys } => format!(
            "sort by {}",
            keys.iter()
                .map(|key| format!(
                    "{} {}",
                    column_name(frame, &key.column_id),
                    if key.descending { "desc" } else { "asc" }
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RenderedFrameStep::Union { frame_id, .. } => {
            format!("union with \"{}\"", frame_name(document, frame_id))
        }
        RenderedFrameStep::Expand { frame_id, .. } => {
            format!("expand across \"{}\"", frame_name(document, frame_id))
        }
        RenderedFrameStep::Pivot {
            names_column_id,
            values_column_id,
            aggregate,
            ..
        } => format!(
            "pivot(names={}, values={}, aggregate={})",
            column_name(frame, names_column_id),
            column_name(frame, values_column_id),
            pivot_aggregate_label(*aggregate),
        ),
        RenderedFrameStep::Unpivot {
            columns,
            name_column_name,
            value_column_name,
            ..
        } => format!(
            "unpivot [{}] into {}/{}",
            columns
                .iter()
                .map(|column| column.label.clone())
                .collect::<Vec<_>>()
                .join(", "),
            name_column_name,
            value_column_name,
        ),
        RenderedFrameStep::Broadcast {
            column_ids,
            vector,
            operator,
            ..
        } => format!(
            "broadcast {} {} across [{}]",
            broadcast_operator_label(*operator),
            vector,
            column_name_list(frame, column_ids.iter()),
        ),
        RenderedFrameStep::ZipVector {
            output_column_name,
            vector,
            ..
        } => format!("{output_column_name} = zip({vector})"),
        RenderedFrameStep::Comment { text } => text.clone(),
    }
}

/// A canvas object's name, the way the rest of the export names it — a
/// value or result gets its qualified path, a block line its
/// `Block.line` pair. `None` for anything else `Expr::Value` could in
/// principle point at, which in practice is nothing: only these three
/// kinds mint the ids that expression carries.
fn object_display_name(document: &Document, object_id: &str) -> Option<String> {
    match document.object(object_id) {
        Ok(DataObject::Value(value)) => {
            Some(document.qualified_export_name(&value.id, &value.name))
        }
        Ok(DataObject::Result(result)) => {
            Some(document.qualified_export_name(&result.id, &result.name))
        }
        Ok(_) => None,
        Err(_) => document
            .block_line(object_id)
            .map(|(block, index)| format!("{}.{}", block.name, block.lines[index].name)),
    }
}

fn reads_for_expressions(document: &Document, expressions: &[&Expr]) -> String {
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for expression in expressions {
        expression.walk_values(&mut |id| {
            if seen.insert(id)
                && let Some(name) = object_display_name(document, id)
            {
                names.push(name);
            }
        });
    }
    names.join(", ")
}

/// Every value, result, and block line one expression names — the same
/// service `excel_scalar_records` needs for a result or block line's own
/// "Reads", which is why this is public within the crate rather than
/// folded into the per-frame walk below.
pub(crate) fn expr_reads(document: &Document, expression: &Expr) -> String {
    reads_for_expressions(document, &[expression])
}

fn step_reads(document: &Document, step: &FrameStep) -> String {
    let expressions: Vec<&Expr> = match step {
        FrameStep::Filter { predicates, .. } => predicates.iter().collect(),
        FrameStep::WithColumns { columns } => {
            columns.iter().map(|column| &column.expression).collect()
        }
        FrameStep::Summarize {
            group_keys,
            aggregates,
            ..
        } => group_keys
            .iter()
            .chain(aggregates.iter())
            .map(|item| &item.expression)
            .collect(),
        FrameStep::Broadcast { vector, .. } | FrameStep::ZipVector { vector, .. } => vec![vector],
        _ => Vec::new(),
    };
    reads_for_expressions(document, &expressions)
}

/// The raw chain this frame's rendered `steps` were built from: its own
/// Wrangle steps when it owns them, or its derivation's, with the leading
/// join stripped when the join is the derivation's whole input rather than
/// one step among others — the same split [`FrameObject::compute`] makes,
/// reproduced here because the raw [`Expr`] trees it needs for "Reads" do
/// not survive into the rendered form.
fn chain_steps(frame: &FrameObject) -> Vec<FrameStep> {
    let Some(derivation) = &frame.derivation else {
        return frame.steps.clone();
    };
    let mut steps = derivation.steps().into_owned();
    if let Some(join) = &derivation.join
        && steps.first() == Some(&FrameStep::Join { join: join.clone() })
    {
        steps.remove(0);
    }
    steps
}

/// Every row this frame contributes to the "How it was computed" sheet:
/// one per column, in column order, then the frame's own lineage — where
/// its rows come from, and each Wrangle step in the order Wrangle runs it.
pub(crate) fn frame_lineage_rows(document: &Document, frame: &FrameObject) -> Vec<LineageRow> {
    let computed = frame.compute(document);
    let chain = chain_steps(frame);
    let origins = step_origins(frame, &chain);

    // Only `withColumns` feeds `ComputedFrame::formulas` — see
    // `rendered_column_formulas` in `engine/frame.rs` — so this mirrors
    // exactly the same restriction rather than a broader "every formula
    // anywhere in the chain" reading that the engine itself does not use.
    let mut with_columns_exprs: HashMap<&str, &Expr> = HashMap::new();
    for step in &chain {
        if let FrameStep::WithColumns { columns } = step {
            for column in columns {
                with_columns_exprs.insert(column.output_column_id.as_str(), &column.expression);
            }
        }
    }

    let mut rows = Vec::new();
    for column in &frame.columns {
        let (computed_as, reads) = if let Some(formula) = &column.formula {
            let text = computed
                .formulas
                .get(&column.id)
                .cloned()
                .unwrap_or_else(|| formula.expression.render(frame, document, 0));
            (text, expr_reads(document, &formula.expression))
        } else if let Some(text) = computed.formulas.get(&column.id) {
            let reads = with_columns_exprs
                .get(column.id.as_str())
                .map(|expression| expr_reads(document, expression))
                .unwrap_or_default();
            (text.clone(), reads)
        } else if let Some(origin) = origins.get(&column.id) {
            match origin {
                // `.get` rather than direct indexing: a derivation whose
                // source frame has since been deleted renders zero steps
                // (`ComputedFrame::steps` comes back empty) while the raw
                // chain this walk reconstructed independently still holds
                // them, and a broken document should export something
                // rather than panic the export.
                StepOrigin::Chain(index) => (
                    computed
                        .steps
                        .get(*index)
                        .map(|step| describe_rendered_step(frame, document, step))
                        .unwrap_or_else(|| "(unavailable)".to_string()),
                    chain
                        .get(*index)
                        .map(|step| step_reads(document, step))
                        .unwrap_or_default(),
                ),
                StepOrigin::Join => (
                    frame
                        .derivation
                        .as_ref()
                        .and_then(|derivation| derivation.join.as_ref())
                        .map(|join| join_description(frame, document, join))
                        .unwrap_or_default(),
                    String::new(),
                ),
            }
        } else if let Some(source) = &column.source_name {
            (format!("source field \"{source}\""), String::new())
        } else {
            ("typed in".to_string(), String::new())
        };
        rows.push(LineageRow {
            frame: frame.name.clone(),
            column: column.name.clone(),
            type_name: data_type_label(column.data_type).to_string(),
            computed_as,
            reads,
        });
    }

    if let Some(derivation) = &frame.derivation {
        let source = frame_name(document, &derivation.source_frame_id);
        let computed_as = match &derivation.join {
            Some(join) => format!(
                "derived from \"{source}\", {}",
                join_description(frame, document, join)
            ),
            None => format!("derived from \"{source}\""),
        };
        rows.push(LineageRow {
            frame: frame.name.clone(),
            column: "(source)".to_string(),
            type_name: String::new(),
            computed_as,
            reads: String::new(),
        });
    }

    // Bounded by both lists' lengths: the same broken-reference case noted
    // above can leave `computed.steps` shorter than the raw chain, and
    // slicing past either end would panic rather than export what it can.
    let skip = computed
        .pass_through_steps
        .min(chain.len())
        .min(computed.steps.len());
    for (offset, (raw_step, rendered_step)) in chain[skip..]
        .iter()
        .zip(computed.steps[skip..].iter())
        .enumerate()
    {
        rows.push(LineageRow {
            frame: frame.name.clone(),
            column: format!("(step {})", offset + 1),
            type_name: String::new(),
            computed_as: describe_rendered_step(frame, document, rendered_step),
            reads: step_reads(document, raw_step),
        });
    }

    rows
}
