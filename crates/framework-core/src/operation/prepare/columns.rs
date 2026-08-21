//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to column IDs, and
//! every precondition checked before anything is applied.
//!
//! Column lifecycle: adding, deleting, retyping, formatting, and the
//! formulas and summaries attached to a column.

use crate::*;

impl Document {
    pub(crate) fn prepare_add_column(
        &self,
        frame_id: Id,
        name: String,
        data_type: DataType,
        after_column_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            if after_column_id.as_ref().is_some_and(|column_id| {
                !frame.columns.iter().any(|column| column.id == *column_id)
            }) {
                return Err(CoreError::ColumnNotFound);
            }
            ReplicatedOperation::AddColumn {
                frame_id,
                column: Column {
                    id: column_id(&name),
                    name,
                    source_name: None,
                    data_type,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                },
                after_column_id,
            }
        })
    }

    pub(crate) fn prepare_delete_column(
        &self,
        frame_id: Id,
        column_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            ReplicatedOperation::DeleteColumn {
                frame_id,
                column_id,
            }
        })
    }

    pub(crate) fn prepare_rename_column(
        &self,
        frame_id: Id,
        column_id: Id,
        name: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            ReplicatedOperation::RenameColumn {
                frame_id,
                column_id,
                name,
            }
        })
    }

    pub(crate) fn prepare_set_column_type(
        &self,
        frame_id: Id,
        column_id: Id,
        data_type: DataType,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            ReplicatedOperation::SetColumnType {
                frame_id,
                column_id,
                data_type,
            }
        })
    }

    pub(crate) fn prepare_set_column_categories(
        &self,
        frame_id: Id,
        column_id: Id,
        categories: Vec<String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .ok_or(CoreError::ColumnNotFound)?;
            let categories = normalized_categories(categories)?;
            validate_category_values(column, &frame.rows, &categories)?;
            ReplicatedOperation::SetColumnCategories {
                frame_id,
                column_id,
                categories,
            }
        })
    }

    pub(crate) fn prepare_set_column_format(
        &self,
        frame_id: Id,
        column_id: Id,
        format: Option<ColumnFormat>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            if !frame.columns.iter().any(|column| column.id == column_id) {
                return Err(CoreError::ColumnNotFound);
            }
            ReplicatedOperation::SetColumnFormat {
                frame_id,
                column_id,
                format: format.map(normalized_column_format),
            }
        })
    }

    pub(crate) fn prepare_add_computed_column(
        &self,
        frame_id: Id,
        name: String,
        formula: String,
        after_column_id: Option<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let expression = self.prepare_formula_for_frame(&frame_id, &formula)?;
            let frame = self.frame(&frame_id)?;
            if after_column_id.as_ref().is_some_and(|column_id| {
                !frame.columns.iter().any(|column| column.id == *column_id)
            }) {
                return Err(CoreError::ColumnNotFound);
            }
            let data_type = frame
                .infer_polars_expression_type(self, &expression)
                .map_err(CoreError::Formula)?;
            ReplicatedOperation::AddColumn {
                frame_id,
                after_column_id: after_column_id
                    .or_else(|| frame.columns.last().map(|column| column.id.clone())),
                column: Column {
                    id: column_id(&name),
                    name,
                    source_name: None,
                    data_type,
                    categories: Vec::new(),
                    format: None,
                    formula: Some(Formula { expression }),
                },
            }
        })
    }

    pub(crate) fn prepare_set_column_formula(
        &self,
        frame_id: Id,
        column_id: Id,
        formula: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let expression = self.prepare_formula_for_frame(&frame_id, &formula)?;
            let data_type = self
                .frame(&frame_id)?
                .infer_polars_expression_type(self, &expression)
                .map_err(CoreError::Formula)?;
            ReplicatedOperation::SetColumnFormula {
                frame_id,
                column_id,
                formula: Formula { expression },
                data_type,
            }
        })
    }

    pub(crate) fn prepare_add_summary(
        &self,
        frame_id: Id,
        column_id: Id,
        operation: SummaryOperation,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            if !frame.columns.iter().any(|column| column.id == column_id) {
                return Err(CoreError::ColumnNotFound);
            }
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .ok_or(CoreError::ColumnNotFound)?;
            if !operation.supports(column.data_type) {
                return Err(CoreError::InvalidOperation(format!(
                    "{} does not apply to {} columns",
                    operation.label(),
                    data_type_name(column.data_type)
                )));
            }
            ReplicatedOperation::AddSummary {
                frame_id,
                summary: Summary {
                    id: id(),
                    column_id,
                    operation,
                    label: operation.label().into(),
                },
            }
        })
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
    fn shift_formulas_require_a_declared_sort() {
        let mut store = demo_store();
        let customers = frame_named(&store.document, "Customers");
        let frame_id = customers.id.clone();
        let amount = customers.columns[0].id.clone();
        let amount_name = customers.columns[0].name.clone();

        let error = store
            .apply(Operation::AddComputedColumn {
                frame_id: frame_id.clone(),
                name: "Previous".into(),
                formula: format!("`{amount_name}`.shift(1)"),
                after_column_id: None,
            })
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("declared row ordering"));
        assert!(message.contains("Sort the frame or bind a sort column"));

        store
            .apply(Operation::SetFramePipeline {
                frame_id,
                steps: vec![
                    FrameStepInput::Sort {
                        keys: vec![SortInput {
                            column_id: amount,
                            descending: false,
                        }],
                    },
                    FrameStepInput::WithColumns {
                        columns: vec![ExistingFormulaInput {
                            output_column_id: id(),
                            name: "Previous".into(),
                            formula: format!("`{amount_name}`.shift(1)"),
                        }],
                    },
                ],
            })
            .unwrap();
    }

    #[test]
    fn shift_guard_also_applies_when_replacing_a_column_formula() {
        let mut store = demo_store();
        let customers = frame_named(&store.document, "Customers");
        let frame_id = customers.id.clone();
        let column_id = customers.columns[0].id.clone();
        let column_name = customers.columns[0].name.clone();

        let error = store
            .apply(Operation::SetColumnFormula {
                frame_id,
                column_id,
                formula: format!("`{column_name}`.shift(-1)"),
            })
            .unwrap_err();
        assert!(error.to_string().contains("bind a sort column"));
    }

    #[test]
    pub(crate) fn categorical_columns_persist_allowed_values_and_reject_invalid_cells() {
        let mut store = Store::new(Document::demo());
        let view = store.view();
        let customers = view
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "Customers" => Some(frame),
                _ => None,
            })
            .unwrap();
        let frame_id = customers.id.clone();
        let column_id = customers
            .columns
            .iter()
            .find(|column| column.name == "Segment")
            .unwrap()
            .id
            .clone();
        let row_id = customers.rows[0].id.clone();

        store
            .apply(Operation::SetColumnType {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                data_type: DataType::Categorical,
            })
            .unwrap();
        let after_type_change = store.view();
        let column = after_type_change
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .unwrap()
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .unwrap();
        assert_eq!(
            column.categories,
            vec!["Enterprise", "Growth", "Small business"]
        );

        let error = store
            .apply(Operation::SetCell {
                frame_id: frame_id.clone(),
                row_id: row_id.clone(),
                column_id: column_id.clone(),
                raw: "Unknown".into(),
            })
            .unwrap_err();
        assert!(error.to_string().contains("not an allowed value"));

        store
            .apply(Operation::SetColumnCategories {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                categories: vec![
                    "Growth".into(),
                    "Enterprise".into(),
                    "Small business".into(),
                    "Public sector".into(),
                ],
            })
            .unwrap();
        store
            .apply(Operation::SetCell {
                frame_id,
                row_id,
                column_id,
                raw: "Public sector".into(),
            })
            .unwrap();
    }

    #[test]
    pub(crate) fn set_column_format_replicates_and_round_trips_through_serde() {
        let document = Document::demo();
        let frame = frame_named(&document, "Orders");
        let frame_id = frame.id.clone();
        let column_id = frame
            .columns
            .iter()
            .find(|column| column.name == "Unit price")
            .unwrap()
            .id
            .clone();
        let mut first = Store::new(document.clone());
        let mut second = Store::new(document);

        let prepared = first
            .prepare_operation(Operation::SetColumnFormat {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                format: Some(ColumnFormat {
                    style: ColumnFormatStyle::Currency,
                    decimals: Some(2),
                    scale: ColumnFormatScale::Units,
                    negative_parens: Some(true),
                    zero_dash: Some(false),
                    currency_code: Some("EUR".into()),
                }),
            })
            .unwrap();
        let serialized = serde_json::to_string(&prepared).unwrap();
        assert!(serialized.contains("\"currencyCode\":\"EUR\""));
        assert!(serialized.contains("\"negativeParens\":true"));
        let replayed: ReplicatedOperation = serde_json::from_str(&serialized).unwrap();

        first.apply_replicated(prepared).unwrap();
        second.apply_replicated(replayed).unwrap();
        assert_eq!(first.document, second.document);

        // The frontend sends camelCase operation JSON; sparse format fields
        // fall back to their serde defaults.
        let operation: Operation = serde_json::from_value(serde_json::json!({
            "type": "setColumnFormat",
            "frameId": frame_id,
            "columnId": column_id,
            "format": { "style": "accounting" }
        }))
        .unwrap();
        assert!(matches!(
            operation,
            Operation::SetColumnFormat {
                format: Some(ColumnFormat {
                    style: ColumnFormatStyle::Accounting,
                    decimals: None,
                    scale: ColumnFormatScale::Units,
                    negative_parens: None,
                    zero_dash: None,
                    currency_code: None,
                }),
                ..
            }
        ));
    }
}
