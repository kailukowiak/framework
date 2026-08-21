use crate::*;
use std::collections::{HashMap, HashSet};

pub(crate) fn validate_frame_style_target(
    frame: &FrameObject,
    target: &FrameStyleTarget,
) -> Result<(), CoreError> {
    let column_id = match target {
        FrameStyleTarget::Column { column_id } | FrameStyleTarget::Cell { column_id, .. } => {
            Some(column_id)
        }
        _ => None,
    };
    if column_id
        .is_some_and(|column_id| !frame.columns.iter().any(|column| column.id == *column_id))
    {
        return Err(CoreError::ColumnNotFound);
    }

    // Derived-frame rows are materialized only for DocumentView, so their
    // deterministic row IDs are accepted here even though canonical rows are empty.
    let row_id = match target {
        FrameStyleTarget::Row { row_id } | FrameStyleTarget::Cell { row_id, .. } => Some(row_id),
        _ => None,
    };
    if !frame.rows.is_empty()
        && row_id.is_some_and(|row_id| !frame.rows.iter().any(|row| row.id == *row_id))
    {
        return Err(CoreError::RowNotFound);
    }
    Ok(())
}

impl Document {
    /// Whether `object_id` may become a tab of `view_id`.
    ///
    /// A tab is another rendering of data the card already shows, so it is
    /// either the card's own object or something reading from a tab already
    /// on it — a pass-through frame branched from one, or a plot drawn from
    /// one. That single rule is what keeps a card about one thing: every tab
    /// on it traces back to the same data, so switching tabs changes the
    /// presentation and never the subject.
    ///
    /// A join has two parents and so no unambiguous home: it is wrangled
    /// from its own card instead.
    pub(crate) fn validate_tab_target(
        &self,
        view_id: &str,
        object_id: &str,
    ) -> Result<(), CoreError> {
        let view = self.view(view_id)?;
        if view.tabs().iter().any(|tab| tab == object_id) {
            return Ok(());
        }
        let source_id = match self
            .objects
            .iter()
            .find(|object| object.id() == object_id)
            .ok_or(CoreError::ObjectNotFound)?
        {
            DataObject::Plot(plot) => &plot.source_frame_id,
            DataObject::Frame(frame) => {
                let Some(derivation) = &frame.derivation else {
                    return Err(CoreError::InvalidOperation(
                        "A tab must read from something the card already shows".into(),
                    ));
                };
                if derivation
                    .steps()
                    .iter()
                    .any(|step| step.lookup_frame_id().is_some())
                {
                    return Err(CoreError::InvalidOperation(
                        "A frame built from two frames has no one home, so it cannot be a tab"
                            .into(),
                    ));
                }
                &derivation.source_frame_id
            }
            _ => {
                return Err(CoreError::InvalidOperation(
                    "Only frames and plots can be tabs of a card".into(),
                ));
            }
        };
        if !view.tabs().contains(source_id) {
            return Err(CoreError::InvalidOperation(
                "A tab must read from something the card already shows".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn validate_sort_keys(
    frame: &FrameObject,
    keys: &[DerivedSort],
) -> Result<(), CoreError> {
    if keys.iter().all(|key| {
        frame
            .columns
            .iter()
            .any(|column| column.id == key.column_id)
    }) {
        return Ok(());
    }
    Err(CoreError::ColumnNotFound)
}

/// A display layer only ever holds a filter and a sort, in that order.
///
/// Anything else — a projection, a summarize, a join — would change the
/// frame's schema, and a frame whose columns depend on how it is being
/// looked at is not a frame any more.
pub(crate) fn validate_promotable_display(frame: &FrameObject) -> Result<(), CoreError> {
    if frame
        .display
        .steps
        .iter()
        .any(|step| !matches!(step, FrameStep::Filter { .. } | FrameStep::Sort { .. }))
    {
        return Err(CoreError::InvalidOperation(
            "A display layer can only filter and sort".into(),
        ));
    }
    // A join is held flat rather than as a chain, and it carries the join's
    // own projection instead of declared columns, so materializing it as a
    // chain would introduce a closing select it cannot satisfy.
    if frame
        .derivation
        .as_ref()
        .is_some_and(|derivation| derivation.steps.is_empty() && derivation.join.is_some())
    {
        return Err(CoreError::InvalidOperation(
            "Rebuild this join as a pipeline before promoting its display layer".into(),
        ));
    }
    Ok(())
}

fn valid_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// A rule's reading of its own hidden column, checked against what that
/// column actually returns.
///
/// The type is the whole check: a ramp over text has no ends, and a case
/// list over a number is a list of values it will never equal. Refusing here
/// is what lets the interface offer exactly one editor per rule and never
/// ask which kind someone meant.
pub(crate) fn validate_frame_style_output(
    output: &FrameStyleOutput,
    data_type: DataType,
) -> Result<(), CoreError> {
    match output {
        FrameStyleOutput::Condition { style } => {
            if data_type != DataType::Boolean {
                return Err(CoreError::InvalidOperation(
                    "This rule styles the rows a formula answers true for, so its formula must \
                     produce true or false"
                        .into(),
                ));
            }
            validate_frame_cell_style(style)?;
            if style.is_empty() {
                return Err(CoreError::InvalidOperation(
                    "A conditional-formatting rule must change at least one style property".into(),
                ));
            }
        }
        FrameStyleOutput::Category { cases, other } => {
            if !matches!(data_type, DataType::String | DataType::Categorical) {
                return Err(CoreError::InvalidOperation(
                    "This rule styles each value a formula returns, so its formula must produce \
                     text"
                        .into(),
                ));
            }
            if cases.is_empty() && other.is_none() {
                return Err(CoreError::InvalidOperation(
                    "A conditional-formatting rule must style at least one value".into(),
                ));
            }
            let mut seen = std::collections::HashSet::new();
            for case in cases {
                if !seen.insert(case.value.as_str()) {
                    return Err(CoreError::InvalidOperation(format!(
                        "'{}' is styled twice by the same rule",
                        case.value
                    )));
                }
                validate_frame_cell_style(&case.style)?;
            }
            if let Some(other) = other {
                validate_frame_cell_style(other)?;
            }
        }
        FrameStyleOutput::Scale { scale } => {
            // Currency and percentage are numbers wearing a format, and a
            // ramp over a money column is the first thing anybody asks for.
            if !matches!(
                data_type,
                DataType::Integer | DataType::Number | DataType::Currency | DataType::Percentage
            ) {
                return Err(CoreError::InvalidOperation(
                    "This rule reads a formula as a position between two colors, so its formula \
                     must produce a number"
                        .into(),
                ));
            }
            if scale.text.is_none() && scale.fill.is_none() {
                return Err(CoreError::InvalidOperation(
                    "A color scale must paint text, fill, or both".into(),
                ));
            }
            if [scale.text.as_ref(), scale.fill.as_ref()]
                .into_iter()
                .flatten()
                .flat_map(|colors| [Some(&colors.low), Some(&colors.high), colors.mid.as_ref()])
                .flatten()
                .any(|color| !valid_color(color))
            {
                return Err(CoreError::InvalidOperation(
                    "Style colors must use #RRGGBB notation".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_frame_cell_style(style: &FrameCellStyle) -> Result<(), CoreError> {
    if style
        .text_color
        .iter()
        .chain(style.fill_color.iter())
        .any(|color| !valid_color(color))
    {
        return Err(CoreError::InvalidOperation(
            "Style colors must use #RRGGBB notation".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_category_raw(column: &Column, raw: &str) -> Result<(), CoreError> {
    if column.data_type == DataType::Categorical
        && !raw.trim().is_empty()
        && !column.categories.iter().any(|category| category == raw)
    {
        return Err(CoreError::InvalidOperation(format!(
            "'{}' is not an allowed value for categorical column '{}'",
            raw, column.name
        )));
    }
    Ok(())
}

pub(crate) fn validate_category_values(
    column: &Column,
    rows: &[Row],
    categories: &[String],
) -> Result<(), CoreError> {
    if let Some(raw) = rows
        .iter()
        .filter_map(|row| row.cells.get(&column.id).map(|cell| cell.raw.as_str()))
        .find(|raw| !raw.trim().is_empty() && !categories.iter().any(|category| category == raw))
    {
        return Err(CoreError::InvalidOperation(format!(
            "cannot remove category '{raw}' while the column still contains it"
        )));
    }
    Ok(())
}

impl Document {
    pub(crate) fn validate_unique_keys(&self) -> Result<(), CoreError> {
        for frame in self.objects.iter().filter_map(|object| match object {
            DataObject::Frame(frame) if !frame.unique_keys.is_empty() => Some(frame),
            _ => None,
        }) {
            let data_frame = self
                .materialize_frame_frame(&frame.id, Layer::Data, &mut HashSet::new())
                .map_err(CoreError::InvalidOperation)?;
            for key in &frame.unique_keys {
                if key.column_ids.is_empty()
                    || key.column_ids.iter().any(|column_id| {
                        !frame.columns.iter().any(|column| column.id == *column_id)
                    })
                {
                    return Err(CoreError::InvalidOperation(
                        "A unique key references a missing column".into(),
                    ));
                }
                let mut values = HashSet::with_capacity(data_frame.height());
                for row_index in 0..data_frame.height() {
                    let row_key = key
                        .column_ids
                        .iter()
                        .map(|column_id| {
                            data_frame
                                .column(column_id)
                                .map_err(|error| CoreError::InvalidOperation(error.to_string()))?
                                .get(row_index)
                                .map(|value| format!("{value:?}"))
                                .map_err(|error| CoreError::InvalidOperation(error.to_string()))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if !values.insert(row_key) {
                        let names = key
                            .column_ids
                            .iter()
                            .filter_map(|column_id| {
                                frame
                                    .columns
                                    .iter()
                                    .find(|column| column.id == *column_id)
                                    .map(|column| column.name.as_str())
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(CoreError::InvalidOperation(format!(
                            "{} cannot use {} as a unique key because it contains duplicates",
                            frame.name, names
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_join_derivations(&self) -> Result<(), CoreError> {
        for frame in self.objects.iter().filter_map(|object| match object {
            DataObject::Frame(frame)
                if frame
                    .derivation
                    .as_ref()
                    .is_some_and(|derivation| derivation.join.is_some()) =>
            {
                Some(frame)
            }
            _ => None,
        }) {
            let derivation = frame.derivation.as_ref().expect("filtered derivation");
            let join = derivation.join.as_ref().expect("filtered join");
            let primary = self.frame(&derivation.source_frame_id)?;
            let lookup = self.frame(&join.lookup_frame_id)?;
            // Left and inner joins execute with Polars many-to-one validation,
            // so the lookup side must stay an enforced unique key or duplicate
            // matches could silently multiply primary rows. Anti and semi
            // joins only test key membership: duplicate lookup keys cannot add
            // rows to (or expand) their result, so the unique-key requirement
            // is deliberately relaxed for those two policies. Null keys still
            // never match under any policy.
            if join.primary_key_column_ids.is_empty()
                || join.primary_key_column_ids.len() != join.lookup_key_column_ids.len()
                || (join.join_type.keeps_lookup_columns()
                    && !lookup
                        .unique_keys
                        .iter()
                        .any(|key| key.column_ids == join.lookup_key_column_ids))
            {
                return Err(CoreError::InvalidOperation(
                    "A join requires an enforced unique key on its lookup frame".into(),
                ));
            }
            // Anti and semi results contain only primary-side columns, so a
            // join edited into one of these policies must first drop every
            // lookup-side output mapping.
            if !join.join_type.keeps_lookup_columns() {
                let lookup_outputs = join
                    .outputs
                    .iter()
                    .filter(|output| output.source_frame_id == join.lookup_frame_id)
                    .map(|output| {
                        frame
                            .columns
                            .iter()
                            .find(|column| column.id == output.output_column_id)
                            .map(|column| column.name.as_str())
                            .unwrap_or(output.output_column_id.as_str())
                    })
                    .collect::<Vec<_>>();
                if !lookup_outputs.is_empty() {
                    return Err(CoreError::InvalidOperation(format!(
                        "A {} join keeps only {} columns; remove {}",
                        join.join_type.label(),
                        primary.name,
                        lookup_outputs.join(", ")
                    )));
                }
            }
            for (primary_id, lookup_id) in join
                .primary_key_column_ids
                .iter()
                .zip(&join.lookup_key_column_ids)
            {
                let primary_column = primary
                    .columns
                    .iter()
                    .find(|column| column.id == *primary_id)
                    .ok_or(CoreError::ColumnNotFound)?;
                let lookup_column = lookup
                    .columns
                    .iter()
                    .find(|column| column.id == *lookup_id)
                    .ok_or(CoreError::ColumnNotFound)?;
                if !join_types_compatible(primary_column.data_type, lookup_column.data_type) {
                    return Err(CoreError::InvalidOperation(
                        "Join columns must have compatible types".into(),
                    ));
                }
            }
            self.materialize_frame_frame(&frame.id, Layer::Data, &mut HashSet::new())
                .map_err(CoreError::InvalidOperation)?;
        }
        Ok(())
    }

    /// What a generator's rule may reach: values, results, block lines, and
    /// literals — never a frame's columns. A rule that read a column would
    /// make the generated frame downstream of a frame no lineage walk knows
    /// about, since a generator is deliberately a root; and reading a live
    /// column through the scalar path also has no cycle guard. The scalar
    /// parser already refuses bare column names, so the one spelling left to
    /// refuse is the qualified `` `Frame`.`Column` `` reference.
    pub(crate) fn validate_generator_rule(&self, expression: &Expr) -> Result<(), CoreError> {
        let mut foreign = false;
        expression.walk(&mut |expression| {
            if matches!(expression, Expr::ForeignColumn { .. }) {
                foreign = true;
            }
        });
        if foreign {
            return Err(CoreError::Formula(
                "A generator's rule reads values, not frame columns. \
                 Put the column expression on a scratchpad line and name the line here."
                    .into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_cell_override(
        &self,
        frame_id: &str,
        column_id: &str,
        formula: Option<&Formula>,
    ) -> Result<(), CoreError> {
        let frame = self.frame(frame_id)?;
        let target_type = frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.data_type)
            .ok_or(CoreError::ColumnNotFound)?;
        let Some(formula) = formula else {
            return Ok(());
        };
        self.ensure_expression_not_recursive(frame_id, column_id, &formula.expression)?;
        let output_type = frame
            .infer_polars_expression_type(self, &formula.expression)
            .map_err(CoreError::Formula)?;
        if !formula.expression.is_explicit_null()
            && !formula_types_compatible(target_type, output_type)
        {
            return Err(CoreError::Formula(
                "A cell override must preserve its column's data type".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn ensure_acyclic(&self, frame_id: &str) -> Result<(), CoreError> {
        let frame = self.frame(frame_id)?;
        let graph: HashMap<&str, Vec<&str>> = frame
            .columns
            .iter()
            .filter_map(|column| {
                column.formula.as_ref().map(|formula| {
                    let mut dependencies = Vec::new();
                    formula.expression.column_dependencies(&mut dependencies);
                    (column.id.as_str(), dependencies)
                })
            })
            .collect();

        fn visit<'a>(
            node: &'a str,
            graph: &HashMap<&'a str, Vec<&'a str>>,
            visiting: &mut HashSet<&'a str>,
            visited: &mut HashSet<&'a str>,
        ) -> bool {
            if visiting.contains(node) {
                return true;
            }
            if visited.contains(node) {
                return false;
            }
            visiting.insert(node);
            if graph.get(node).is_some_and(|edges| {
                edges
                    .iter()
                    .any(|edge| visit(edge, graph, visiting, visited))
            }) {
                return true;
            }
            visiting.remove(node);
            visited.insert(node);
            false
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        if graph
            .keys()
            .any(|node| visit(node, &graph, &mut visiting, &mut visited))
        {
            return Err(CoreError::CircularDependency);
        }
        Ok(())
    }

    pub(crate) fn ensure_expression_not_recursive(
        &self,
        frame_id: &str,
        target_column_id: &str,
        expression: &Expr,
    ) -> Result<(), CoreError> {
        let frame = self.frame(frame_id)?;
        let graph: HashMap<&str, Vec<&str>> = frame
            .columns
            .iter()
            .filter_map(|column| {
                column.formula.as_ref().map(|formula| {
                    let mut dependencies = Vec::new();
                    formula.expression.column_dependencies(&mut dependencies);
                    (column.id.as_str(), dependencies)
                })
            })
            .collect();
        let mut dependencies = Vec::new();
        expression.column_dependencies(&mut dependencies);

        fn reaches<'a>(
            current: &'a str,
            target: &str,
            graph: &HashMap<&'a str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
        ) -> bool {
            if current == target {
                return true;
            }
            if !visited.insert(current) {
                return false;
            }
            graph.get(current).is_some_and(|edges| {
                edges
                    .iter()
                    .any(|edge| reaches(edge, target, graph, visited))
            })
        }

        if dependencies
            .into_iter()
            .any(|dependency| reaches(dependency, target_column_id, &graph, &mut HashSet::new()))
        {
            return Err(CoreError::CircularDependency);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn anti_and_semi_joins_reject_lookup_side_output_columns() {
        let mut store = Store::new(Document {
            id: id(),
            name: "Join outputs".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        });
        store
            .apply(Operation::AddFrame {
                name: "Orders".into(),
                grid: vec![
                    vec!["Order ID".into(), "Customer ID".into()],
                    vec!["O-1".into(), "C-1".into()],
                    vec!["O-2".into(), "C-9".into()],
                ],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        store
            .apply(Operation::AddFrame {
                name: "Customers".into(),
                grid: vec![
                    vec!["Customer ID".into(), "Customer name".into()],
                    vec!["C-1".into(), "Ada".into()],
                ],
                x: 0.0,
                y: 400.0,
            })
            .unwrap();
        let (orders_id, order_id, order_customer_id) = {
            let frame = frame_named(&store.document, "Orders");
            (
                frame.id.clone(),
                frame.columns[0].id.clone(),
                frame.columns[1].id.clone(),
            )
        };
        let (customers_id, customer_id, customer_name) = {
            let frame = frame_named(&store.document, "Customers");
            (
                frame.id.clone(),
                frame.columns[0].id.clone(),
                frame.columns[1].id.clone(),
            )
        };

        for join_type in [FrameJoinType::Anti, FrameJoinType::Semi] {
            assert!(matches!(
                store.apply(Operation::AddJoinFrame {
                    primary_frame_id: orders_id.clone(),
                    lookup_frame_id: customers_id.clone(),
                    primary_key_column_ids: vec![order_customer_id.clone()],
                    lookup_key_column_ids: vec![customer_id.clone()],
                    join_type,
                    columns: vec![
                        JoinColumnInput {
                            source_frame_id: orders_id.clone(),
                            source_column_id: order_id.clone(),
                            name: "Order ID".into(),
                        },
                        JoinColumnInput {
                            source_frame_id: customers_id.clone(),
                            source_column_id: customer_name.clone(),
                            name: "Customer name".into(),
                        },
                    ],
                    name: "Invalid membership join".into(),
                    x: 600.0,
                    y: 0.0,
                }),
                Err(CoreError::InvalidOperation(message))
                    if message.contains(join_type.label()) && message.contains("Customer name")
            ));
        }

        // A left join carrying lookup outputs cannot be edited into an anti
        // join while those outputs remain mapped.
        store
            .apply(Operation::SetUniqueKey {
                frame_id: customers_id.clone(),
                column_ids: vec![customer_id.clone()],
                enabled: true,
            })
            .unwrap();
        store
            .apply(Operation::AddJoinFrame {
                primary_frame_id: orders_id.clone(),
                lookup_frame_id: customers_id.clone(),
                primary_key_column_ids: vec![order_customer_id],
                lookup_key_column_ids: vec![customer_id],
                join_type: FrameJoinType::Left,
                columns: vec![
                    JoinColumnInput {
                        source_frame_id: orders_id,
                        source_column_id: order_id,
                        name: "Order ID".into(),
                    },
                    JoinColumnInput {
                        source_frame_id: customers_id,
                        source_column_id: customer_name,
                        name: "Customer name".into(),
                    },
                ],
                name: "Orders with customers".into(),
                x: 700.0,
                y: 0.0,
            })
            .unwrap();
        store
            .document
            .objects
            .iter_mut()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "Orders with customers" => Some(frame),
                _ => None,
            })
            .unwrap()
            .derivation
            .as_mut()
            .unwrap()
            .join
            .as_mut()
            .unwrap()
            .join_type = FrameJoinType::Anti;
        let message = store
            .document
            .validate_join_derivations()
            .unwrap_err()
            .to_string();
        assert!(message.contains("anti") && message.contains("Customer name"));
    }
}
