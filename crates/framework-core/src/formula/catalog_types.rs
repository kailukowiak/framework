//! Descriptive result metadata shared by formula discovery surfaces.
pub(super) fn formula_function_return_type(id: &str) -> &'static str {
    match id {
        "root.lookup" | "root.map_values" | "root.coalesce" | "root.when" | "root.sequence"
        | "root.recur" | "root.previous" | "expr.fill_null" | "expr.shift" | "expr.filter"
        | "expr.over" => "dynamic",
        "expr.is_null" | "expr.is_not_null" | "expr.is_between" | "expr.is_in"
        | "dt.is_leap_year" | "str.contains" => "boolean",
        "root.frame_len" => "integer",
        "root.date" | "root.today" | "root.now" | "dt.date" | "dt.month_start" | "dt.month_end"
        | "dt.offset_by" | "str.to_date" => "date",
        "str.to_uppercase" | "str.to_lowercase" | "root.format" | "expr.format" => "string",
        // Whatever it was asked to become — the one function whose answer is
        // named by its own argument.
        "expr.cast" => "dynamic",
        // Still a number, whichever way it is written.
        "expr.show" => "number",
        _ => "number",
    }
}

pub(super) fn formula_function_null_behavior(id: &str) -> &'static str {
    match id {
        "root.today" | "root.now" => "never null",
        "root.coalesce" => "returns first non-null",
        "root.lookup" | "root.map_values" => "matches null keys and preserves mapped nulls",
        "expr.is_null" | "expr.is_not_null" => "inspects null",
        "expr.fill_null" => "replaces nulls",
        "root.sum_horizontal"
        | "root.mean_horizontal"
        | "root.min_horizontal"
        | "root.max_horizontal" => "configurable Polars behavior",
        _ => "propagates null",
    }
}
