use super::*;

/// Derive both spellings from the original signatures. Their arities and
/// parameter help are executable contracts, so the receiver must actually
/// disappear from them rather than remain as an argument a person can supply
/// twice. The original root entries remain for saved formulas and Excel habits.
pub(super) fn extend(mut catalog: Vec<FormulaFunction>) -> Vec<FormulaFunction> {
    let mut variants = Vec::new();
    for function in catalog.iter().filter(|f| f.category == "Financial") {
        let mut namespaced = function.clone();
        namespaced.id = format!("namespace.finance.{}", function.name);
        namespaced.name = format!("finance.{}", function.name);
        namespaced.signature = format!("finance.{}", function.signature);
        namespaced.aliases = vec![namespaced.name.to_uppercase()];
        variants.push(namespaced);

        let receiver = crate::formula::financial_namespace::receiver_parameter(&function.name);
        let mut method = function.clone();
        method.id = format!("finance.{}", function.name);
        method.name = format!(".finance.{}", function.name);
        let labels = signature_parameter_labels(&function.signature);
        let receiver_index = function
            .arguments
            .iter()
            .position(|arg| arg.name == receiver)
            .unwrap();
        let required = function.arguments[receiver_index].required;
        method.signature = format!(
            "{}({})",
            method.name,
            labels
                .into_iter()
                .enumerate()
                .filter_map(|(i, label)| (i != receiver_index).then_some(label))
                .collect::<Vec<_>>()
                .join(", ")
        );
        method.arguments.remove(receiver_index);
        method.minimum_arguments -= usize::from(required);
        method.maximum_arguments -= 1;
        method.description = format!("Receiver supplies {receiver}. {}", function.description);
        method.aliases = vec![method.name.to_uppercase()];
        variants.push(method);
    }
    catalog.extend(variants);
    catalog
}
