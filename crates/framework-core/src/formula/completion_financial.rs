use super::*;

pub(super) fn namespace(receiver: bool) -> Suggestion {
    Suggestion {
        id: "namespace.finance".into(),
        label: if receiver { ".finance" } else { "finance" }.into(),
        insert_text: "finance.".into(),
        kind: SuggestionKind::Namespace,
        detail: "Loan payments and discounted cash flows".into(),
        score: 0,
        match_indices: Vec::new(),
    }
}

pub(super) fn complete_static(partial: &str) -> CompletionResult {
    let suggestions = crate::formula_function_catalog()
        .iter()
        .filter(|f| f.id.starts_with("namespace.finance."))
        .map(|f| {
            let mut suggestion = root_function_suggestion(f);
            suggestion.label = f.name.trim_start_matches("finance.").into();
            suggestion.insert_text = format!("{}(", suggestion.label);
            suggestion
        })
        .collect();
    CompletionResult {
        replace_start: 0,
        receiver_dtype: None,
        namespace: Some("finance".into()),
        suggestions: rank(suggestions, partial),
        note: None,
        active_function_id: None,
        active_argument: None,
    }
}
