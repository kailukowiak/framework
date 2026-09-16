use super::FormulaFunctionDefinition;

pub(crate) const FUNCTIONS: &[FormulaFunctionDefinition] = &[
    formula_function!(
        "root.under",
        "under",
        ["what if", "scenario value"],
        "Scenarios",
        "under(scenario, expression)",
        "The expression's value as the named scenario would read it, without switching the document into that scenario. Name the scenario in backticks: under(`Upside`, ebitda). Evaluates a private copy of the document, so nothing is materialized or changed.",
        2,
        2
    ),
    formula_function!(
        "root.solve",
        "solve",
        ["goal seek", "find the input"],
        "Scenarios",
        "solve(expression == target, by=value, within=[low, high], tolerance=1e-9)",
        "Goal seek: the number the named value must hold for the two sides to meet, searched by bisection between low and high. Refuses when the bracket does not contain a crossing. On its own line the answer carries its iterations and residual, and an Apply action that sets the value as an ordinary undoable edit.",
        1,
        1
    ),
];
