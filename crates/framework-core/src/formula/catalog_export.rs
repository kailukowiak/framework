use super::*;

pub fn formula_function_catalog() -> Vec<FormulaFunction> {
    let hand_written = POLARS_FORMULA_FUNCTIONS
        .iter()
        .chain(financial::FUNCTIONS.iter())
        .chain(controls::FUNCTIONS.iter())
        .map(|function| FormulaFunction {
            id: function.id.into(),
            name: function.name.into(),
            aliases: function
                .aliases
                .iter()
                .map(|alias| (*alias).into())
                .collect(),
            category: function.category.into(),
            signature: function.signature.into(),
            description: function.description.into(),
            minimum_arguments: function.minimum_arguments,
            maximum_arguments: function.maximum_arguments,
            return_type: formula_function_return_type(function.id).into(),
            null_behavior: formula_function_null_behavior(function.id).into(),
            arguments: formula_function_arguments(function.id, function.signature),
        });
    let generated = crate::formula::generated_bindings::GENERATED_FORMULA_FUNCTIONS
        .iter()
        .map(|function| FormulaFunction {
            id: function.id.into(),
            name: function.name.into(),
            aliases: Vec::new(),
            category: function.category.into(),
            signature: function.signature.into(),
            description: function.description.into(),
            minimum_arguments: function.minimum_arguments,
            maximum_arguments: function.maximum_arguments,
            return_type: function.return_type.into(),
            null_behavior: "native Polars behavior".into(),
            arguments: formula_function_arguments(function.id, function.signature),
        });
    financial_namespace::extend(hand_written.chain(generated).collect())
}
