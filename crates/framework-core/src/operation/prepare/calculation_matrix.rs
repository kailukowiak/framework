use crate::*;

impl Document {
    pub(crate) fn prepare_add_calculation_matrix(
        &self,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let object_id = id();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::CalculationMatrix(CalculationMatrixObject {
                id: object_id.clone(),
                name,
                rows: Vec::new(),
                columns: Vec::new(),
                body: CalculationMatrixBody {
                    source: String::new(),
                    formula: None,
                    error: None,
                },
            }),
            view: CanvasView {
                id: id(),
                object_id,
                x,
                y,
                width: 440.0,
                height: 240.0,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
            container_id: None,
        })
    }

    pub(crate) fn prepare_set_calculation_matrix(
        &self,
        object_id: Id,
        rows: Vec<CalculationMatrixFormulaInput>,
        columns: Vec<CalculationMatrixFormulaInput>,
        body_source: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let DataObject::CalculationMatrix(current) = self.object(&object_id)? else {
            return Err(CoreError::InvalidOperation(
                "That is not a calculation matrix".into(),
            ));
        };
        let mut rows = self.prepare_matrix_axis(rows, &current.rows)?;
        let mut columns = self.prepare_matrix_axis(columns, &current.columns)?;
        let mut names = std::collections::HashSet::new();
        let mut naming_error = None;
        for item in rows.iter_mut().chain(&mut columns) {
            let invalid = if item.name.trim().is_empty() {
                Some("Matrix axis names cannot be blank".to_string())
            } else if !names.insert(item.name.to_lowercase()) {
                Some(format!(
                    "Matrix axis name '{}' is used more than once",
                    item.name
                ))
            } else {
                None
            };
            if let Some(error) = invalid {
                item.error = Some(error.clone());
                naming_error.get_or_insert(error);
            }
        }
        let scope = matrix_scope(&object_id, &rows, &columns);
        let parse_source = qualify_bare_axis_names(
            &body_source,
            rows.iter().chain(&columns).map(|item| item.name.as_str()),
        );
        let body = if let Some(error) = naming_error {
            CalculationMatrixBody {
                source: body_source,
                formula: None,
                error: Some(error),
            }
        } else if body_source.trim().is_empty() {
            CalculationMatrixBody {
                source: body_source,
                formula: None,
                error: None,
            }
        } else {
            match Parser::new(&parse_source, &scope, self).and_then(Parser::parse) {
                Ok(expression) => CalculationMatrixBody {
                    source: body_source,
                    formula: Some(Formula { expression }),
                    error: None,
                },
                Err(error) => CalculationMatrixBody {
                    source: body_source,
                    formula: None,
                    error: Some(error.to_string()),
                },
            }
        };
        Ok(ReplicatedOperation::SetCalculationMatrix {
            object_id,
            rows,
            columns,
            body,
        })
    }

    fn prepare_matrix_axis(
        &self,
        inputs: Vec<CalculationMatrixFormulaInput>,
        previous: &[CalculationMatrixAxisFormula],
    ) -> Result<Vec<CalculationMatrixAxisFormula>, CoreError> {
        Ok(inputs
            .into_iter()
            .map(|input| {
                let item_id = input
                    .id
                    .filter(|id| previous.iter().any(|old| old.id == *id))
                    .unwrap_or_else(id);
                match self.parse_formula_rule(&input.formula) {
                    Ok(expression) => match self.evaluate_to_series(&expression) {
                        Ok((data_type, _)) => CalculationMatrixAxisFormula {
                            id: item_id,
                            name: input.name,
                            source: input.formula,
                            formula: Some(Formula { expression }),
                            data_type,
                            error: None,
                        },
                        Err(error) => CalculationMatrixAxisFormula {
                            id: item_id,
                            name: input.name,
                            source: input.formula,
                            formula: Some(Formula { expression }),
                            data_type: DataType::String,
                            error: Some(error),
                        },
                    },
                    Err(error) => CalculationMatrixAxisFormula {
                        id: item_id,
                        name: input.name,
                        source: input.formula,
                        formula: None,
                        data_type: DataType::String,
                        error: Some(error.to_string()),
                    },
                }
            })
            .collect())
    }
}

/// Matrix fields are authored like local variables (`rate * term`), while
/// ordinary frame formulas retain the explicit backtick spelling.  Rewrite
/// only complete identifiers and never text inside strings or backticks.
fn qualify_bare_axis_names<'a>(source: &str, names: impl Iterator<Item = &'a str>) -> String {
    let names = names
        .filter(|name| name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()))
        .collect::<std::collections::HashSet<_>>();
    let chars = source.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    let mut quote = None;
    while index < chars.len() {
        let character = chars[index];
        if matches!(character, '\'' | '"' | '`') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
            output.push(character);
            index += 1;
            continue;
        }
        if quote.is_none() && (character == '_' || character.is_ascii_alphabetic()) {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let word = chars[start..index].iter().collect::<String>();
            if names.contains(word.as_str()) {
                output.push('`');
                output.push_str(&word);
                output.push('`');
            } else {
                output.push_str(&word);
            }
        } else {
            output.push(character);
            index += 1;
        }
    }
    output
}

pub(crate) fn matrix_scope(
    id: &str,
    rows: &[CalculationMatrixAxisFormula],
    columns: &[CalculationMatrixAxisFormula],
) -> FrameObject {
    FrameObject {
        id: format!("{id}:output"),
        name: "Calculation Matrix output".into(),
        columns: rows
            .iter()
            .chain(columns)
            .map(|item| Column {
                id: item.id.clone(),
                name: item.name.clone(),
                source_name: None,
                data_type: item.data_type,
                categories: Vec::new(),
                format: None,
                formula: None,
            })
            .collect(),
        ..FrameObject::default()
    }
}
