//! Editing a control rewrites only the value argument of its ordinary
//! constructor. Resolve against the latest expression, not UI-cached source:
//! another editor may have changed bounds or neighboring Scratchwork lines.
use crate::formula::controls;
use crate::*;

impl Document {
    /// A variable's text editor and its declared input are two ways to edit
    /// the same formula, and both resolve to existing replicated operations.
    pub(crate) fn prepare_variable_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        match operation {
            Operation::AddVariable {
                name,
                formula,
                x,
                y,
            } => self.prepare_add_variable(name, formula, x, y),
            Operation::SetResultFormula { object_id, formula } => {
                self.prepare_set_result_formula(object_id, formula)
            }
            Operation::SetParameterValue { object_id, value } => {
                self.prepare_set_parameter_value(object_id, value)
            }
            _ => Err(CoreError::InvalidOperation(
                "Not a variable operation".into(),
            )),
        }
    }

    pub(crate) fn parameter_inputs(&self) -> Vec<ParameterInput> {
        let mut inputs = Vec::new();
        let mut push = |id: &str, name: &str, expression: &Expr| {
            if self.frozen_values.contains_key(id) {
                return;
            }
            if let Ok(Some(control)) = controls::read(expression) {
                inputs.push(ParameterInput {
                    id: id.into(),
                    name: name.into(),
                    control,
                });
            }
        };
        for object in &self.objects {
            match object {
                DataObject::Result(result) => {
                    push(&result.id, &result.name, &result.formula.expression)
                }
                DataObject::Block(block) => {
                    for line in &block.lines {
                        if line.named
                            && let Some(expression) = line.expression()
                        {
                            push(&line.id, &line.name, expression);
                        }
                    }
                }
                _ => {}
            }
        }
        inputs
    }

    pub(crate) fn prepare_set_parameter_value(
        &self,
        object_id: Id,
        value: ScalarValue,
    ) -> Result<ReplicatedOperation, CoreError> {
        if self.frozen_values.contains_key(&object_id) {
            return Err(CoreError::InvalidOperation(
                "Unfreeze this variable before changing its control".into(),
            ));
        }
        if let Ok(DataObject::Result(result)) = self.object(&object_id) {
            let expression =
                changed(&result.formula.expression, &value).map_err(CoreError::Formula)?;
            return Ok(ReplicatedOperation::SetResultFormula {
                object_id,
                formula: Formula { expression },
            });
        }
        let (block, index) = self
            .block_line(&object_id)
            .ok_or(CoreError::ObjectNotFound)?;
        let line = &block.lines[index];
        if !line.named {
            return Err(CoreError::InvalidOperation(
                "Name this variable before using its control".into(),
            ));
        }
        let expression = changed(
            line.expression()
                .ok_or_else(|| CoreError::Formula("This variable has no valid control".into()))?,
            &value,
        )
        .map_err(CoreError::Formula)?;
        let mut lines = block.lines.clone();
        lines[index].source =
            expression.render_in_scope(&FrameObject::default(), self, Some(block), 0);
        lines[index].formula = Some(Formula { expression });
        lines[index].error = None;
        Ok(ReplicatedOperation::SetBlockLines {
            block_id: block.id.clone(),
            lines,
        })
    }
}

fn changed(original: &Expr, value: &ScalarValue) -> Result<Expr, String> {
    let control = controls::read(original)?.ok_or("This variable is not a control constructor")?;
    controls::check_value(&control, value)?;
    let replacement = controls::expression(value)?;
    let mut expression = original.clone();
    if let Expr::PolarsCall {
        name,
        arguments,
        keyword_arguments,
    } = &mut expression
    {
        let position = match name.as_str() {
            "slider" => 3,
            "dropdown" => 1,
            _ => 0,
        };
        if arguments.len() > position {
            arguments[position] = replacement;
        } else if let Some((_, argument)) =
            keyword_arguments.iter_mut().find(|(key, _)| key == "value")
        {
            *argument = replacement;
        } else {
            keyword_arguments.push(("value".into(), replacement));
        }
    }
    Ok(expression)
}
