use super::FormulaFunctionDefinition;

pub(crate) const FUNCTIONS: &[FormulaFunctionDefinition] = &[
    formula_function!("root.slider", "slider", [], "Controls", "slider(start, stop, step, value=start)", "A numeric variable with a slider UI. Literal bounds and positive step; selected value defaults to start. Exact values within the range are allowed between slider steps. A named variable or Scratchwork line displays the control.", 3, 4),
    formula_function!("root.dropdown", "dropdown", [], "Controls", "dropdown(options, value=first)", "A variable with a dropdown UI. Options are a nonempty literal list of distinct values of one type. Selection defaults to the first option. Reference the variable in formulas or filters.", 1, 2),
    formula_function!("root.date_input", "date_input", [], "Controls", "date_input(value)", "A date variable with a date-picker UI. Supply a literal date, such as date(2026,12,31). Reference the variable in a formula or filter; changing the control updates the variable's formula.", 1, 1),
];
